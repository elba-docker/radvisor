pub mod docker;
pub mod kubernetes;
mod errors;

use crate::shared::ContainerMetadata;
use crate::cli::Mode;
use std::marker::Send;

pub use crate::polling::providers::errors::{ConnectionError, FetchError};

/// A container metadata provider
pub trait Provider: Send {
    /// Performs initialization/a connection check to see if the current process can
    /// access the necessary resources to later retrieve lists of container metadata
    fn try_connect(&mut self) -> Option<ConnectionError>;
    /// Attempts to get a list of containers, returning a FetchError if it failed
    fn fetch(&mut self) -> Result<Vec<ContainerMetadata>, FetchError>;
}

/// Gets the corresponding provider for the CLI polling mode
pub fn for_mode(mode: Mode) -> Box<dyn Provider> {
    match mode {
        Mode::Docker => docker::Docker::new(),
        Mode::Kubernetes => kubernetes::Kubernetes::new(),
    }
}
