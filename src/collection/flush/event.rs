use crate::util::{self, Buffer};
use std::io::Result as IoResult;
use std::path::{Path, PathBuf};

use csv::Writer;
use serde::Serialize;

// Max length of a target ID
// (Docker container or Kubernetes pod)
const TARGET_ID_BUFFER_LENGTH: usize = 64;

/// Stores metadata about a buffer flush event
#[derive(Debug, Serialize)]
pub struct FlushEvent {
    timestamp: u128,
    target_id: Buffer<TARGET_ID_BUFFER_LENGTH>,
    written:   usize,
    success:   bool,
}

impl FlushEvent {
    /// Consumes an IO result to make a new `FlushEvent`,
    /// taking the current timestamp at the time of invocation
    /// and copying the given ID into the buffer
    #[must_use]
    pub fn new<A: AsRef<str>>(result: &IoResult<usize>, id: A) -> Self {
        let success = result.is_ok();
        let written = *result.as_ref().unwrap_or(&0);
        Self {
            timestamp: util::nano_ts(),
            target_id: Buffer::from_str_truncate(id),
            written,
            success,
        }
    }
}

/// Stores a vec of flush events
pub struct FlushLog {
    pub path:   PathBuf,
    pub events: Vec<FlushEvent>,
}

impl FlushLog {
    #[must_use]
    pub fn new<A: AsRef<Path>>(event_log_path: A, capacity: usize) -> Self {
        Self {
            path:   event_log_path.as_ref().to_path_buf(),
            events: Vec::with_capacity(capacity),
        }
    }

    /// Consumes the inner events, writing them to the file.
    /// This should be called upon teardown of rAdvisor
    pub fn write(&mut self) -> IoResult<usize> {
        let mut writer = Writer::from_path(&self.path)?;
        let event_count = self.events.len();
        for event in self.events.drain(..) {
            writer.serialize(event)?;
        }
        writer.flush()?;
        Ok(event_count)
    }
}
