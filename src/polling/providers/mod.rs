#[cfg(feature = "docker")]
pub mod docker;
#[cfg(feature = "kubernetes")]
pub mod kubernetes;

use crate::cli::RunCommand;
use crate::shared::CollectionEvent;
use crate::shell::Shell;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Clap;
use failure::{Error, Fail};
use serde::Serialize;

/// An error that occurred during provider initialization/connection check,
/// including a suggestion message printed to stdout
#[derive(Debug, Fail)]
#[fail(display = "{}", suggestion)]
pub struct InitializationError {
    pub suggestion: String,
    pub original:   Option<Error>,
}

/// A target metadata provider
pub trait Provider {
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

pub use provider_type::ProviderType;
mod provider_type {
    // There seems to be a bug around EnumString macro expansion
    // that causes clippy to complain, so we include ProviderType in its own
    // private module
    #![allow(clippy::default_trait_access)]

    use super::KubernetesOptions;
    use crate::cli::{AUTHORS, VERSION};
    use clap::Clap;
    use serde::{Serialize, Serializer};
    use strum_macros::IntoStaticStr;

    #[derive(IntoStaticStr, Clap, Clone, Debug, PartialEq)]
    #[strum(serialize_all = "snake_case")]
    pub enum ProviderType {
        #[cfg(feature = "docker")]
        #[cfg(feature = "kubernetes")]
        #[clap(
            version = VERSION.unwrap_or("unknown"),
            author = AUTHORS.as_deref().unwrap_or("contributors"),
            about = "Runs collection using docker as the target backend; collecting stats for \
            each container"
        )]
        Docker,

        #[cfg(feature = "kubernetes")]
        #[clap(
            version = VERSION.unwrap_or("unknown"),
            author = AUTHORS.as_deref().unwrap_or("contributors"),
            about = "Runs collection using kubernetes as the target backend; collecting stats for \
            each pod"
        )]
        Kubernetes(KubernetesOptions),
    }

    // Implement custom serialize so that inner structs are left off
    impl Serialize for ProviderType {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_str(self.into())
        }
    }
}

/// Type of provider to use to generate collection events
#[allow(clippy::default_trait_access)]
#[cfg(feature = "kubernetes")]
#[derive(Default, Clap, Clone, Debug, PartialEq, Serialize)]
pub struct KubernetesOptions {
    /// Location of kubernetes config file (used to connect to the cluster)
    #[clap(parse(from_os_str), short = 'k', long = "kube-config")]
    pub kube_config: Option<PathBuf>,
}

impl ProviderType {
    /// Gets the corresponding provider for the CLI polling mode
    #[must_use]
    pub fn into_impl(self) -> Box<dyn Provider> {
        match self {
            #[cfg(feature = "docker")]
            Self::Docker => Box::new(docker::Docker::new()),
            #[cfg(feature = "kubernetes")]
            Self::Kubernetes(_) => Box::new(kubernetes::Kubernetes::new()),
        }
    }
}
