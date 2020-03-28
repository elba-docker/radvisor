pub mod cgroups;
pub mod docker;
mod errors;
pub mod kubernetes;

use crate::cli::{Mode, Opts};
use crate::shared::ContainerMetadata;
use std::marker::Send;

// Re-export error types
pub use crate::polling::providers::errors::{FetchError, InitializationError};

/// A container metadata provider
pub trait Provider: Send {
    /// Performs initialization/a connection check to see if the current process
    /// can access the necessary resources to later retrieve lists of
    /// container metadata
    fn initialize(&mut self, opts: &Opts) -> Option<InitializationError>;
    /// Attempts to get a list of containers, returning a FetchError if it
    /// failed
    fn fetch(&mut self) -> Result<Vec<ContainerMetadata>, FetchError>;
}

/// Gets the corresponding provider for the CLI polling mode
pub fn for_mode(mode: Mode) -> Box<dyn Provider> {
    match mode {
        Mode::Docker => docker::Docker::new(),
        Mode::Kubernetes => kubernetes::Kubernetes::new(),
    }
}
