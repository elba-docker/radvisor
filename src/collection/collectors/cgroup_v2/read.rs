use crate::collection::buffers::WorkingBuffers;
use crate::util::{self, BufferLike, ByteLines, LazyQuantity};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

#[derive(Copy, Clone, PartialEq)]
pub struct Empty;

/// Tries to read the given file handle,
/// and directly write the contents as a field to the next record.
/// If the written field was empty, returns Err(Empty).
pub fn single_value_file(
    file: &Option<File>,
    buffers: &mut WorkingBuffers,
    default: &'static [u8],
) -> Result<(), Empty> {
    let content = match read_to_buffer(file, buffers) {
        None => &[],
        Some(_) => buffers.buffer.trim(),
    };

    let is_empty = util::content_len_raw(content) == 0;
    if is_empty {
        buffers.record.push_field(default);
    } else {
        buffers.record.push_field(content);
    }

    buffers.buffer.clear();

    if is_empty {
        Err(Empty)
    } else {
        Ok(())
    }
}

/// Attempts to read the given file into the buffer, if it exists.
/// If successful, returns Some with the length of the part of the file read.
/// If the file handle wasn't given, or reading was unsuccessful, returns None.
fn read_to_buffer(file: &Option<File>, buffers: &mut WorkingBuffers) -> Option<usize> {
    match file {
        None => None,
        Some(f) => {
            let mut file_mut = f;
            let result = match file_mut.read(&mut buffers.buffer.b) {
                Err(_) => None,
                Ok(len) => {
                    buffers.buffer.len = len;
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
            let _result = file_mut.seek(SeekFrom::Start(0));
            result
        },
    }
}

/// Tries to read the given file handle,
/// attempting to find the given keys in the file's contents.
/// The keys' values are written to the row buffer
/// in the same order as the keys slice,
/// and if a value does not exist, the cell is empty.
/// If all of the written values were empty,
/// then Err(Empty) is returned.
pub fn flat_keyed_file<const K: usize>(
    file: &Option<File>,
    buffers: &mut WorkingBuffers,
    keys: &[&'static [u8]; K],
    defaults: &[&'static [u8]; K],
) -> Result<(), Empty> {
    // Ignore errors: the buffer will just remain empty
    // and all of the below processing will result in empty fields.
    // It is important to always write K fields,
    // so we don't return early.
    let _result = read_to_buffer(file, buffers);

    // Create K slices,
    // each pointing to a location in the buffer
    // where the statistic was found.
    // After scanning each line,
    // the slices will be consumed to add records.
    let mut slices: [&[u8]; K] = [&[]; K];

    let lines = ByteLines::new(&buffers.buffer.b);
    for (line, _) in lines {
        // Split the the buffer by the space in the middle
        // to obtain the key and value:
        if let Some(space) = util::find_char(line, 0, util::is_space) {
            let (key, value) = (&line[..space], &line[(space + 1)..]);
            for (i, &target_key) in keys.iter().enumerate() {
                if target_key == key {
                    slices[i] = value;
                    break;
                }
            }
        }
    }

    // Consume each of the slices
    let mut all_empty = true;
    for (i, slice) in slices.iter().enumerate() {
        all_empty = all_empty && slice.is_empty();
        if slice.is_empty() {
            buffers.record.push_field(defaults[i]);
        } else {
            buffers.record.push_field(slice);
        }
    }

    buffers.buffer.clear();

    if all_empty {
        Err(Empty)
    } else {
        Ok(())
    }
}

/// Tries to read the given file handle,
/// attempting to read it in as an IO stats file.
/// In this mode, it will search for the given keys
/// similar to `read::flat_keyed_file`,
/// but it will also sum multiple values for the same key
/// that occur in the IO stats file-specific format,
/// where each line gives stats for a single device.
/// In this way, the written values of this function
/// give the total IO stats over all devices.
/// If all of the written values were 0,
/// then Err(Empty) is returned.
pub fn io_stat_file<const K: usize>(
    file: &Option<File>,
    buffers: &mut WorkingBuffers,
    keys: &[&'static [u8]; K],
) -> Result<(), Empty> {
    // Ignore errors: the buffer will just remain empty
    // and all of the below processing will result in empty fields.
    // It is important to always write K fields,
    // so we don't return early.
    let _result = read_to_buffer(file, buffers);

    // Create K lazy quantities,
    // where each corresponds to the nth key.
    // As we scan each line in the stat file,
    // we look for values for each key,
    // and when found, add the value to the lazy quantity.
    // This prevents us from parsing the bytes as an integer
    // unless we need to add two values together.
    let mut quantities = [LazyQuantity::<'_, u64>::default(); K];

    let lines = ByteLines::new(&buffers.buffer.b);
    for (line, _) in lines {
        let fields = IoLineFieldIter::new(line);
        for (key, value) in fields {
            for (i, &target_key) in keys.iter().enumerate() {
                if target_key == key {
                    quantities[i] = quantities[i].plus(value);
                    break;
                }
            }
        }
    }

    // Consume each of the quantities
    let mut all_zero = true;
    for qty in quantities {
        all_zero = all_zero && qty.is_zero();
        qty.write_to_record(&mut buffers.copy_buffer, &mut buffers.record);
    }

    buffers.buffer.clear();
    buffers.copy_buffer.clear();

    if all_zero {
        Err(Empty)
    } else {
        Ok(())
    }
}

pub struct IoLineFieldIter<'a> {
    remainder: &'a [u8],
}

impl<'a> IoLineFieldIter<'a> {
    fn new(line: &'a [u8]) -> Self {
        // The first space separates the device from the fields
        match util::find_char(line, 0, util::is_space) {
            Some(space) => {
                let (_dev, fields) = (&line[..space], &line[(space + 1)..]);
                Self { remainder: fields }
            },
            // Return an empty iterator
            None => Self {
                remainder: &line[0..0],
            },
        }
    }
}

impl<'a> Iterator for IoLineFieldIter<'a> {
    type Item = (&'a [u8], &'a [u8]);
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.remainder.is_empty() {
                return None;
            }

            match util::find_char(self.remainder, 0, util::is_space) {
                None => return None,
                Some(space) => {
                    let (pair, rest) = (&self.remainder[..space], &self.remainder[(space + 1)..]);
                    self.remainder = rest;

                    // Try to parse the key=value pair
                    if let Some(equals) = util::find_char(pair, 0, |c| c == b'=') {
                        let (key, value) = (&pair[..equals], &pair[(equals + 1)..]);
                        return Some((key, value));
                    }

                    // If the current pair was not valid; continue to next pair
                },
            }
        }
    }
}
