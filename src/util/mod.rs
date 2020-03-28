pub mod buffer;

use libc::{clock_gettime, timespec, CLOCK_REALTIME};

pub static N: u8 = '\n' as u8;
pub static R: u8 = '\r' as u8;
pub static S: u8 = ' ' as u8;

/// Returns true if the given char is a line feed, carriage return, or normal
/// space
#[inline]
pub fn is_whitespace(c: u8) -> bool {
    is_newline(c) || is_space(c)
}

/// Returns true if the given char is a normal whitespace
#[inline]
pub fn is_space(c: u8) -> bool {
    c == S
}

/// Returns true if the given char is a line feed or carriage return
#[inline]
pub fn is_newline(c: u8) -> bool {
    c == N || c == R
}

/// Gets the nanosecond unix timestamp for a stat read
pub fn nano_ts() -> u128 {
    let mut tp: timespec = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // Invoke clock_gettime from time.h in libc
    unsafe {
        clock_gettime(CLOCK_REALTIME, &mut tp);
    }
    (tp.tv_nsec as u128) + ((tp.tv_sec as u128) * 1000000000)
}

/// Gets the second unix timestamp for the stat filename
pub fn second_ts() -> u64 {
    let mut tp: timespec = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // Invoke clock_gettime from time.h in libc
    unsafe {
        clock_gettime(CLOCK_REALTIME, &mut tp);
    }
    tp.tv_sec as u64
}

/// Finds the position of the next character, starting at the given index. If a NUL character
/// is reached before the target, the result is an None. Else, if the target is found, the result
/// is a Some including the index of the target char.
pub fn find_char<P>(buffer: &[u8], start: usize, is_target: P) -> Option<usize>
where
    P: Fn(u8) -> bool,
{
    let mut newline = start;
    loop {
        if newline >= buffer.len() {
            // unexpected end of memory
            return None;
        }

        let char_at = buffer[newline];
        if is_target(char_at) {
            return Some(newline);
        } else if char_at == 0 {
            // unexpected end of string
            return None;
        } else {
            newline += 1;
        }
    }
}

/// Represents an anonymous slice, lacking any memory ownership semantics
#[derive(Default, Clone, Copy)]
pub struct AnonymousSlice {
    pub start: usize,
    pub length: usize,
}

impl AnonymousSlice {
    /// Consumes a slice structure to narrow a larger slice to the specific slice the structure
    /// represents
    pub fn consume<'a, 'b: 'a, T>(&self, slice: &'b [T]) -> Option<&'a [T]> {
        let end = self.start + self.length;
        if end >= slice.len() {
            None
        } else {
            Some(&slice[self.start..end])
        }
    }
}

/// Provides iteration capabilities to a raw byte buffer, iterating on each newline found
pub struct ByteLines<'a> {
    buffer: &'a [u8],
    position: usize,
    done: bool,
}

impl<'a> ByteLines<'a> {
    /// Instantiates a new iterator with the given source byte buffer
    pub fn new(buffer: &'a [u8]) -> Self {
        ByteLines {
            buffer,
            position: 0,
            done: false,
        }
    }
}

impl<'a> Iterator for ByteLines<'a> {
    type Item = (&'a [u8], usize);

    /// Attempts to find the next line slice, finding the position of the next newline. If it
    /// can't find a next newline, then it returns None
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        match find_char(self.buffer, self.position, is_newline) {
            Some(pos) => {
                let result = Some((&self.buffer[self.position..pos], self.position));
                self.position = pos + 1;
                result
            }
            None => {
                self.done = true;
                // Attempt to return the remaining content on the last line
                let remaining_len = buffer::content_len_raw(&self.buffer[self.position..]);
                if remaining_len > 0 {
                    Some((
                        &self.buffer[self.position..(self.position + remaining_len)],
                        self.position,
                    ))
                } else {
                    None
                }
            }
        }
    }
}
