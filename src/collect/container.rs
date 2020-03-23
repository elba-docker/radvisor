use crate::collect::buffer::{self, Buffer, BufferLike};
use crate::collect::collector::{Collector, ProcFileHandles};
use crate::util;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

use numtoa::NumToA;
use lazy_static::lazy_static;

use csv::{ByteRecord, Error};

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
        "cpu.throttling.throttled",
        "cpu.throttling.throttled_time"
    ]);
    /// Length of each row of the collected stats
    static ref ROW_LENGTH: usize = HEADER.len();
    /// Empty buffer
    static ref EMPTY_BUFFER: [u8; 0] = [];
}

/// Length of the buffer for each row. Designed to be a reasonable upper limit to prevent
/// expensive re-allocation
const ROW_BUFFER_SIZE: usize = 1200;

/// Gets an amortized byte record containing the entries for a header row in the stats
/// CSV log files
pub fn get_header() -> &'static ByteRecord {
    &HEADER
}

/// Initializes all file handles to /proc files, utilizing them over the entire timeline of
/// the container monitoring. If a handle fails to open, the struct field will be None
pub fn initialize_file_handles(id: &str) -> ProcFileHandles {
    let current_pids = open_proc_file(id, "pids", "pids.current");
    let max_pids = open_proc_file(id, "pids", "pids.max");
    let cpu_stat = open_proc_file(id, "cpu", "cpu.stat");
    let cpuacct_stat = open_proc_file(id, "cpuacct", "cpuacct.stat");
    let cpuacct_usage = open_proc_file(id, "cpuacct", "cpuacct.usage");
    let cpuacct_usage_sys = open_proc_file(id, "cpuacct", "cpuacct.usage_sys");
    let cpuacct_usage_user = open_proc_file(id, "cpuacct", "cpuacct.usage_user");
    let cpuacct_usage_percpu = open_proc_file(id, "cpuacct", "cpuacct.usage_percpu");
    ProcFileHandles {
        current_pids,
        max_pids,
        cpu_stat,
        cpuacct_stat,
        cpuacct_usage,
        cpuacct_usage_sys,
        cpuacct_usage_user,
        cpuacct_usage_percpu,
    }
}

/// Opens a stats file in /proc for the cgroup corresponding to the given container ID,
/// in the given subsystem
fn open_proc_file(id: &str, subsystem: &str, file: &str) -> Option<File> {
    File::open(format!(
        "/sys/fs/cgroup/{}/docker/{}/{}",
        subsystem, id, file
    ))
    .ok()
}

/// Working buffers used to avoid heap allocations at runtime
pub struct WorkingBuffers {
    record: ByteRecord,
    buffer: Buffer,
}

impl WorkingBuffers {
    /// Allocates the working buffers on the heap using upper limits to avoid expensive
    /// heap allocations at runtime
    pub fn new() -> WorkingBuffers {
        let record = ByteRecord::with_capacity(ROW_BUFFER_SIZE, *ROW_LENGTH);
        let buffer = Buffer::new();
        WorkingBuffers { record, buffer }
    }
}

/// Collects the current statistics for the given container, writing the CSV entries to
/// the writer. Utilizes /proc and cgroups (Linux-only)
pub fn collect(buffers: &mut WorkingBuffers, collector: &mut Collector) -> Result<(), Error> {
    collect_read(&mut buffers.record, &mut buffers.buffer);
    collect_pids(
        &mut buffers.record,
        &mut buffers.buffer,
        &collector.file_handles,
    );
    collect_cpu(
        &mut buffers.record,
        &mut buffers.buffer,
        &collector.file_handles,
    );
    collector.writer.write_byte_record(&buffers.record)?;
    buffers.record.clear();
    Ok(())
}

/// Collects the nanosecond unix timestamp read time
#[inline]
fn collect_read(record: &mut ByteRecord, buffer: &mut Buffer) -> () {
    record.push_field(util::nano_ts().numtoa(10, &mut buffer.b));
    // numtoa writes starting at the end of the buffer, so clear backwards
    buffer.clear_unmanaged_backwards();
}

/// Collects all stats for the pids subsystem
/// see https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v1/pids.html
#[inline]
fn collect_pids(record: &mut ByteRecord, buffer: &mut Buffer, handles: &ProcFileHandles) -> () {
    try_file_to_entry(&handles.current_pids, record, buffer);
    try_file_to_entry(&handles.max_pids, record, buffer);
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
fn collect_cpu(record: &mut ByteRecord, buffer: &mut Buffer, handles: &ProcFileHandles) -> () {
    try_file_to_entry(&handles.cpuacct_usage, record, buffer);
    try_file_to_entry(&handles.cpuacct_usage_sys, record, buffer);
    try_file_to_entry(&handles.cpuacct_usage_user, record, buffer);
    try_file_to_entry(&handles.cpuacct_usage_percpu, record, buffer);
    try_stat_file_to_entries(&handles.cpuacct_stat, &CPUACCT_STAT_OFFSETS, record, buffer);
    try_stat_file_to_entries(&handles.cpu_stat, &CPU_STAT_OFFSETS, record, buffer);
}

/// Tries to read the given file handle, and directly write the contents as a field to the record
fn try_file_to_entry(file: &Option<File>, record: &mut ByteRecord, buffer: &mut Buffer) -> () {
    if let Some(file) = file {
        let mut file = file;
        // Ignore errors: if either operation fails, then the result will be pushing empty
        // buffers to the CSV rows, which lets the other monitoring continue
        match file.read(&mut buffer.b) {
            Ok(len) => buffer.len += len,
            _ => {}
        };
        let _ = file.seek(SeekFrom::Start(0));
    }

    let trimmed = buffer.trim();
    if buffer::content_len_raw(trimmed) == 0 {
        // Buffer ended up empty; prevent writing NUL bytes
        record.push_field(&EMPTY_BUFFER[..]);
    } else {
        record.push_field(&trimmed);
    }
    
    buffer.clear()
}

fn try_stat_file_to_entries(
    file: &Option<File>,
    offsets: &[usize],
    record: &mut ByteRecord,
    buffer: &mut Buffer,
) -> () {
    // track whether we should keep parsing or if we should just fill in the entries
    // with empty buffers
    let mut successful = true;
    if let Some(file) = file {
        let mut file = file;
        match file.read(&mut buffer.b) {
            Ok(len) => {
                buffer.len += len;
                if len == 0 {
                    successful = false;
                }
            }
            Err(_) => successful = false,
        };
        // Ignore errors: if seeking fails, then the result will be pushing empty
        // buffers to the CSV rows, which lets the other monitoring continue
        let _ = file.seek(SeekFrom::Start(0));
    } else {
        successful = false;
    }

    let mut line_start = 0;
    for i in 0..offsets.len() {
        if successful {
            // Parse next
            let target = line_start + offsets[i] + 1;
            let mut newline = target;
            if target < buffer.len {
                // Find the location of the newline
                loop {
                    let char_at = buffer.b[newline];
                    if util::is_newline(char_at) {
                        break;
                    } else if char_at == 0 {
                        // unexpected end of string
                        successful = false;
                        break;
                    } else {
                        newline += 1;
                    }
                }

                // Push the parsed number as the next field
                let number_slice = &buffer.b[target..newline];
                record.push_field(buffer::trim_raw(number_slice));
                // Set line_start to the start of the next line (after newline)
                line_start = newline + 1;
                continue;
            }
        }

        // If execution fell through (unsuccessful), write empty slice to record
        record.push_field(&EMPTY_BUFFER[..]);
    }

    buffer.clear();
}
