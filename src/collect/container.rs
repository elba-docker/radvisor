use crate::util;
use std::fs::File;

use lazy_static::lazy_static;

use csv::{ByteRecord, Writer, Error};

lazy_static! {
    /// CSV header for the stats collector
    static ref HEADER: ByteRecord = ByteRecord::from(vec![
        "read", "id",
    ]);
    /// Length of each row of the collected stats
    static ref ROW_LENGTH: usize = HEADER.len();
}

/// Length of the buffer for each row. Designed to be a reasonable upper limit to prevent
/// expensive re-allocation
const ROW_BUFFER_SIZE: usize = 1200;

/// Gets an amortized byte record containing the entries for a header row in the stats
/// CSV log files
pub fn get_header() -> &'static ByteRecord {
    &HEADER
}

/// Collects the current statistics for the given container, writing the CSV entries to
/// the writer. Utilizes /proc and cgroups (Linux-only)
pub fn collect(id: &str, writer: &mut Writer<File>) -> Result<(), Error> {
    let mut record = ByteRecord::with_capacity(ROW_BUFFER_SIZE, *ROW_LENGTH);

    // Write records
    record.push_field(util::nano_ts().to_string().as_bytes());
    record.push_field(id.as_bytes());

    writer.write_byte_record(&record)?;
    Ok(())
}
