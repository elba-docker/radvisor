use crate::polling::providers::cgroups::CgroupPath;
use crate::shell::Shell;
use std::sync::Arc;
use std::time::Duration;

use bus::BusReader;

/// Target metadata published to the collection thread
pub struct TargetMetadata {
    /// Structure with relative cgroup path (relative to the cgroup root)
    pub cgroup:        CgroupPath,
    /// Yaml-formatted target info
    pub info:          String,
    pub id:            String,
    pub provider_type: &'static str,
}

/// Common context used for the two interval worker threads (collection and
/// polling)
pub struct IntervalWorkerContext {
    pub term_rx:  BusReader<()>,
    pub interval: Duration,
    pub shell:    Arc<Shell>,
}
