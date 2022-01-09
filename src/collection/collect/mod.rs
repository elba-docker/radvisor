use crate::collection::collect::files::ProcFileHandles;
use crate::collection::collector::Collector;
use crate::collection::perf_table::{Column, ColumnType, TableMetadata};
use crate::util::{self, AnonymousSlice, Buffer};
use csv::{ByteRecord, Error};
use std::collections::BTreeMap;

pub mod files;
pub mod read;

lazy_static::lazy_static! {
    /// CSV header for the stats collector
    static ref HEADER: ByteRecord = ByteRecord::from(get_headers());

    /// Length of each row of the collected stats
    static ref ROW_LENGTH: usize = HEADER.len();
}

/// Creates the headers for the logfiles
fn get_headers() -> Vec<String> {
    let mut headers = (vec![
        "read",
        "pids.current",
        "pids.max",
        "cpu.usage.total",
        "cpu.usage.system",
        "cpu.usage.user",
        "cpu.usage.percpu",
        "cpu.stat.user",
        "cpu.stat.system",
        "cpu.throttling.periods",
        "cpu.throttling.throttled.count",
        "cpu.throttling.throttled.time",
        "memory.usage.current",
        "memory.usage.max",
        "memory.limit.hard",
        "memory.limit.soft",
        "memory.failcnt",
        "memory.hierarchical_limit.memory",
        "memory.hierarchical_limit.memoryswap",
        "memory.cache",
        "memory.rss.all",
        "memory.rss.huge",
        "memory.mapped",
        "memory.swap",
        "memory.paged.in",
        "memory.paged.out",
        "memory.fault.total",
        "memory.fault.major",
        "memory.anon.inactive",
        "memory.anon.active",
        "memory.file.inactive",
        "memory.file.active",
        "memory.unevictable",
        "blkio.time",
        "blkio.sectors",
    ])
    .into_iter()
    .map(String::from)
    .collect::<Vec<_>>();

    // Add in the IO 4-part headers
    append_io_headers(&mut headers, "blkio.service.bytes");
    append_io_headers(&mut headers, "blkio.service.ios");
    append_io_headers(&mut headers, "blkio.service.time");
    append_io_headers(&mut headers, "blkio.queued");
    append_io_headers(&mut headers, "blkio.wait");
    append_io_headers(&mut headers, "blkio.merged");
    append_io_headers(&mut headers, "blkio.throttle.service.bytes");
    append_io_headers(&mut headers, "blkio.throttle.service.ios");
    append_io_headers(&mut headers, "blkio.bfq.service.bytes");
    append_io_headers(&mut headers, "blkio.bfq.service.ios");

    headers
}

/// Expands a single I/O prefix to the 4 headers that will end up in the logfile
/// (read, write, sync, async)
pub fn append_io_headers(headers: &mut Vec<String>, base: &'static str) {
    headers.push(base.to_owned() + ".read");
    headers.push(base.to_owned() + ".write");
    headers.push(base.to_owned() + ".sync");
    headers.push(base.to_owned() + ".async");
}

/// Gets the perf table metadata for this collection setup
/// (currently static)
#[must_use]
pub fn get_table_metadata() -> TableMetadata {
    let mut columns: BTreeMap<String, Column> = BTreeMap::new();
    // Include metadata on the read (timestamp) column
    columns.insert(String::from("read"), Column::Scalar {
        r#type: ColumnType::Epoch19,
    });
    // Include metadata on the cpu.usage.percpu column,
    // which is a vector column that contains a space-delimited entry per CPU
    columns.insert(String::from("cpu.usage.percpu"), Column::Vector {
        r#type: ColumnType::Int,
        count:  util::remap::<_, usize>(util::num_cores()),
    });
    TableMetadata {
        delimiter: ",",
        columns,
    }
}

/// Gets an amortized byte record containing the entries for a header row in the
/// stats CSV log files
#[must_use]
pub fn get_header() -> &'static ByteRecord { &HEADER }

/// Length of the buffer for each row. Designed to be a reasonable upper limit
/// to prevent expensive re-allocation
const ROW_BUFFER_SIZE: usize = 1200;

/// Length of the buffer used to read proc files in with. Designed to be an
/// upper limit for the various virtual files that need to be read
const WORKING_BUFFER_SIZE: usize = 1024;

/// Length of the buffer used to build up stat file entries as the reader uses
/// pre-examined layouts to map lines to entries.
///
/// **Currently set to the number of entries used for `memory.stat`**
const SLICES_BUFFER_SIZE: usize = 16;

/// Working buffers used to avoid heap allocations at runtime
pub struct WorkingBuffers {
    record:      ByteRecord,
    buffer:      Buffer<WORKING_BUFFER_SIZE>,
    copy_buffer: Buffer<WORKING_BUFFER_SIZE>,
    slices:      [AnonymousSlice; SLICES_BUFFER_SIZE],
}

impl Default for WorkingBuffers {
    fn default() -> Self { Self::new() }
}

impl WorkingBuffers {
    /// Allocates the working buffers using upper limits to avoid expensive heap
    /// allocations at runtime
    #[must_use]
    pub fn new() -> Self {
        Self {
            record:      ByteRecord::with_capacity(ROW_BUFFER_SIZE, *ROW_LENGTH),
            slices:      [<AnonymousSlice>::default(); SLICES_BUFFER_SIZE],
            buffer:      Buffer::<WORKING_BUFFER_SIZE>::default(),
            copy_buffer: Buffer::<WORKING_BUFFER_SIZE>::default(),
        }
    }
}

