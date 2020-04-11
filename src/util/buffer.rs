//! Contains `Buffer`, a statically-sized working buffer that can be used to
//! additionally include length when in managed mode.

use crate::util::byte::is_whitespace;

/// Working buffer of raw bytes. Can operate both in **managed** mode (where it
/// keeps track of length) and **unmanaged** mode (where it acts) as a plain
/// byte buffer.
pub struct Buffer<const SIZE: usize> {
    pub len: usize,
    pub b:   [u8; SIZE],
}

pub trait BufferLike {
    /// **(Managed)** Clears a buffer by setting each element to 0 until it
    /// reaches the end of the content
    fn clear(&mut self);
    /// **(Unmanaged)** Clears a buffer by setting each element to 0 until it
    /// reaches a 0 value, starting from the start of the buffer
    fn clear_unmanaged(&mut self);
    /// **(Unmanaged)** clears a buffer by setting each element to 0 until it
    /// reaches a 0 value, starting from the end of the buffer
    fn clear_unmanaged_backwards(&mut self);
    /// **(Managed)** returns a slice without a trailing newline
    fn trim(&self) -> &[u8];
    /// **(Unmanaged)** Finds the length of the buffer's contents, ended by a 0
    /// terminator
    fn unmanaged_len(&self) -> usize;
    /// **(Managed)** Gets a sub-slice of the buffer that only includes
    /// non-NUL characters
    fn content(&self) -> &[u8];
    /// **(Unmanaged)** Gets a sub-slice of the buffer that only includes
    /// non-NUL characters
    fn content_unmanaged(&self) -> &[u8];
}

impl<const SIZE: usize> BufferLike for Buffer<SIZE> {
    #[inline]
    #[must_use]
    fn trim(&self) -> &[u8] {
        // Prevent underflow later by early terminating
        if self.len == 0 {
            return &self.b[0..0];
        }

        let mut start = 0;
        while start < self.len && is_whitespace(self.b[start]) {
            start += 1;
        }

        let mut end = self.len - 1;
        while end > start && is_whitespace(self.b[end]) {
            end -= 1;
        }

        &self.b[start..=end]
    }

    #[inline]
    #[must_use]
    fn content_unmanaged(&self) -> &[u8] {
        let mut end = 0;
        while end < self.b.len() && self.b[end] != 0_u8 {
            end += 1;
        }
        &self.b[0..end]
    }

    #[inline]
    fn content(&self) -> &[u8] {
        &self.b[0..self.len]
    }

    #[inline]
    fn clear(&mut self) {
        for i in 0..self.len {
            self.b[i] = 0_u8;
        }
        self.len = 0;
    }

    #[inline]
    fn clear_unmanaged(&mut self) {
        for i in 0..SIZE {
            if self.b[i] == 0_u8 {
                break;
            } else {
                self.b[i] = 0_u8;
            }
        }
        self.len = 0;
    }

    #[inline]
    fn clear_unmanaged_backwards(&mut self) {
        for i in (0..SIZE).rev() {
            if self.b[i] == 0_u8 {
                break;
            } else {
                self.b[i] = 0_u8;
            }
        }
        self.len = 0;
    }

    #[inline]
    #[must_use]
    fn unmanaged_len(&self) -> usize { content_len_raw(&self.b) }
}

/// Determines the length of the non-zero content in a raw buffer slice
#[inline]
#[must_use]
pub fn content_len_raw(buf: &[u8]) -> usize {
    let mut len: usize = 0;
    for byte in buf {
        if *byte == 0_u8 {
            break;
        } else {
            len += 1;
        }
    }
    len
}

/// Trims a raw buffer slice to not include any starting or ending whitespace
#[inline]
#[must_use]
pub fn trim_raw(buf: &[u8]) -> &[u8] {
    let len: usize = content_len_raw(buf);
    // Prevent underflow later by early terminating
    if len == 0 {
        return &buf[0..=0];
    }

    let mut start = 0;
    while start < len && is_whitespace(buf[start]) {
        start += 1;
    }

    let mut end = len - 1;
    while end > start && is_whitespace(buf[end]) {
        end -= 1;
    }

    &buf[start..=end]
}
