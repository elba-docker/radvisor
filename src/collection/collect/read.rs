use crate::collection::collect::buffer::{self, Buffer, BufferLike};
use crate::util;
use std::fs::File;
use std::io::{SeekFrom, Seek, Read};

use csv::ByteRecord;
use lazy_static::lazy_static;

lazy_static! {
    static ref EMPTY_BUFFER: [u8; 0] = [];
}

/// Tries to read the given file handle, and directly write the contents as a field to the record
pub fn entry<const S: usize>(
    file: &Option<File>,
    record: &mut ByteRecord,
    buffer: &mut Buffer<S>,
) -> () {
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

pub fn stat_file<const S: usize>(
    file: &Option<File>,
    offsets: &[usize],
    record: &mut ByteRecord,
    buffer: &mut Buffer<S>,
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
