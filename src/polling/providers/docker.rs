use crate::polling::providers::{FetchError, Provider};
use crate::shared::ContainerMetadata;
use crate::util;

use shiplift::rep::Container;
use tokio::runtime::Runtime;

const CONNECTION_ERROR_MESSAGE: &str =
    "Could not connect to the docker socket. Are you running rAdvisor as root?\nIf running at a non-standard URL, set DOCKER_HOST to the correct URL.";

pub struct Docker {
    client: shiplift::Docker,
}

impl Docker {
    pub fn new() -> Box<dyn Provider> {
        Box::new(Docker {
            client: shiplift::Docker::new(),
        })
    }
}

impl Provider for Docker {
    fn can_connect(&self) -> bool {
        let future = self.client.ping();
        let result = Runtime::new().unwrap().block_on(future);
        result.is_ok()
    }

    fn connection_error_message(&self) -> String {
        CONNECTION_ERROR_MESSAGE.to_owned()
    }

    fn fetch(&self) -> Result<Vec<ContainerMetadata>, FetchError> {
        let future = self.client.containers().list(&Default::default());
        match Runtime::new().unwrap().block_on(future) {
            Err(e) => Err(FetchError::new(Some(e.into()))),
            Ok(containers) => Ok(containers
                .into_iter()
                .filter_map(convert_container)
                .collect::<Vec<_>>()),
        }
    }
}

/// Converts a container to a its metadata, or rejects it if it shouldn't
/// be collected for the next polling tick
fn convert_container(c: Container) -> Option<ContainerMetadata> {
    match should_collect_stats(&c) {
        true => Some(format_metadata(c)),
        false => None,
    }
}

/// Whether radvisor should collect statistics for the given container
fn should_collect_stats(_c: &Container) -> bool {
    true
}

/// Formats container info used for the header row and cgroup path
fn format_metadata(c: Container) -> ContainerMetadata {
    ContainerMetadata {
        // Uses debug formatting despite poor performance because this
        // function is invoked relatively infrequently
        info: format!(
            "# ID: {}\n# Names: {:?}\n# Command: {}\n# Image: {}\n# Status: {}\n# Labels: {:?}\n# Ports: {:?}\n# Created: {}\n# Size: {:?}\n# Root FS Size: {:?}\n# Poll time: {}\n",
            c.id, c.names, c.command, c.image, c.status, c.labels, c.ports, c.created, c.size_rw, c.size_root_fs, util::nano_ts()
        ),
        // Container cgroups are under the docker parent, and are in leaf
        // cgroups by (full) container ID
        cgroup: format!("docker/{}", c.id),
        id: c.id
    }
}
