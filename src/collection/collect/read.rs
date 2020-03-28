use crate::collection::collect::{AnonymousSlice, WorkingBuffers};
use crate::util;
use crate::util::buffer::{self, Buffer, BufferLike};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

use lazy_static::lazy_static;

lazy_static! {
    static ref EMPTY_BUFFER: &'static [u8] = &[];
}

/// Tries to read the given file handle, and directly write the contents as a field to the record
pub fn entry(file: &Option<File>, buffers: &mut WorkingBuffers) -> () {
    // Ignore errors: the buffer will just remain empty
    read_to_buffer(file, buffers);

    let trimmed = buffers.buffer.trim();
    if buffer::content_len_raw(trimmed) == 0 {
        // Buffer ended up empty; prevent writing NUL bytes
        buffers.record.push_field(&EMPTY_BUFFER[..]);
    } else {
        buffers.record.push_field(&trimmed);
    }

    buffers.buffer.clear()
}

/// Parses every entry in a stats file, where each entry is a alphabetic key followed by a number,
/// and then a newline. Attempts to parse offsets.len() entries from the file, using the precomputed
/// offsets array to skip reading the alphabetic key.
pub fn stat_file(file: &Option<File>, offsets: &[usize], buffers: &mut WorkingBuffers) -> () {
    // Track whether we should keep parsing or if we should fill in the entries with empty buffers
    let successful = read_to_buffer(file, buffers).is_some();

    let mut success_count = 0;
    if successful {
        let mut line_start = 0;
        for i in 0..offsets.len() {
            // Parse next
            let target = line_start + offsets[i] + 1;
            if target >= buffers.buffer.len {
                break;
            }
            // Find the location of the newline
            match util::find_char(&buffers.buffer.b, target, util::is_newline) {
                Some(newline) => {
                    // Push the parsed number as the next field
                    let number_slice = &buffers.buffer.b[target..newline];
                    buffers.record.push_field(buffer::trim_raw(number_slice));
                    // Set line_start to the start of the next line (after newline)
                    line_start = newline + 1;
                    success_count += 1;
                }
                None => {
                    break;
                }
            }
        }
    }

    // Write empty buffers for remaining positions that weren't parsed successfully
    for _ in 0..(offsets.len() - success_count) {
        buffers.record.push_field(&EMPTY_BUFFER[..]);
    }

    buffers.buffer.clear();
}

/// Used to store the results of an initial examination of the layout of a `.stat` cgroup file,
/// determining how each line maps to the target entries. Used for `.stat` files where the contents
/// vary depending on the system/configuration, such as `memory.stat` in the `memory` subsystem
pub struct StatFileLayout {
    /// Line-to-entry map, where if a line corresponds to an indexed entry, its value will be Some
    /// and the inner value will be that index, along with the length of the entry. Otherwise, if
    /// a line doesn't correspond to any indexed entry, its value will be None
    lines: Vec<Option<StatFileLine>>,
}

/// Represents metadata about a single stat file line that corresponds to an entry in the log output.
pub struct StatFileLine {
    /// Index of the corresponding entry
    entry: usize,
    /// Length of the line header to skip when parsing
    offset: usize,
}

impl StatFileLayout {
    /// Examines the layout of a stat file, to determine on which lines predetermined entries exist for
    /// faster processing during collection
    pub fn new(file: &Option<File>, entries: &[&[u8]]) -> Self {
        let mut buffer: Vec<u8> = Vec::new();
        let read_successful = match file {
            None => false,
            Some(file) => {
                let mut file_mut = file;
                let result = file_mut.read_to_end(&mut buffer);
                let _ = file_mut.seek(SeekFrom::Start(0));
                result.is_ok()
            }
        };
        if read_successful {
            let mut lines_to_entries: Vec<Option<StatFileLine>> = Vec::new();
            let lines = util::ByteLines::new(&buffer);
            for (line, _) in lines {
                match util::find_char(&line, 0, util::is_space) {
                    None => {}
                    Some(space_pos) => {
                        let key = buffer::trim_raw(&line[0..space_pos]);
                        let index_option = find_index(entries, key);
                        lines_to_entries.push(index_option.map(|idx| StatFileLine {
                            entry: idx,
                            offset: entries[idx].len(),
                        }));
                    }
                }
            }
            StatFileLayout {
                lines: lines_to_entries,
            }
        } else {
            StatFileLayout {
                lines: Vec::with_capacity(0),
            }
        }
    }
}

