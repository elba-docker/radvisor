use crate::util::{AnonymousSlice, Buffer};
use csv::ByteRecord;

/// Length of the buffer for each row. Designed to be a reasonable upper limit
/// to prevent expensive re-allocation
const ROW_BUFFER_SIZE: usize = 1200;

/// Length of the buffer used to read proc files in with. Designed to be an
/// upper limit for the various virtual files that need to be read
const WORKING_BUFFER_SIZE: usize = 16384;

/// Length of the buffer used to build up stat file entries as the reader uses
/// pre-examined layouts to map lines to entries.
///
/// **Currently set to the number of entries used for `memory.stat`**
/// (for the cgroups v1 Collector implementation)
const SLICES_BUFFER_SIZE: usize = 16;

const BASE_FIELD_COUNT: usize = 75;

/// Working buffers used to avoid heap allocations at runtime
#[allow(clippy::module_name_repetitions)]
pub struct WorkingBuffers {
    pub record:      ByteRecord,
    pub buffer:      Buffer<WORKING_BUFFER_SIZE>,
    pub copy_buffer: Buffer<WORKING_BUFFER_SIZE>,
    pub slices:      [AnonymousSlice; SLICES_BUFFER_SIZE],
}

impl Default for WorkingBuffers {
    fn default() -> Self { Self::new() }
}

impl WorkingBuffers {
    /// Allocates the working buffers using upper limits to avoid expensive heap
    /// allocations at runtime
    #[must_use]
    pub fn new() -> Self {
        Self {
            record:      ByteRecord::with_capacity(ROW_BUFFER_SIZE, BASE_FIELD_COUNT),
            slices:      [<AnonymousSlice>::default(); SLICES_BUFFER_SIZE],
            buffer:      Buffer::<WORKING_BUFFER_SIZE>::default(),
            copy_buffer: Buffer::<WORKING_BUFFER_SIZE>::default(),
        }
    }
}
