pub mod cgroups;
pub mod docker;
mod errors;
pub mod kubernetes;

use crate::cli::{Mode, Opts};
use crate::shared::TargetMetadata;
use crate::shell::Shell;
use std::marker::Send;
use std::sync::Arc;

// Re-export error types
pub use crate::polling::providers::errors::{FetchError, InitializationError};

/// A target metadata provider
pub trait Provider: Send {
    /// Performs initialization/a connection check to see if the current process
    /// can access the necessary resources to later retrieve lists of
    /// container metadata
    fn initialize(&mut self, opts: &Opts, shell: Arc<Shell>) -> Result<(), InitializationError>;
    /// Attempts to get a list of collection targets, returning a FetchError if
    /// it failed
    fn fetch(&mut self) -> Result<Vec<TargetMetadata>, FetchError>;
}

/// Gets the corresponding provider for the CLI polling mode
pub fn for_mode(mode: Mode) -> Box<dyn Provider> {
    match mode {
        Mode::Docker => docker::Docker::new(),
        Mode::Kubernetes => kubernetes::Kubernetes::new(),
    }
}