/// Tries to find the index in the source array of the given target slice, comparing each byte-by-byte
/// until the target is found. Runs in O(n) time on the length of source
fn find_index(source: &[&[u8]], target: &[u8]) -> Option<usize> {
    for i in 0..source.len() {
        if source[i].len() == target.len() {
            let mut all_equal = true;
            for j in 0..target.len() {
                if source[i][j] != target[j] {
                    all_equal = false;
                    break;
                }
            }
            if all_equal {
                return Some(i);
            }
        }
    }
    None
}

/// Reads and parses a stat file, using a pre-examined layout to quickly read the desired entries
/// from the file.
pub fn with_layout(
    file: &Option<File>,
    layout: &StatFileLayout,
    buffers: &mut WorkingBuffers,
) -> () {
    let successful = read_to_buffer(file, buffers).is_some();
    if successful {
        let lines = util::ByteLines::new(&buffers.buffer.b);
        let mut i = 0;
        for (line, start) in lines {
            match &layout.lines[i] {
                None => {}
                Some(line_metadata) => {
                    let value_start = start + line_metadata.offset + 1;
                    let value_end = start + line.len();
                    buffers.slices[line_metadata.entry] = AnonymousSlice {
                        start: value_start,
                        length: value_end - value_start,
                    }
                }
            }
            i += 1;
        }
    }

    // Write all slices to the record
    for i in 0..buffers.slices.len() {
        let slice: &[u8] = match buffers.slices[i].consume(&buffers.buffer.b) {
            Some(s) => s,
            None => &EMPTY_BUFFER,
        };
        buffers.record.push_field(slice);
    }

    clear_slice_buffer(buffers);
    buffers.buffer.clear();
}

/// Clears the slice buffer, resetting all values to their default
fn clear_slice_buffer(buffers: &mut WorkingBuffers) -> () {
    let default_value = <AnonymousSlice>::default();
    for i in 0..buffers.slices.len() {
        buffers.slices[i] = default_value;
    }
}

/// Attempts to read the given file into the buffer, if it exists. If successful, returns
/// Some with the length of the part of the file read. If the file handle wasn't given, or
/// reading was unsuccessful, returns a None
fn read_to_buffer(file: &Option<File>, buffers: &mut WorkingBuffers) -> Option<usize> {
    match file {
        None => None,
        Some(f) => {
            let mut file_mut = f;
            let result = match file_mut.read(&mut buffers.buffer.b) {
                Err(_) => None,
                Ok(len) => {
                    buffers.buffer.len += len;
                    if len != 0 {
                        Some(len)
                    } else {
                        None
                    }
                }
            };
            // Ignore errors: if seeking fails, then the effect next time will be pushing empty
            // buffers to the CSV rows, which lets the other monitoring continue
            let _ = file_mut.seek(SeekFrom::Start(0));
            result
        }
    }
}

/// Tries to read the entire file, moving each line to a comma-separated string
pub fn all(file: &Option<File>, buffers: &mut WorkingBuffers) -> () {
    // Ignore errors: the buffer will just remain empty
    read_to_buffer(file, buffers);

    let trimmed = buffers.buffer.trim();
    if buffer::content_len_raw(trimmed) == 0 {
        // Buffer ended up empty; prevent writing NUL bytes
        buffers.record.push_field(&EMPTY_BUFFER[..]);
    } else {
        // Copy over to temporary buffer
        copy_lines_to_commas(&buffers.buffer, &mut buffers.copy_buffer);
        buffers
            .record
            .push_field(&buffers.copy_buffer.content_unmanaged());
    }

    buffers.buffer.clear();
    buffers.copy_buffer.clear();
}

pub static COMMA: u8 = ',' as u8;

/// Copies lines from the incoming buffer to the target buffer
fn copy_lines_to_commas<const S: usize, const T: usize>(
    source: &Buffer<S>,
    target: &mut Buffer<T>,
) -> () {
    let mut start = 0;
    let mut comma_at_end = false;
    let lines = util::ByteLines::new(&source.b);
    for (line, _) in lines {
        let end = start + line.len();
        if end >= target.b.len() {
            return;
        }

        target.b[start..end].clone_from_slice(&line);
        target.b[end] = COMMA;
        start = end + 1;
        comma_at_end = true;
    }

    // if last was written to, reset comma to NUL terminator
    if comma_at_end {
        target.b[start - 1] = 0u8;
    }
}
