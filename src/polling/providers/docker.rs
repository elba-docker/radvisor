use crate::cli::Opts;
use crate::polling::providers::cgroups::{self, CgroupManager, CgroupPath};
use crate::polling::providers::{FetchError, InitializationError, Provider};
use crate::shared::TargetMetadata;
use crate::shell::Shell;
use crate::util;
use std::cell::RefCell;
use std::fmt::Write;
use std::sync::Arc;

use lru_time_cache::LruCache;
use serde_yaml;
use shiplift::rep::Container;
use tokio_compat::runtime::Runtime;

const CONNECTION_ERROR_MESSAGE: &str = "Could not connect to the docker socket. Are you running \
                                        rAdvisor as root?\nIf running at a non-standard URL, set \
                                        DOCKER_HOST to the correct URL.";

const PROVIDER_TYPE: &str = "docker";

/// Number of polling blocks that need to elapse before container info strings
/// are evicted from the time-based LRU cache
const POLLING_BLOCK_EXPIRY: u32 = 5u32;

pub struct Docker {
    client:         shiplift::Docker,
    runtime:        Runtime,
    cgroup_manager: CgroupManager,
    info_cache:     Option<RefCell<LruCache<String, String>>>,
    shell:          Option<Arc<Shell>>,
}

impl Provider for Docker {
    fn initialize(&mut self, opts: &Opts, shell: Arc<Shell>) -> Result<(), InitializationError> {
        self.shell = Some(Arc::clone(&shell));
        self.shell().status("Initializing", "Docker API provider");

        // Ping the Docker API to make sure the current process can connect
        let future = self.client.ping();
        match self.runtime.block_on(future) {
            Ok(_) => {},
            Err(_) => return Err(InitializationError::new(CONNECTION_ERROR_MESSAGE)),
        }

        // Make sure cgroups are mounted properly
        if !cgroups::cgroups_mounted_properly() {
            return Err(InitializationError::new(cgroups::INVALID_MOUNT_MESSAGE));
        }

        self.info_cache = Some(RefCell::new(LruCache::with_expiry_duration(
            opts.polling_interval * POLLING_BLOCK_EXPIRY,
        )));

        Ok(())
    }

    fn fetch(&mut self) -> Result<Vec<TargetMetadata>, FetchError> {
        let future = self.client.containers().list(&Default::default());
        match self.runtime.block_on(future) {
            Err(e) => Err(FetchError::new(Some(e.into()))),
            Ok(containers) => {
                let original_num = containers.len();
                let container_metadata = containers
                    .into_iter()
                    .filter_map(|c| self.convert_container(c))
                    .collect::<Vec<_>>();

                self.shell().verbose(|sh| {
                    sh.info(format!(
                        "Received {} -> {} containers from the Docker API",
                        original_num,
                        container_metadata.len()
                    ))
                });

                Ok(container_metadata)
            },
        }
    }
}

impl Docker {
    pub fn new() -> Box<dyn Provider> {
        Box::new(Docker {
            client:         shiplift::Docker::new(),
            runtime:        Runtime::new().unwrap(),
            cgroup_manager: CgroupManager::new(),
            info_cache:     None,
            shell:          None,
        })
    }

    /// Converts a container to a its metadata, or rejects it if it shouldn't
    /// be collected for the next polling tick
    fn convert_container(&mut self, c: Container) -> Option<TargetMetadata> {
        match should_collect_stats(&c) {
            true => match self.try_format_metadata(&c) {
                Some(metadata) => Some(metadata),
                None => {
                    self.shell().warn(format!(
                        "Could not create container metadata for container {}: cgroup path could \
                         not be constructed or does not exist",
                        c.names.iter().nth(0).unwrap_or(&c.id)
                    ));
                    None
                },
            },
            false => None,
        }
    }

    /// Try to get info from LRU cache before re-serializing it
    fn get_info(&mut self, c: &Container) -> String {
        let mut info_cache = self
            .info_cache
            .as_ref()
            .expect("LRU Cache must be initialized: invariant violated")
            .borrow_mut();
        match info_cache.get(&c.id) {
            Some(info) => info.clone(),
            None => {
                let info = self.format_info(&c);
                info_cache.insert(c.id.clone(), info.clone());
                info
            },
        }
    }

    /// Formats container info used for the header row and cgroup path. Can fail
    /// if the cgroup doesn't exist.
    fn try_format_metadata(&mut self, c: &Container) -> Option<TargetMetadata> {
        match self.get_cgroup(c) {
            None => None,
            Some(cgroup) => Some(TargetMetadata {
                id: c.id.clone(),
                info: self.get_info(c),
                cgroup,
                provider_type: PROVIDER_TYPE,
            }),
        }
    }

    /// Gets the group path for the given container, printing out a
    /// message upon the first successful cgroup resolution
    fn get_cgroup(&mut self, c: &Container) -> Option<CgroupPath> {
        // Determine if the manager had a resolved group beforehand
        let had_driver = self.cgroup_manager.driver().is_some();

        // Container cgroups are under the dockerd parent, and are in leaf
        // cgroups by (full) container ID. Cgroup path depends on the driver used:
        // according to https://docs.docker.com/engine/reference/commandline/dockerd/#default-cgroup-parent ,
        // "[container cgroups are mounted at] `/docker` for fs cgroup driver and
        // `system.slice` for systemd cgroup driver." The .slice is omitted
        let cgroup_option: Option<CgroupPath> = self
            .cgroup_manager
            .get_cgroup_divided(&["system", &c.id], &["docker", &c.id]);

        if !had_driver {
            if let Some(driver) = self.cgroup_manager.driver() {
                self.shell()
                    .info(format!("Identified {} as cgroup driver", driver));
            }
        }

        cgroup_option
    }

    /// Formats container info headers to YAML for display at the top of each
    /// CSV file. Uses serde-yaml to serialize the Container struct to YAML,
    /// before adding an extra field in `PolledAt`
    fn format_info(&self, c: &Container) -> String {
        match try_format_info(c) {
            Ok(yaml) => yaml,
            Err(err) => {
                self.shell().error(format!(
                    "Could not serialize container info for container {}: {}",
                    &c.id, err
                ));
                String::from("")
            },
        }
    }

    /// Gets a reference to the current shell
    fn shell(&self) -> &Shell {
        self.shell
            .as_ref()
            .expect("Shell must be initialized: invariant violated")
    }
}

/// Whether radvisor should collect statistics for the given container
fn should_collect_stats(_c: &Container) -> bool { true }

/// Attempts to format container info, potentially failing to do so
fn try_format_info(c: &Container) -> Result<String, Box<dyn std::error::Error>> {
    let serde_output = serde_yaml::to_string(c)?;
    // Remove top ---
    let mut yaml = String::from(serde_output.trim_start_matches("---\n")) + "\n";
    writeln!(&mut yaml, "PolledAt: {}", util::nano_ts())?;
    Ok(yaml)
}
