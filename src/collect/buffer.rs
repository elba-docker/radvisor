use crate::util;

/// Length of the buffer used to read proc files in with. Designed to be an upper
/// limit for the various virtual files that need to be read
const WORKING_BUFFER_SIZE: usize = 768;

/// Working buffer used to read proc files in. Can operate both in **managed**
/// mode (where it keeps track of length) and **unmanaged** mode (where it acts)
/// as a plain byte buffer.
pub struct Buffer {
    pub len: usize,
    pub b: [u8; WORKING_BUFFER_SIZE],
}

pub trait BufferLike {
    /// **(Managed)** Clears a buffer by setting each element to 0 until it reaches
    /// the end of the content
    fn clear(&mut self) -> ();
    /// **(Unmanaged)** Clears a buffer by setting each element to 0 until it reaches
    /// a 0 value, starting from the start of the buffer
    fn clear_unmanaged(&mut self) -> ();
    /// **(Unmanaged)** clears a buffer by setting each element to 0 until it reaches
    /// a 0 value, starting from the end of the buffer
    fn clear_unmanaged_backwards(&mut self) -> ();
    /// **(Managed)** returns a slice without a trailing newline
    fn trim(&self) -> &[u8];
    /// **(Unmanaged)** Finds the length of the buffer's contents, ended by a 0
    /// terminator
    fn unmanaged_len(&self) -> usize;
    fn new() -> Self;
}

impl BufferLike for Buffer {
    fn new() -> Self {
        Buffer {
            len: 0,
            b: [0u8; WORKING_BUFFER_SIZE],
        }
    }

    #[inline]
    fn trim(&self) -> &[u8] {
        let mut start = 0;
        while start < self.len && util::is_space(self.b[start]) {
            start += 1;
        }

        let mut end = self.len - 1;
        while end > start && util::is_space(self.b[end]) {
            end -= 1;
        }

        &self.b[start..=end]
    }

    #[inline]
    fn clear(&mut self) -> () {
        for i in 0..self.len {
            self.b[i] = 0;
        }
        self.len = 0;
    }

    #[inline]
    fn clear_unmanaged(&mut self) -> () {
        for i in 0..self.b.len() {
            if self.b[i] == 0 {
                break;
            } else {
                self.b[i] = 0;
            }
        }
        self.len = 0;
    }

    #[inline]
    fn clear_unmanaged_backwards(&mut self) -> () {
        for i in (0..self.b.len()).rev() {
            if self.b[i] == 0 {
                break;
            } else {
                self.b[i] = 0;
            }
        }
        self.len = 0;
    }

    #[inline]
    fn unmanaged_len(&self) -> usize {
        content_len_raw(&self.b)
    }
}

/// Determines the length of the non-zero content in a raw buffer slice
#[inline]
pub fn content_len_raw(buf: &[u8]) -> usize {
    let mut len: usize = 0;
    for i in 0..buf.len() {
        if buf[i] == 0 {
            break;
        } else {
            len += 1;
        }
    }
    len
}

/// Trims a raw buffer slice to not include any starting or ending whitespace
#[inline]
pub fn trim_raw(buf: &[u8]) -> &[u8] {
    let len: usize = content_len_raw(buf);

    let mut start = 0;
    while start < len && util::is_space(buf[start]) {
        start += 1;
    }

    let mut end = len - 1;
    while end > start && util::is_space(buf[end]) {
        end -= 1;
    }

    &buf[start..=end]
}
