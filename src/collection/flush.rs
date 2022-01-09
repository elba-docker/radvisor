#![allow(clippy::module_name_repetitions)]

use crate::util;
use arraystring::{typenum::U64, ArrayString};
use csv::Writer;
use serde::Serialize;
use std::io::{Result as IoResult, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Stores metadata about a buffer flush event
#[derive(Debug, Serialize)]
pub struct FlushEvent {
    timestamp: u128,
    target_id: ArrayString<U64>,
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
            target_id: ArrayString::from_str_truncate(id),
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

/// Sits between a buffered writer and some destination writer (such as a file),
/// logging when the buffered writer flushes to its destination.
/// This is useful to log when rAdvisor flushes its collection buffers to files,
/// allowing it to note the time of these flushes
pub struct FlushLogger<T: Write> {
    log:    Option<Arc<Mutex<FlushLog>>>,
    id:     String,
    writer: T,
}

impl<T: Write> FlushLogger<T> {
    #[must_use]
    pub fn new(writer: T, id: String, log: Option<Arc<Mutex<FlushLog>>>) -> Self {
        Self { log, id, writer }
    }
}

impl<T: Write> Write for FlushLogger<T> {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        let result = self.writer.write(buf);
        if let Some(log_lock) = &self.log {
            // If logging is enabled, log the flush event
            let event = FlushEvent::new(&result, &self.id);
            let mut log = log_lock.lock().unwrap();
            log.events.push(event);
        }
        result
    }

    // We don't need to track anything for flushes,
    // since a flush to an upstream buffered reader causes a write itself
    fn flush(&mut self) -> IoResult<()> { self.writer.flush() }
}
