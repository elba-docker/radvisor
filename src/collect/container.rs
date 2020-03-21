use crate::collect::collector::{Collector, ProcFileHandles};
use crate::util;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

use lazy_static::lazy_static;

use csv::{ByteRecord, Error};

lazy_static! {
    /// CSV header for the stats collector
    static ref HEADER: ByteRecord = ByteRecord::from(vec![
        "read", "pids.current", "pids.max"
    ]);
    /// Length of each row of the collected stats
    static ref ROW_LENGTH: usize = HEADER.len();
    /// Empty record field used upon failure to read from /proc
    static ref EMPTY_FIELD: Vec<u8> = Vec::with_capacity(0);
}

/// Length of the buffer for each row. Designed to be a reasonable upper limit to prevent
/// expensive re-allocation
const ROW_BUFFER_SIZE: usize = 1200;

/// Length of the buffer used to read proc files in with. Designed to be an upper limit for
/// the various virtual files that need to be read
const WORKING_BUFFER_SIZE: usize = 700;

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
    ProcFileHandles {
        current_pids,
        max_pids,
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

/// Collects the current statistics for the given container, writing the CSV entries to
/// the writer. Utilizes /proc and cgroups (Linux-only)
pub fn collect(collector: &mut Collector) -> Result<(), Error> {
    let mut record = ByteRecord::with_capacity(ROW_BUFFER_SIZE, *ROW_LENGTH);
    let mut buffer = Vec::with_capacity(WORKING_BUFFER_SIZE);

    // Write records
    record.push_field(util::nano_ts().to_string().as_bytes());
    collect_pids(&mut record, &mut buffer, &collector.file_handles);

    collector.writer.write_byte_record(&record)?;
    Ok(())
}

/// Collects all stats for the pids subsystem
/// see https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v1/pids
fn collect_pids(record: &mut ByteRecord, buffer: &mut Vec<u8>, handles: &ProcFileHandles) -> () {
    try_file_to_entry(&handles.current_pids, record, buffer);
    try_file_to_entry(&handles.max_pids, record, buffer);
}

/// Tries to read the given file handle, and directly write the contents as a field to the record
fn try_file_to_entry(file: &Option<File>, record: &mut ByteRecord, buffer: &mut Vec<u8>) -> () {
    if let Some(file) = file {
        let mut file = file;
        // Ignore results: if either operation fail, then the result will be pushing empty
        // buffers to the CSV rows, which lets the other monitoring continue
        let _ = file.read_to_end(buffer);
        let _ = file.seek(SeekFrom::Start(0));
    }
    record.push_field(remove_trailing_newline(&buffer));
    buffer.clear();
}

/// Newline char as u8
static N: u8 = '\n' as u8;

/// Removes the trailing newline from the given slice
fn remove_trailing_newline(buf: &[u8]) -> &[u8] {
    match buf.len() != 0 && buf[buf.len() - 1] == N {
        // Make sure the buffer isn't empty before doing len - 1
        true => &buf[0..buf.len() - 1],
        false => buf,
    }
}
