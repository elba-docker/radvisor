use crate::cli::Opts;
use crate::polling::providers::cgroups::{self, CgroupManager};
use crate::polling::providers::{FetchError, InitializationError, Provider};
use crate::shared::ContainerMetadata;
use crate::util;

use shiplift::rep::Container;
use tokio_compat::runtime::Runtime;

const CONNECTION_ERROR_MESSAGE: &str = "Could not connect to the docker socket. Are you running \
                                        rAdvisor as root?\nIf running at a non-standard URL, set \
                                        DOCKER_HOST to the correct URL.";

pub struct Docker {
    client:         shiplift::Docker,
    runtime:        Runtime,
    cgroup_manager: CgroupManager,
}

impl Docker {
    pub fn new() -> Box<dyn Provider> {
        Box::new(Docker {
            client:         shiplift::Docker::new(),
            runtime:        Runtime::new().unwrap(),
            cgroup_manager: CgroupManager::new(),
        })
    }

    /// Converts a container to a its metadata, or rejects it if it shouldn't
    /// be collected for the next polling tick
    fn convert_container(&mut self, c: Container) -> Option<ContainerMetadata> {
        match should_collect_stats(&c) {
            true => match try_format_metadata(&c, &mut self.cgroup_manager) {
                Some(metadata) => Some(metadata),
                None => {
                    eprintln!(
                        "Could not create container metadata for container {}:\ncgroup path could \
                         not be constructed or does not exist",
                        c.names.iter().nth(0).unwrap_or(&c.id)
                    );
                    None
                },
            },
            false => None,
        }
    }
}

impl Provider for Docker {
    fn initialize(&mut self, _: &Opts) -> Option<InitializationError> {
        println!("Initializing Docker API provider");

        // Ping the Docker API to make sure the current process can connect
        let future = self.client.ping();
        let result = self.runtime.block_on(future);
        match result {
            Ok(_) => {},
            Err(_) => return Some(InitializationError::new(CONNECTION_ERROR_MESSAGE)),
        }

        // Make sure cgroups are mounted properly
        if !cgroups::cgroups_mounted_properly() {
            return Some(InitializationError::new(cgroups::INVALID_MOUNT_MESSAGE));
        }

        None
    }

    fn fetch(&mut self) -> Result<Vec<ContainerMetadata>, FetchError> {
        let future = self.client.containers().list(&Default::default());
        match self.runtime.block_on(future) {
            Err(e) => Err(FetchError::new(Some(e.into()))),
            Ok(containers) => Ok(containers
                .into_iter()
                .filter_map(|c| self.convert_container(c))
                .collect::<Vec<_>>()),
        }
    }
}

/// Whether radvisor should collect statistics for the given container
fn should_collect_stats(_c: &Container) -> bool { true }

/// Formats container info used for the header row and cgroup path. Can fail
/// if the cgroup doesn't exist.
fn try_format_metadata(
    c: &Container,
    cgroup_manager: &mut CgroupManager,
) -> Option<ContainerMetadata> {
    // Container cgroups are under the dockerd parent, and are in leaf
    // cgroups by (full) container ID. Cgroup path depends on the driver used:
    // according to https://docs.docker.com/engine/reference/commandline/dockerd/#default-cgroup-parent ,
    // "[container cgroups are mounted at] `/docker` for fs cgroup driver and
    // `system.slice` for systemd cgroup driver." The .slice is omitted
    match cgroup_manager.make_cgroup_divided(&["system", &c.id], &["docker", &c.id]) {
        None => None,
        Some(cgroup) => Some(ContainerMetadata {
            cgroup,
            id: c.id.clone(),
            // Uses debug formatting despite poor performance because this
            // function is invoked relatively infrequently
            info: format!(
                "# ID: {}\n# Names: {:?}\n# Command: {}\n# Image: {}\n# Status: {}\n# Labels: \
                 {:?}\n# Ports: {:?}\n# Created: {}\n# Size: {:?}\n# Root FS Size: {:?}\n# Poll \
                 time: {}\n",
                c.id,
                c.names,
                c.command,
                c.image,
                c.status,
                c.labels,
                c.ports,
                c.created,
                c.size_rw,
                c.size_root_fs,
                util::nano_ts()
            ),
        }),
    }
}
