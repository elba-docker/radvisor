pub mod buffer;

pub static N: u8 = b'\n';
pub static R: u8 = b'\r';
pub static S: u8 = b' ';

/// Gets the nanosecond unix timestamp for a stat read
pub fn nano_ts() -> u128 { imp::nano_ts() }

/// Gets the second unix timestamp for the stat filename
pub fn second_ts() -> u64 { imp::second_ts() }

#[cfg(unix)]
mod imp {
    use libc::{clock_gettime, timespec, CLOCK_REALTIME};
    use std::mem;

    /// Invokes `clock_gettime` from time.h in libc to get a `timespec` struct
    fn get_time() -> timespec {
        let mut tp: timespec = unsafe { mem::zeroed() };
        unsafe {
            clock_gettime(CLOCK_REALTIME, &mut tp);
        }
        tp
    }

    pub fn nano_ts() -> u128 {
        let tp = get_time();
        (tp.tv_nsec as u128) + ((tp.tv_sec as u128) * 1_000_000_000)
    }

    pub fn second_ts() -> u64 { get_time().tv_sec as u64 }
}

#[cfg(windows)]
mod imp {
    use std::mem;
    use winapi::shared::minwindef::{FILETIME, SYSTEMTIME, WORD};
    use winapi::um::sysinfoapi;

    /// Number of seconds between the start of the Windows epoch (Jan 1. 1601)
    /// and the start of the Unix epoch (Jan 1. 1970)
    const EPOCH_DIFFERENCE: u64 = 11644473600;
    /// Number of nanoseconds between the start of the Windows epoch (Jan 1.
    /// 1601) and the start of the Unix epoch (Jan 1. 1970)
    const NANO_EPOCH_DIFFERENCE: u128 = (EPOCH_DIFFERENCE as u128) * 1_000_000_000;
    /// Number of nanoseconds per tick
    const TICK_LENGTH: u128 = 100;
    /// Number of ticks per second
    const TICK: i64 = 1_000_000_000 / (TICK_LENGTH as i64);

    /// Executes a win32 call, returning a timestamp that represents the number
    /// of 100 ns intervals since January 1, 1601 (UTC). Invokes
    /// `GetSystemTimePreciseAsFileTime` from Sysinfoapi.h in
    /// [winapi](https://docs.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getsystemtimepreciseasfiletime)
    fn file_timestamp() -> i64 {
        let mut file_time: FILETIME = unsafe { mem::zeroed() };
        unsafe {
            sysinfoapi::GetSystemTimePreciseAsFileTime(&mut file_time);
        }

        (file_time.dwLowDateTime as i64) + (file_time.dw_HighDateTime as i64) << 32;
    }

    pub fn nano_ts() -> u128 { (file_timestamp() as u128) * TICK_LENGTH + NANO_EPOCH_DIFFERENCE }

    pub fn second_ts() -> u64 { (file_timestamp() / TICK) as u64 + EPOCH_DIFFERENCE }
}

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

/// Represents an anonymous slice, lacking any memory ownership semantics
#[derive(Default, Clone, Copy)]
pub struct AnonymousSlice {
    pub start:  usize,
    pub length: usize,
}

impl AnonymousSlice {
    /// Consumes a slice structure to narrow a larger slice to the specific
    /// slice the structure represents
    pub fn consume<'a, 'b: 'a, T>(&self, slice: &'b [T]) -> Option<&'a [T]> {
        let end = self.start + self.length;
        if end >= slice.len() {
            None
        } else {
            Some(&slice[self.start..end])
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
                let remaining_len = buffer::content_len_raw(&self.buffer[self.position..]);
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
