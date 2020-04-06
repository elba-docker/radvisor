//! Manipulation methods for treating byte arrays (`[u8]`) as UTF-8/ASCII
//! strings

use crate::util::buffer::content_len_raw;

pub static N: u8 = b'\n';
pub static R: u8 = b'\r';
pub static S: u8 = b' ';

/// Returns true if the given char is a line feed, carriage return, or normal
/// space
#[inline]
pub fn is_whitespace(c: u8) -> bool { is_newline(c) || is_space(c) }

/// Returns true if the given char is a normal whitespace
#[inline]
pub fn is_space(c: u8) -> bool { c == S }

/// Returns true if the given char is a line feed or carriage return
#[inline]
pub fn is_newline(c: u8) -> bool { c == N || c == R }

/// Finds the position of the next character, starting at the given index. If a
/// NUL character is reached before the target, the result is an None. Else, if
/// the target is found, the result is a Some including the index of the target
/// char.
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

/// Provides iteration capabilities to a raw byte buffer, iterating on each
/// newline found
pub struct ByteLines<'a> {
    buffer:   &'a [u8],
    position: usize,
    done:     bool,
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

    /// Attempts to find the next line slice, finding the position of the next
    /// newline. If it can't find a next newline, then it returns None
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        match find_char(self.buffer, self.position, is_newline) {
            Some(pos) => {
                let result = Some((&self.buffer[self.position..pos], self.position));
                self.position = pos + 1;
                result
            },
            None => {
                self.done = true;
                // Attempt to return the remaining content on the last line
                let remaining_len = content_len_raw(&self.buffer[self.position..]);
                if remaining_len > 0 {
                    Some((
                        &self.buffer[self.position..(self.position + remaining_len)],
                        self.position,
                    ))
                } else {
                    None
                }
            },
        }
    }
}
