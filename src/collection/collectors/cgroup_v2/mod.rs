mod files;
mod read;

use crate::collection::buffers::WorkingBuffers;
use crate::collection::collectors::{Collector as CollectorTrait, StatWriter};
use crate::collection::perf_table::{Column, ColumnType, TableMetadata};
use crate::util::{self, CgroupDriver, CgroupPath};
use anyhow::Error;
use csv::ByteRecord;
use files::ProcFileHandles;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Implements `crate::collection::collector::Collector`
/// for cgroup v2-sourced data
pub struct Collector {
    cgroup:       CgroupPath,
    file_handles: Option<ProcFileHandles>,
}

impl Collector {
    pub const fn new(cgroup: CgroupPath) -> Self {
        Self {
            cgroup,
            file_handles: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
struct Metadata<'a> {
    cgroup:        &'a PathBuf,
    cgroup_driver: &'a CgroupDriver,
}

impl CollectorTrait for Collector {
    fn metadata(&mut self) -> Option<serde_yaml::Value> {
        let metadata = Metadata {
            cgroup:        &self.cgroup.path,
            cgroup_driver: &self.cgroup.driver,
        };

        serde_yaml::to_value(&metadata).ok()
    }

    fn table_metadata(&mut self) -> TableMetadata {
        let mut columns: BTreeMap<String, Column> = BTreeMap::new();
        // Include metadata on the read (timestamp) column
        columns.insert(String::from("read"), Column::Scalar {
            r#type: ColumnType::Epoch19,
        });
        TableMetadata {
            delimiter: ",",
            columns,
        }
    }

    fn get_type(&self) -> &'static str { "cgroup_v2" }

    fn init(&mut self) -> Result<(), Error> {
        // Open file handles to all of the /proc files in the cgroupfs
        let handles = ProcFileHandles::new(&self.cgroup.path);
        self.file_handles = Some(handles);
        Ok(())
    }

    fn write_header(&mut self, writer: &mut StatWriter) -> Result<(), csv::Error> {
        writer.write_byte_record(&HEADER)
    }

    fn collect(
        &mut self,
        writer: &mut StatWriter,
        working_buffers: &mut WorkingBuffers,
    ) -> Result<(), csv::Error> {
        let file_handles = self
            .file_handles
            .as_ref()
            .expect("file handles not yet initialized during collect()");

        collect_read(working_buffers);
        let pids_result = collect_pids(working_buffers, file_handles);
        let cpu_result = collect_cpu(working_buffers, file_handles);
        let memory_result = collect_memory(working_buffers, file_handles);
        let io_result = collect_io(working_buffers, file_handles);

        // If all of the cgroup file reads were empty,
        // skip writing the byte record.
        if pids_result == Err(read::Empty)
            && cpu_result == Err(read::Empty)
            && memory_result == Err(read::Empty)
            && io_result == Err(read::Empty)
        {
            // Discard the working record
            working_buffers.record.clear();
        } else {
            let result = writer.write_byte_record(&working_buffers.record);
            working_buffers.record.clear();
            result?;
        }

        Ok(())
    }
}

lazy_static::lazy_static! {
    /// Static CSV header for the stats collector
    static ref HEADER: ByteRecord = ByteRecord::from(get_headers());
}

/// Creates the headers for the logfiles
#[allow(clippy::vec_init_then_push)]
fn get_headers() -> Vec<String> {
    let mut headers: Vec<String> = vec![];
    // Add read headers
    headers.push("read".into());
    // Add pids headers
    headers.push("pids.current".into());
    headers.push("pids.max".into());
    // Add cpu headers
    for cpu_stat_key in CPU_STAT_KEYS {
        headers.push(format!(
            "cpu.stat/{}",
            String::from_utf8(cpu_stat_key.to_vec()).unwrap()
        ));
    }
    // Add memory headers
    headers.push("memory.current".into());
    headers.push("memory.high".into());
    headers.push("memory.max".into());
    for memory_stat_key in MEMORY_STAT_KEYS {
        headers.push(format!(
            "memory.stat/{}",
            String::from_utf8(memory_stat_key.to_vec()).unwrap()
        ));
    }
    // Add io headers
    for io_stat_key in IO_STAT_KEYS {
        headers.push(format!(
            "io.stat/{}",
            String::from_utf8(io_stat_key.to_vec()).unwrap()
        ));
    }

    headers
}

/// Collects the nanosecond unix timestamp read time
#[inline]
fn collect_read(buffers: &mut WorkingBuffers) {
    let nano_ts = util::nano_ts();
    let mut itoa_buffer = itoa::Buffer::new();
    let formatted = itoa_buffer.format(nano_ts);
    buffers.record.push_field(formatted.as_bytes());
}

/// Collects all stats for the pids controller
/// see <https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html#pid>
#[inline]
fn collect_pids(
    buffers: &mut WorkingBuffers,
    handles: &ProcFileHandles,
) -> Result<(), read::Empty> {
    let pids_current = read::single_value_file(&handles.pids_current, buffers, b"0");
    let pids_max = read::single_value_file(&handles.pids_max, buffers, b"max");
    if pids_current == Err(read::Empty) && pids_max == Err(read::Empty) {
        Err(read::Empty)
    } else {
        Ok(())
    }
}

/// Keys to read from the cpu.stat file
const CPU_STAT_KEYS: [&[u8]; 6] = [
    b"usage_usec",
    b"system_usec",
    b"user_usec",
    b"nr_periods",
    b"nr_throttled",
    b"throttled_usec",
];
const CPU_STAT_DEFAULTS: [&[u8]; 6] = [b"0"; 6];

/// Collects all stats for the cpu controller
/// see <https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html#cpu>
#[inline]
fn collect_cpu(buffers: &mut WorkingBuffers, handles: &ProcFileHandles) -> Result<(), read::Empty> {
    read::flat_keyed_file(
        &handles.cpu_stat,
        buffers,
        &CPU_STAT_KEYS,
        &CPU_STAT_DEFAULTS,
    )
}

/// Keys to read from the memory.stat file
const MEMORY_STAT_KEYS: [&[u8]; 18] = [
    b"anon",
    b"file",
    b"kernel_stack",
    b"pagetables",
    b"percpu",
    b"sock",
    b"shmem",
    b"file_mapped",
    b"file_dirty",
    b"file_writeback",
    b"swapcached",
    b"inactive_anon",
    b"active_anon",
    b"inactive_file",
    b"active_file",
    b"unevictable",
    b"pgfault",
    b"pgmajfault",
];
const MEMORY_STAT_DEFAULTS: [&[u8]; 18] = [b"0"; 18];

/// Collects all stats for the memory controller
/// see <https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html#memory>
#[inline]
fn collect_memory(
    buffers: &mut WorkingBuffers,
    handles: &ProcFileHandles,
) -> Result<(), read::Empty> {
    let mem_current = read::single_value_file(&handles.memory_current, buffers, b"0");
    let mem_high = read::single_value_file(&handles.memory_high, buffers, b"max");
    let mem_max = read::single_value_file(&handles.memory_max, buffers, b"max");
    let mem_stat = read::flat_keyed_file(
        &handles.memory_stat,
        buffers,
        &MEMORY_STAT_KEYS,
        &MEMORY_STAT_DEFAULTS,
    );
    if mem_current == Err(read::Empty)
        && mem_high == Err(read::Empty)
        && mem_max == Err(read::Empty)
        && mem_stat == Err(read::Empty)
    {
        Err(read::Empty)
    } else {
        Ok(())
    }
}

/// Keys to read and get totals for from the io.stat file
const IO_STAT_KEYS: [&[u8]; 6] = [b"rbytes", b"wbytes", b"rios", b"wios", b"dbytes", b"dios"];

/// Collects all stats for the io controller
/// see <https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html#io>
#[inline]
fn collect_io(buffers: &mut WorkingBuffers, handles: &ProcFileHandles) -> Result<(), read::Empty> {
    read::io_stat_file(&handles.io_stat, buffers, &IO_STAT_KEYS)
}
