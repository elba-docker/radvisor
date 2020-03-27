use std::time::Duration;
use bus::BusReader;

/// Container metadata published to the collection thread
pub struct ContainerMetadata {
    pub cgroup: String,
    pub info: String,
    pub id: String,
}

/// Common context used for the two interval worker threads (collection and polling)
pub struct IntervalWorkerContext {
    pub term_rx: BusReader<()>,
    pub interval: Duration,
}
