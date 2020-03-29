use std::time::Duration;

use bus::BusReader;

/// Target metadata published to the collection thread
pub struct TargetMetadata {
    /// Absolute cgroup path, relative to the cgroup root
    pub cgroup: String,
    /// Yaml-formatted target info
    pub info:   String,
    pub id:     String,
}

/// Common context used for the two interval worker threads (collection and
/// polling)
pub struct IntervalWorkerContext {
    pub term_rx:  BusReader<()>,
    pub interval: Duration,
}
