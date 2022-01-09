use crate::shell::Shell;
use crate::util::CgroupPath;
use bus::BusReader;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;

/// Common context used for the two interval worker threads (collection and
/// polling)
pub struct IntervalWorkerContext {
    pub interval: Duration,
    pub term_rx:  BusReader<()>,
    pub shell:    Arc<Shell>,
}

/// Unique target ID
pub type Id = String;

/// Type of event pushed to common channel between the polling and collection
/// threads, used to update the state of the collection
#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum CollectionEvent {
    Stop(Id),
    Start {
        target: CollectionTarget,
        method: CollectionMethod,
    },
}

/// Type of collection used; corresponds to a resultant CSV schema
#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum CollectionMethod {
    LinuxCgroupsV1(CgroupPath),
}

/// Single container/pod/process/other entity that represents a single target
/// with which to run statistic collection against
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct CollectionTarget {
    /// Provider type that generated the collection target
    pub provider:  &'static str,
    /// Unique ID
    pub id:        Id,
    /// Human-readable entity string
    pub name:      String,
    /// Optional partially serialized metadata
    pub metadata:  Option<serde_yaml::Value>,
    /// Time of polling
    pub poll_time: u128,
}