/// Collects the current statistics for the given target, writing the CSV
/// entries to the writer. Utilizes /proc and cgroups (Linux-only)
pub fn run(collector: &mut Collector, buffers: &mut WorkingBuffers) -> Result<(), Error> {
    collect_read(buffers);
    collect_pids(buffers, &collector.file_handles);
    collect_cpu(buffers, &collector.file_handles);
    collect_memory(buffers, &collector.file_handles, &collector.memory_layout);
    collect_blkio(buffers, &collector.file_handles);
    collector.writer.write_byte_record(&buffers.record)?;
    buffers.record.clear();
    Ok(())
}

/// Collects the nanosecond unix timestamp read time
#[inline]
fn collect_read(buffers: &mut WorkingBuffers) {
    let nano_ts = util::nano_ts();
    let mut itoa_buffer = itoa::Buffer::new();
    let formatted = itoa_buffer.format(nano_ts);
    buffers.record.push_field(formatted.as_bytes());
}

/// Collects all stats for the pids subsystem
/// see <https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v1/pids.html>
#[inline]
fn collect_pids(buffers: &mut WorkingBuffers, handles: &ProcFileHandles) {
    read::entry(&handles.current_pids, buffers);
    read::entry(&handles.max_pids, buffers);
}

/// String offsets used for row headers for the cpuacct.stat file
const CPUACCT_STAT_OFFSETS: [usize; 2] = ["user".len(), "system".len()];
/// String offsets used for row headers for the cpu.stat file
const CPU_STAT_OFFSETS: [usize; 3] = [
    "nr_periods".len(),
    "nr_throttled".len(),
    "throttled_time".len(),
];

/// Collects all stats for the cpu and cpuacct subsystems
/// see <https://access.redhat.com/documentation/en-us/red_hat_enterprise_linux/6/html/resource_management_guide/sec-cpuacct>
#[inline]
fn collect_cpu(buffers: &mut WorkingBuffers, handles: &ProcFileHandles) {
    read::entry(&handles.cpuacct_usage, buffers);
    read::entry(&handles.cpuacct_usage_sys, buffers);
    read::entry(&handles.cpuacct_usage_user, buffers);
    read::entry(&handles.cpuacct_usage_percpu, buffers);
    read::stat_file(&handles.cpuacct_stat, &CPUACCT_STAT_OFFSETS, buffers);
    read::stat_file(&handles.cpu_stat, &CPU_STAT_OFFSETS, buffers);
}

/// Original entries in the memory.stat file that map to columns (in the same
/// order) in the final output
const MEMORY_STAT_ENTRIES: &[&[u8]] = &[
    b"hierarchical_memory_limit",
    b"hierarchical_memsw_limit",
    b"total_cache",
    b"total_rss",
    b"total_rss_huge",
    b"total_mapped_file",
    b"total_swap",
    b"total_pgpgin",
    b"total_pgpgout",
    b"total_pgfault",
    b"total_pgmajfault",
    b"total_inactive_anon",
    b"total_active_anon",
    b"total_inactive_file",
    b"total_active_file",
    b"total_unevictable",
];

/// Generates a stat file layout struct for `memory.stat`
#[must_use]
pub fn examine_memory(handles: &ProcFileHandles) -> read::StatFileLayout {
    read::StatFileLayout::new(&handles.memory_stat, MEMORY_STAT_ENTRIES)
}

/// Collects all stats for the memory subsystem
/// see <https://access.redhat.com/documentation/en-us/red_hat_enterprise_linux/6/html/resource_management_guide/sec-memory>
#[inline]
fn collect_memory(
    buffers: &mut WorkingBuffers,
    handles: &ProcFileHandles,
    layout: &read::StatFileLayout,
) {
    read::entry(&handles.memory_usage_in_bytes, buffers);
    read::entry(&handles.memory_max_usage_in_bytes, buffers);
    read::entry(&handles.memory_limit_in_bytes, buffers);
    read::entry(&handles.memory_soft_limit_in_bytes, buffers);
    read::entry(&handles.memory_failcnt, buffers);
    read::with_layout(&handles.memory_stat, layout, buffers);
}

/// Collects all stats for the blkio subsystem
/// see <https://www.kernel.org/doc/Documentation/cgroup-v1/blkio-controller.txt>
#[inline]
fn collect_blkio(buffers: &mut WorkingBuffers, handles: &ProcFileHandles) {
    read::simple_io(&handles.blkio_time, buffers);
    read::simple_io(&handles.blkio_sectors, buffers);
    read::io(&handles.blkio_io_service_bytes, buffers);
    read::io(&handles.blkio_io_serviced, buffers);
    read::io(&handles.blkio_io_service_time, buffers);
    read::io(&handles.blkio_io_queued, buffers);
    read::io(&handles.blkio_io_wait_time, buffers);
    read::io(&handles.blkio_io_merged, buffers);
    read::io(&handles.blkio_throttle_io_service_bytes, buffers);
    read::io(&handles.blkio_throttle_io_serviced, buffers);
    read::io(&handles.blkio_bfq_io_service_bytes, buffers);
    read::io(&handles.blkio_bfq_io_serviced, buffers);
}
