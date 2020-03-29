use crate::cli::Opts;
use crate::polling::providers::cgroups::{self, CgroupManager};
use crate::polling::providers::{FetchError, InitializationError, Provider};
use crate::shared::TargetMetadata;
use crate::util;
use std::fmt::Write;
use std::time::Duration;

use colored::*;
use lru_time_cache::LruCache;
use serde_yaml;
use shiplift::rep::Container;
use tokio_compat::runtime::Runtime;

const CONNECTION_ERROR_MESSAGE: &str = "Could not connect to the docker socket. Are you running \
                                        rAdvisor as root?\nIf running at a non-standard URL, set \
                                        DOCKER_HOST to the correct URL.";

/// Number of polling blocks that need to elapse before container info strings
/// are evicted from the time-based LRU cache
const POLLING_BLOCK_EXPIRY: u64 = 5;

pub struct Docker {
    client:         shiplift::Docker,
    runtime:        Runtime,
    cgroup_manager: CgroupManager,
    info_cache:     Option<LruCache<String, String>>,
}

impl Docker {
    pub fn new() -> Box<dyn Provider> {
        Box::new(Docker {
            client:         shiplift::Docker::new(),
            runtime:        Runtime::new().unwrap(),
            cgroup_manager: CgroupManager::new(),
            info_cache:     None,
        })
    }

    /// Converts a container to a its metadata, or rejects it if it shouldn't
    /// be collected for the next polling tick
    fn convert_container(&mut self, c: Container) -> Option<TargetMetadata> {
        match should_collect_stats(&c) {
            true => match self.try_format_metadata(&c) {
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

    /// Try to get info from LRU cache before re-serializing it
    fn get_info(&mut self, c: &Container, cgroup: &str) -> String {
        let info_cache = self
            .info_cache
            .as_mut()
            .expect("LRU Cache must be initialized: invariant violated");
        match info_cache.get(&c.id) {
            Some(info) => info.clone(),
            None => {
                let info = format_info(&c, &cgroup);
                println!("{}", info);
                info_cache.insert(c.id.clone(), info.clone());
                info
            },
        }
    }

    /// Formats container info used for the header row and cgroup path. Can fail
    /// if the cgroup doesn't exist.
    fn try_format_metadata(&mut self, c: &Container) -> Option<TargetMetadata> {
        // Container cgroups are under the dockerd parent, and are in leaf
        // cgroups by (full) container ID. Cgroup path depends on the driver used:
        // according to https://docs.docker.com/engine/reference/commandline/dockerd/#default-cgroup-parent ,
        // "[container cgroups are mounted at] `/docker` for fs cgroup driver and
        // `system.slice` for systemd cgroup driver." The .slice is omitted
        match self
            .cgroup_manager
            .get_cgroup_divided(&["system", &c.id], &["docker", &c.id])
        {
            None => None,
            Some(cgroup) => Some(TargetMetadata {
                id: c.id.clone(),
                info: self.get_info(c, &cgroup),
                cgroup,
            }),
        }
    }
}

impl Provider for Docker {
    fn initialize(&mut self, opts: &Opts) -> Option<InitializationError> {
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

        self.info_cache = Some(LruCache::with_expiry_duration(Duration::from_millis(
            opts.polling_interval * POLLING_BLOCK_EXPIRY,
        )));

        None
    }

    fn fetch(&mut self) -> Result<Vec<TargetMetadata>, FetchError> {
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

/// Formats container info headers to YAML for display at the top of each CSV
/// file. Uses serde-yaml to serialize the Container struct to YAML, before
/// adding a couple extra fields in `PollTime` and `Cgroup`
fn format_info(c: &Container, cgroup: &str) -> String {
    match try_format_info(c, cgroup) {
        Ok(yaml) => yaml,
        Err(err) => {
            eprintln!(
                "Could not serialize container info for container {}:\n{}",
                &c.id.red(),
                format!("{:?}", err).red()
            );
            String::from("")
        },
    }
}

/// Attempts to format container info, potentially failing to do so
fn try_format_info(c: &Container, cgroup: &str) -> Result<String, Box<dyn std::error::Error>> {
    let serde_output = serde_yaml::to_string(c)?;
    // Remove top ---
    let mut yaml = String::from(serde_output.trim_start_matches("---\n")) + "\n";
    writeln!(&mut yaml, "PollTime: {}", util::nano_ts())?;
    writeln!(&mut yaml, "Cgroup: {}", cgroup)?;
    Ok(yaml)
}
