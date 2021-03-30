#[cfg(feature = "docker")]
pub mod docker;
#[cfg(feature = "kubernetes")]
pub mod kubernetes;

use crate::cli::{CollectionOptions, PollingOptions, RunCommand};
use crate::shared::CollectionEvent;
use crate::shell::Shell;
#[cfg(feature = "kubernetes")]
use std::path::PathBuf;
use std::sync::Arc;

use clap::Clap;
#[cfg(feature = "kubernetes")]
use clap::ValueHint;
use failure::{Error, Fail};
use serde::{Serialize, Serializer};

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

    #[cfg(feature = "docker")]
    use super::DockerOptions;
    #[cfg(feature = "kubernetes")]
    use super::KubernetesOptions;
    use crate::cli::{AUTHORS, VERSION};
    use clap::Clap;
    use strum_macros::IntoStaticStr;

    /// Type of provider to use to generate collection events
    #[derive(IntoStaticStr, Clap, Clone, Debug, PartialEq)]
    #[strum(serialize_all = "snake_case")]
    pub enum ProviderType {
        #[cfg(feature = "docker")]
        #[clap(
            version = VERSION.unwrap_or("unknown"),
            author = AUTHORS.as_deref().unwrap_or("contributors"),
            about = "Runs collection using docker as the target backend; collecting stats for \
            each container"
        )]
        Docker(DockerOptions),

        #[cfg(feature = "kubernetes")]
        #[clap(
            version = VERSION.unwrap_or("unknown"),
            author = AUTHORS.as_deref().unwrap_or("contributors"),
            about = "Runs collection using kubernetes as the target backend; collecting stats for \
            each pod"
        )]
        Kubernetes(KubernetesOptions),
    }
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

impl ProviderType {
    /// Gets the inner options struct for Kubernetes
    #[must_use]
    #[cfg(feature = "kubernetes")]
    pub fn into_inner_kubernetes(self) -> KubernetesOptions {
        match self {
            #[cfg(feature = "docker")]
            Self::Docker(_) => panic!("Cannot unwrap Docker provider to Kubernetes options"),
            Self::Kubernetes(opts) => opts,
        }
    }
    /// Gets the inner options struct for Docker
    #[must_use]
    #[cfg(feature = "docker")]
    pub fn into_inner_docker(self) -> DockerOptions {
        match self {
            Self::Docker(opts) => opts,
            #[cfg(feature = "kubernetes")]
            Self::Kubernetes(_) => panic!("Cannot unwrap Kubernetes provider to Docker options"),
        }
    }

    /// Gets the corresponding provider for the CLI polling mode
    #[must_use]
    pub fn get_impl(&self) -> Box<dyn Provider> {
        match self {
            #[cfg(feature = "docker")]
            Self::Docker(_) => Box::new(docker::Docker::new()),
            #[cfg(feature = "kubernetes")]
            Self::Kubernetes(_) => Box::new(kubernetes::Kubernetes::new()),
        }
    }

    /// Gets the collection options
    #[must_use]
    pub const fn collection(&self) -> &CollectionOptions {
        match self {
            #[cfg(feature = "docker")]
            Self::Docker(opts) => &opts.collection,
            #[cfg(feature = "kubernetes")]
            Self::Kubernetes(opts) => &opts.collection,
        }
    }

    /// Gets the polling options
    #[must_use]
    pub const fn polling(&self) -> &PollingOptions {
        match self {
            #[cfg(feature = "docker")]
            Self::Docker(opts) => &opts.polling,
            #[cfg(feature = "kubernetes")]
            Self::Kubernetes(opts) => &opts.polling,
        }
    }
}

// Note that both DockerOptions and KubernetesOptions include duplicate flags.
// This is needed due to a bug in Clap https://github.com/clap-rs/clap/issues/2053
#[cfg(feature = "docker")]
#[derive(Clap, Clone, Debug, PartialEq)]
pub struct DockerOptions {
    // Polling-related options
    #[clap(flatten)]
    pub polling: PollingOptions,

    // Collection-related options
    #[clap(flatten)]
    pub collection: CollectionOptions,
}

#[cfg(feature = "kubernetes")]
#[derive(Clap, Clone, Debug, PartialEq)]
pub struct KubernetesOptions {
    /// Location of kubernetes config file (used to connect to the cluster)
    #[clap(
        parse(from_os_str),
        short = 'k',
        long = "kube-config",
        value_hint = ValueHint::FilePath
    )]
    pub kube_config: Option<PathBuf>,

    // Polling-related options
    #[clap(flatten)]
    pub polling: PollingOptions,

    // Collection-related options
    #[clap(flatten)]
    pub collection: CollectionOptions,
}
