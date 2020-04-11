#[cfg(feature = "docker")]
pub mod docker;
#[cfg(feature = "kubernetes")]
pub mod kubernetes;

use crate::cli::RunCommand;
use crate::shared::CollectionEvent;
use crate::shell::Shell;
use std::marker::Send;
use std::sync::Arc;
use strum_macros::EnumString;

use clap::Clap;
use failure::{Error, Fail};
use serde::Serialize;

/// An error that occurred during provider initialization/connection check,
/// including a suggestion message printed to stdout
#[derive(Debug, Fail)]
#[fail(display = "{}", suggestion)]
pub struct InitializationError {
    suggestion: String,
}

/// A target metadata provider
pub trait Provider: Send {
    /// Performs initialization/a connection check to see if the current process
    /// can access the necessary resources to later retrieve lists of
    /// targets metadata and generate collection events
    fn initialize(
        &mut self,
        opts: &RunCommand,
        shell: Arc<Shell>,
    ) -> Result<(), InitializationError>;
    /// Attempts to poll the provider for a list of collection events (new/old
    /// targets), returning an Error if it failed
    fn poll(&mut self) -> Result<Vec<CollectionEvent>, Error>;
}

/// Type of provider to use to generate collection events
#[derive(EnumString, Clap, Clone, Copy, Debug, PartialEq, Serialize)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    #[cfg(feature = "docker")]
    #[clap(
        about = "runs collection using docker as the target backend; collecting stats for each \
                 container"
    )]
    Docker,

    #[cfg(feature = "kubernetes")]
    #[clap(
        about = "runs collection using kubernetes as the target backend; collecting stats for \
                 each pod"
    )]
    Kubernetes,
}

impl ProviderType {
    /// Gets the corresponding provider for the CLI polling mode
    #[must_use]
    pub fn into_impl(self) -> Box<dyn Provider> {
        match self {
            #[cfg(feature = "docker")]
            Self::Docker => Box::new(docker::Docker::new()),
            #[cfg(feature = "kubernetes")]
            Self::Kubernetes => Box::new(kubernetes::Kubernetes::new()),
        }
    }
}
