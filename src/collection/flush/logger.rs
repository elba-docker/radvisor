use crate::collection::flush::event::{FlushEvent, FlushLog};
use std::io::{Result, Write};
use std::sync::{Arc, Mutex};

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
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
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
    fn flush(&mut self) -> Result<()> { self.writer.flush() }
}
