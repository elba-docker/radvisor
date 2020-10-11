use crate::collection::collect::{AnonymousSlice, WorkingBuffers};
use crate::util::{self, BufferLike, LazyQuantity};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

const EMPTY_BUFFER: &[u8] = &[];

/// Tries to read the given file handle, and directly write the contents as a
/// field to the record
pub fn entry(file: &Option<File>, buffers: &mut WorkingBuffers) {
    // Ignore errors: the buffer will just remain empty
    read_to_buffer(file, buffers);

    let trimmed = buffers.buffer.trim();
    if util::content_len_raw(trimmed) == 0 {
        // Buffer ended up empty; prevent writing NUL bytes
        buffers.record.push_field(&EMPTY_BUFFER[..]);
    } else {
        buffers.record.push_field(trimmed);
    }

    buffers.buffer.clear()
}

/// Parses every entry in a stats file, where each entry is a alphabetic key
/// followed by a number, and then a newline. Attempts to parse offsets.len()
/// entries from the file, using the precomputed offsets array to skip reading
/// the alphabetic key.
pub fn stat_file(file: &Option<File>, offsets: &[usize], buffers: &mut WorkingBuffers) {
    // Track whether we should keep parsing or if we should fill in the entries with
    // empty buffers
    let successful = read_to_buffer(file, buffers).is_some();

    let mut success_count = 0;
    if successful {
        let mut line_start = 0;
        for offset in offsets {
            // Parse next
            let target = line_start + offset + 1;
            if target >= buffers.buffer.len {
                break;
            }
            // Find the location of the newline
            if let Some(newline) = util::find_char(&buffers.buffer.b, target, util::is_newline) {
                // Push the parsed number as the next field
                let number_slice = &buffers.buffer.b[target..newline];
                buffers.record.push_field(util::trim_raw(number_slice));
                // Set line_start to the start of the next line (after newline)
                line_start = newline + 1;
                success_count += 1;
            } else {
                break;
            }
        }
    }

    // Write empty buffers for remaining positions that weren't parsed successfully
    for _ in 0..(offsets.len() - success_count) {
        buffers.record.push_field(&EMPTY_BUFFER[..]);
    }

    buffers.buffer.clear();
}

/// Used to store the results of an initial examination of the layout of a
/// `.stat` cgroup file, determining how each line maps to the target entries.
/// Used for `.stat` files where the contents vary depending on the
/// system/configuration, such as `memory.stat` in the `memory` subsystem
pub struct StatFileLayout {
    /// Line-to-entry map, where if a line corresponds to an indexed entry, its
    /// value will be Some and the inner value will be that index, along
    /// with the length of the entry. Otherwise, if a line doesn't
    /// correspond to any indexed entry, its value will be None
    lines: Vec<Option<StatFileLine>>,
}

/// Represents metadata about a single stat file line that corresponds to an
/// entry in the log output.
pub struct StatFileLine {
    /// Index of the corresponding entry
    entry:  usize,
    /// Length of the line header to skip when parsing
    offset: usize,
}

impl StatFileLayout {
    /// Examines the layout of a stat file, to determine on which lines
    /// predetermined entries exist for faster processing during collection
    #[must_use]
    pub fn new(file: &Option<File>, entries: &[&[u8]]) -> Self {
        let mut buffer: Vec<u8> = Vec::new();
        let read_successful = match file {
            None => false,
            Some(file) => {
                let mut file_mut = file;
                let result = file_mut.read_to_end(&mut buffer);
                // Ignore errors: if seeking fails, then the effect next time will be pushing
                // empty buffers to the CSV rows, which lets the other monitoring
                // continue
                let _ = file_mut.seek(SeekFrom::Start(0));
                result.is_ok()
            },
        };
        if read_successful {
            let mut lines_to_entries: Vec<Option<StatFileLine>> = Vec::new();
            let lines = util::ByteLines::new(&buffer);
            for (line, _) in lines {
                match util::find_char(line, 0, util::is_space) {
                    None => {},
                    Some(space_pos) => {
                        let key = util::trim_raw(&line[0..space_pos]);
                        let index_option = find_index(entries, key);
                        lines_to_entries.push(index_option.map(|idx| StatFileLine {
                            entry:  idx,
                            offset: entries[idx].len(),
                        }));
                    },
                }
            }
            Self {
                lines: lines_to_entries,
            }
        } else {
            Self {
                lines: Vec::with_capacity(0),
            }
        }
    }
}

