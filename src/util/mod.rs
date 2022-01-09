// Items in the util crate are imported at the root level, so repetition of the
// module names isn't seen by users outside the crate (and is important for
// context)
#![allow(clippy::module_name_repetitions)]

//! Contains utility methods for processing various data structures, such as
//! bytes, buffers, or system-specific calls

pub(self) mod buffer;
pub(self) mod byte;
pub(self) mod cgroup;
pub(self) mod lazy_quantity;
pub(self) mod pool;
pub(self) mod system;

pub use buffer::*;
pub use byte::*;
pub use cgroup::*;
pub use lazy_quantity::*;
pub use pool::*;
pub use system::*;

/// Represents an anonymous slice, lacking any memory ownership semantics
#[derive(Default, Clone, Copy)]
pub struct AnonymousSlice {
    pub start:  usize,
    pub length: usize,
}

impl AnonymousSlice {
    /// Consumes a slice structure to narrow a larger slice to the specific
    /// slice the structure represents
    #[must_use]
    pub fn consume<'a, 'b: 'a, T>(&self, slice: &'b [T]) -> Option<&'a [T]> {
        let end = self.start + self.length;
        if end >= slice.len() {
            None
        } else {
            Some(&slice[self.start..end])
        }
    }
}
