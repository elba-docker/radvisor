use crate::collection::collect::buffer::{Buffer, BufferLike};
use crate::collection::collect::files::ProcFileHandles;
use crate::collection::collector::Collector;
use crate::util;

use csv::{ByteRecord, Error};
use lazy_static::lazy_static;
use numtoa::NumToA;

pub mod buffer;
pub mod files;
pub mod read;

lazy_static! {
    /// CSV header for the stats collector
    static ref HEADER: ByteRecord = ByteRecord::from(vec![
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
    ]);
    /// Length of each row of the collected stats
    static ref ROW_LENGTH: usize = HEADER.len();
    /// Empty buffer
    static ref EMPTY_BUFFER: [u8; 0] = [];
}

/// Gets an amortized byte record containing the entries for a header row in the stats
/// CSV log files
pub fn get_header() -> &'static ByteRecord {
    &HEADER
}

/// Length of the buffer for each row. Designed to be a reasonable upper limit to prevent
/// expensive re-allocation
const ROW_BUFFER_SIZE: usize = 1200;

/// Length of the buffer used to read proc files in with. Designed to be an upper
/// limit for the various virtual files that need to be read
const WORKING_BUFFER_SIZE: usize = 1024;

/// Working buffers used to avoid heap allocations at runtime
pub struct WorkingBuffers {
    record: ByteRecord,
    buffer: Buffer<WORKING_BUFFER_SIZE>,
}

impl WorkingBuffers {
    /// Allocates the working buffers on the heap using upper limits to avoid expensive
    /// heap allocations at runtime
    pub fn new() -> WorkingBuffers {
        let record = ByteRecord::with_capacity(ROW_BUFFER_SIZE, *ROW_LENGTH);
        let buffer = Buffer {
            len: 0,
            b: [0u8; WORKING_BUFFER_SIZE],
        };
        WorkingBuffers { record: record, buffer: buffer }
    }
}

/// Collects the current statistics for the given container, writing the CSV entries to
/// the writer. Utilizes /proc and cgroups (Linux-only)
pub fn run(collector: &mut Collector, buffers: &mut WorkingBuffers) -> Result<(), Error> {
    collect_read(buffers);
    collect_pids(buffers, &collector.file_handles);
    collect_cpu(buffers, &collector.file_handles);
    collect_memory(buffers, &collector.file_handles);
    collector.writer.write_byte_record(&buffers.record)?;
    buffers.record.clear();
    Ok(())
}

/// Collects the nanosecond unix timestamp read time
#[inline]
fn collect_read(buffers: &mut WorkingBuffers) -> () {
    buffers.record.push_field(util::nano_ts().numtoa(10, &mut buffers.buffer.b));
    // numtoa writes starting at the end of the buffer, so clear backwards
    buffers.buffer.clear_unmanaged_backwards();
}

/// Collects all stats for the pids subsystem
/// see https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v1/pids.html
#[inline]
fn collect_pids(buffers: &mut WorkingBuffers, handles: &ProcFileHandles) -> () {
    read::entry(&handles.current_pids, &mut buffers.record, &mut buffers.buffer);
    read::entry(&handles.max_pids, &mut buffers.record, &mut buffers.buffer);
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
/// see https://access.redhat.com/documentation/en-us/red_hat_enterprise_linux/6/html/resource_management_guide/sec-cpuacct
#[inline]
fn collect_cpu(buffers: &mut WorkingBuffers, handles: &ProcFileHandles) -> () {
    read::entry(&handles.cpuacct_usage, &mut buffers.record, &mut buffers.buffer);
    read::entry(&handles.cpuacct_usage_sys, &mut buffers.record, &mut buffers.buffer);
    read::entry(&handles.cpuacct_usage_user, &mut buffers.record, &mut buffers.buffer);
    read::entry(&handles.cpuacct_usage_percpu, &mut buffers.record, &mut buffers.buffer);
    read::stat_file(&handles.cpuacct_stat, &CPUACCT_STAT_OFFSETS, &mut buffers.record, &mut buffers.buffer);
    read::stat_file(&handles.cpu_stat, &CPU_STAT_OFFSETS, &mut buffers.record, &mut buffers.buffer);
}

/// Collects all stats for the memory subsystem
/// see https://access.redhat.com/documentation/en-us/red_hat_enterprise_linux/6/html/resource_management_guide/sec-memory
#[inline]
fn collect_memory(buffers: &mut WorkingBuffers, handles: &ProcFileHandles) -> () {
    read::entry(&handles.memory_usage_in_bytes, &mut buffers.record, &mut buffers.buffer);
    read::entry(&handles.memory_max_usage_in_bytes, &mut buffers.record, &mut buffers.buffer);
    read::entry(&handles.memory_limit_in_bytes, &mut buffers.record, &mut buffers.buffer);
    read::entry(&handles.memory_soft_limit_in_bytes, &mut buffers.record, &mut buffers.buffer);
    read::entry(&handles.memory_failcnt, &mut buffers.record, &mut buffers.buffer);
}