/// Tries to find the index in the source array of the given target slice,
/// comparing each byte-by-byte until the target is found. Runs in O(n) time on
/// the length of source
fn find_index(source: &[&[u8]], target: &[u8]) -> Option<usize> {
    for (i, slice) in source.iter().enumerate() {
        if slice.len() == target.len() {
            let mut all_equal = true;
            for j in 0..target.len() {
                if slice[j] != target[j] {
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

/// Reads and parses a stat file, using a pre-examined layout to quickly read
/// the desired entries from the file.
pub fn with_layout(file: &Option<File>, layout: &StatFileLayout, buffers: &mut WorkingBuffers) {
    let successful = read_to_buffer(file, buffers).is_some();
    if successful {
        let lines = util::ByteLines::new(&buffers.buffer.b);
        for (i, (line, start)) in lines.enumerate() {
            match &layout.lines[i] {
                None => {},
                Some(line_metadata) => {
                    let value_start = start + line_metadata.offset + 1;
                    let value_end = start + line.len();
                    buffers.slices[line_metadata.entry] = AnonymousSlice {
                        start:  value_start,
                        length: value_end - value_start,
                    }
                },
            }
        }
    }

    // Write all slices to the record
    for i in 0..buffers.slices.len() {
        let slice: &[u8] = match buffers.slices[i].consume(&buffers.buffer.b) {
            Some(s) => s,
            None => EMPTY_BUFFER,
        };
        buffers.record.push_field(slice);
    }

    clear_slice_buffer(buffers);
    buffers.buffer.clear();
}

/// Clears the slice buffer, resetting all values to their default
fn clear_slice_buffer(buffers: &mut WorkingBuffers) {
    let default_value = <AnonymousSlice>::default();
    for i in 0..buffers.slices.len() {
        buffers.slices[i] = default_value;
    }
}

/// Attempts to read the given file into the buffer, if it exists. If
/// successful, returns Some with the length of the part of the file read. If
/// the file handle wasn't given, or reading was unsuccessful, returns a None
fn read_to_buffer(file: &Option<File>, buffers: &mut WorkingBuffers) -> Option<usize> {
    match file {
        None => None,
        Some(f) => {
            let mut file_mut = f;
            let result = match file_mut.read(&mut buffers.buffer.b) {
                Err(_) => None,
                Ok(len) => {
                    buffers.buffer.len += len;
                    if len == 0 {
                        None
                    } else {
                        Some(len)
                    }
                },
            };
            // Ignore errors: if seeking fails, then the effect next time will be pushing
            // empty buffers to the CSV rows, which lets the other monitoring
            // continue
            let _ = file_mut.seek(SeekFrom::Start(0));
            result
        },
    }
}

/// Group of I/O quantities that are added up
#[derive(Default)]
struct IoQuantities<'a> {
    read_total:  LazyQuantity<'a, u64>,
    write_total: LazyQuantity<'a, u64>,
    sync_total:  LazyQuantity<'a, u64>,
    async_total: LazyQuantity<'a, u64>,
}

/// Tries to read an IO file and creates aggregate stats for
/// read, write, sync, and async.
/// The original files are in the form of:
/// ```txt
/// 8:0 Read 4272128
/// 8:0 Write 0
/// 8:0 Sync 4272128
/// 8:0 Async 0
/// 8:0 Discard 0
/// 8:0 Total 4272128
/// 11:0 Read 1073152
/// 11:0 Write 0
/// 11:0 Sync 1073152
/// 11:0 Async 0
/// 11:0 Discard 0
/// 11:0 Total 1073152
/// Total 5345280
/// ```
pub fn io(file: &Option<File>, buffers: &mut WorkingBuffers) {
    // Ignore errors: the buffer will just remain empty
    read_to_buffer(file, buffers);

    let trimmed = buffers.buffer.trim();
    if util::content_len_raw(trimmed) == 0 {
        // Buffer ended up empty; prevent writing NUL bytes
        buffers.record.push_field(&EMPTY_BUFFER[..]);
        buffers.record.push_field(&EMPTY_BUFFER[..]);
        buffers.record.push_field(&EMPTY_BUFFER[..]);
        buffers.record.push_field(&EMPTY_BUFFER[..]);
    } else {
        // Scan each line and aggregate into 4 records
        aggregate_lines(buffers);
    }

    buffers.buffer.clear();
}

/// Scans each line in the buffer and aggregates the trailing numbers
/// to make entries for read, write, sync, and async
fn aggregate_lines<'a>(buffers: &'a mut WorkingBuffers) {
    // File contained contents:
    // parse each line and keep track of each total
    let mut quantities: IoQuantities<'a> = IoQuantities::default();
    let lines = util::ByteLines::new(&buffers.buffer.b);
    for (line, _) in lines {
        // Get the category in the middle
        if let Some(space) = util::find_char(line, 0, util::is_space) {
            let category_to_end = &line[(space + 1)..];
            if let Some(number_slice) = parse_category(category_to_end, b"Read") {
                quantities.read_total = quantities.read_total.plus(number_slice);
            } else if let Some(number_slice) = parse_category(category_to_end, b"Write") {
                quantities.write_total = quantities.write_total.plus(number_slice);
            } else if let Some(number_slice) = parse_category(category_to_end, b"Sync") {
                quantities.sync_total = quantities.sync_total.plus(number_slice);
            } else if let Some(number_slice) = parse_category(category_to_end, b"Async") {
                quantities.async_total = quantities.async_total.plus(number_slice);
            }
        }
    }

    // Add each quantity to the record (consuming them)
    let IoQuantities {
        read_total,
        write_total,
        sync_total,
        async_total,
    } = quantities;
    read_total.write_to_record(&mut buffers.copy_buffer, &mut buffers.record);
    write_total.write_to_record(&mut buffers.copy_buffer, &mut buffers.record);
    sync_total.write_to_record(&mut buffers.copy_buffer, &mut buffers.record);
    async_total.write_to_record(&mut buffers.copy_buffer, &mut buffers.record);
}

/// Determines if the slice starts with the given category prefix,
/// and if it does, parses the number at the end of the slice
fn parse_category<'a>(slice: &'a [u8], prefix: &[u8]) -> Option<&'a [u8]> {
    if slice.len() < prefix.len() {
        return None;
    }

    // Make sure that the slice starts with the category prefix
    for (i, b) in prefix.iter().enumerate() {
        if slice[i] != *b {
            return None;
        }
    }

    // Search for the second delimiter
    if let Some(space) = util::find_char(slice, 0, util::is_space) {
        return Some(&slice[(space + 1)..]);
    }

    None
}

/// Tries to read a simple IO file and aggregates to make a total.
/// The original files are in the form of:
/// ```txt
/// 8:0 213264
/// 11:0 0
/// ```
pub fn simple_io(file: &Option<File>, buffers: &mut WorkingBuffers) {
    // Ignore errors: the buffer will just remain empty
    read_to_buffer(file, buffers);

    let trimmed = buffers.buffer.trim();
    if util::content_len_raw(trimmed) == 0 {
        // Buffer ended up empty; prevent writing NUL bytes
        buffers.record.push_field(&EMPTY_BUFFER[..]);
    } else {
        // Scan each line and aggregate into a single record
        aggregate_lines_simple(buffers);
    }

    buffers.buffer.clear();
}

/// Scans each line in the buffer and aggregates the trailing numbers
/// to make a single entry, which is written to the record
fn aggregate_lines_simple<'a>(buffers: &'a mut WorkingBuffers) {
    // File contained contents:
    // parse each line and keep track of total
    let mut quantity: LazyQuantity<'a, u64> = LazyQuantity::default();
    let lines = util::ByteLines::new(&buffers.buffer.b);
    for (line, _) in lines {
        // Get the number at the end
        if let Some(space) = util::find_char(line, 0, util::is_space) {
            let number_slice = &line[(space + 1)..];
            quantity = quantity.plus(number_slice);
        }
    }

    quantity.write_to_record(&mut buffers.copy_buffer, &mut buffers.record);
}
