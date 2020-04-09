use crate::cli::Opts;
use crate::polling::providers::{InitializationError, Provider};
use crate::shared::{CollectionEvent, CollectionMethod, CollectionTarget};
use crate::shell::Shell;
use crate::util::{self, CgroupManager, CgroupPath, ItemPool};
use std::collections::BTreeMap;
use std::sync::Arc;

use failure::Error;
use shiplift::rep::Container;
use tokio_compat::runtime::Runtime;

const PROVIDER_TYPE: &str = "docker";

pub struct Docker {
    container_id_pool: ItemPool<String>,
    cgroup_manager:    CgroupManager,
    runtime:           Runtime,
    client:            shiplift::Docker,
    shell:             Option<Arc<Shell>>,
}

impl Default for Docker {
    fn default() -> Self {
        Docker {
            container_id_pool: ItemPool::new(),
            cgroup_manager:    CgroupManager::new(),
            runtime:           Runtime::new().unwrap(),
            client:            shiplift::Docker::new(),
            shell:             None,
        }
    }
}

/// Possible errors that can occur during Docker provider initialization
#[derive(Clone, Copy, Debug)]
enum DockerInitError {
    ConnectionFailed,
    InvalidCgroupMount,
}

impl Into<InitializationError> for DockerInitError {
    fn into(self) -> InitializationError {
        match self {
            DockerInitError::ConnectionFailed => InitializationError {
                suggestion: String::from(
                    "Could not connect to the docker socket. Are you running rAdvisor as \
                     root?\nIf running at a non-standard URL, set DOCKER_HOST to the correct URL.",
                ),
            },
            DockerInitError::InvalidCgroupMount => InitializationError {
                suggestion: String::from(util::INVALID_CGROUP_MOUNT_MESSAGE),
            },
        }
    }
}

/// Possible error that can occur during Docker container collection target
/// initialization
#[derive(Debug)]
enum StartCollectionError {
    MetadataSerializationError(Error),
    CgroupNotFound,
}

impl Provider for Docker {
    fn initialize(&mut self, _opts: &Opts, shell: Arc<Shell>) -> Result<(), InitializationError> {
        self.shell = Some(Arc::clone(&shell));
        self.shell().status("Initializing", "Docker API provider");

        match self.try_init() {
            Ok(_) => Ok(()),
            Err(init_err) => Err(init_err.into()),
        }
    }

    fn poll(&mut self) -> Result<Vec<CollectionEvent>, Error> {
        let future = self.client.containers().list(&Default::default());
        let containers = self.runtime.block_on(future)?;

        let original_num = containers.len();
        let to_collect: BTreeMap<String, Container> = containers
            .into_iter()
            .filter(should_collect_stats)
            .map(|c| (c.id.clone(), c))
            .collect::<BTreeMap<_, _>>();

        let ids = to_collect.keys().map(String::clone);
        let mut events: Vec<CollectionEvent> = Vec::new();
        let (added, removed) = self.container_id_pool.update(ids);

        let removed_len = removed.len();
        events.reserve_exact(added.len() + removed_len);
        // Add all removed Ids as Stop events
        events.extend(removed.into_iter().map(CollectionEvent::Stop));

        // Add all added Ids as Start events
        let start_events = added
            .into_iter()
            .flat_map(|id| {
                // It shouldn't be possible to have an Id that doesn't exist in the map, but
                // check anyways
                let container = match to_collect.get(&id) {
                    Some(container) => container,
                    None => {
                        self.shell().error(format!(
                            "Processed Id from ItemPool added result that was not in fetched \
                             container list. This is a bug!\nId: {}",
                            id
                        ));
                        return None;
                    },
                };

                match self.make_start_event(container) {
                    Ok(start) => Some(start),
                    Err(error) => {
                        let container_display = display(container);
                        match error {
                            StartCollectionError::CgroupNotFound => {
                                self.shell().warn(format!(
                                    "Could not create container metadata for container {}: cgroup \
                                     path could not be constructed or does not exist",
                                    container_display
                                ));
                            },
                            StartCollectionError::MetadataSerializationError(cause) => {
                                self.shell().warn(format!(
                                    "Could not serialize container metadata: {}",
                                    cause
                                ));
                            },
                        }

                        // Ignore container and continue initializing the rest
                        None
                    },
                }
            })
            .collect::<Vec<_>>();
        let processed_num = start_events.len();
        events.extend(start_events);

        self.shell().verbose(|sh| {
            sh.info(format!(
                "Received {} -> {} (+{}, -{}) containers from the Docker API",
                original_num,
                to_collect.len(),
                processed_num,
                removed_len
            ))
        });

        Ok(events)
    }
}

impl Docker {
    pub fn new() -> Self { Default::default() }

    /// Attempts to initialize the Docker provider, failing if the connection
    /// check to the Docker daemon failed or if the needed Cgroups aren't
    /// mounted properly
    fn try_init(&mut self) -> Result<(), DockerInitError> {
        // Ping the Docker API to make sure the current process can connect
        let future = self.client.ping();
        self.runtime
            .block_on(future)
            .map_err(|_| DockerInitError::ConnectionFailed)?;

        // Make sure cgroups are mounted properly
        if !util::cgroups_mounted_properly() {
            return Err(DockerInitError::InvalidCgroupMount);
        }

        Ok(())
    }

    /// Converts a container to a collection start event, preparing all
    /// serialization/cgroup checks needed
    fn make_start_event(
        &mut self,
        container: &Container,
    ) -> Result<CollectionEvent, StartCollectionError> {
        let method = self.get_collection_method(container)?;
        let metadata = match serde_yaml::to_value(container) {
            Ok(metadata) => metadata,
            Err(err) => {
                return Err(StartCollectionError::MetadataSerializationError(
                    Error::from(err),
                ));
            },
        };

        Ok(CollectionEvent::Start {
            method,
            target: CollectionTarget {
                provider: PROVIDER_TYPE,
                metadata: Some(metadata),
                name:     container.names.get(0).unwrap_or(&container.id).clone(),
                id:       container.id.clone(),
            },
        })
    }

    /// Gets the collection method struct for the container, resolving the
    /// proper collection method
    fn get_collection_method(
        &mut self,
        container: &Container,
    ) -> Result<CollectionMethod, StartCollectionError> {
        // Only one type of CollectionMethod currently
        match self.get_cgroup(container) {
            Some(cgroup) => Ok(CollectionMethod::LinuxCgroups(cgroup)),
            None => Err(StartCollectionError::CgroupNotFound),
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

    /// Gets a reference to the current shell
    fn shell(&self) -> &Shell {
        self.shell
            .as_ref()
            .expect("Shell must be initialized: invariant violated")
    }
}

/// Whether radvisor should collect statistics for the given container
fn should_collect_stats(_c: &Container) -> bool { true }

/// Gets a human-readable representation of the container, attempting to use the
/// name before using the Id as a fallback
fn display(container: &Container) -> &str { container.names.get(0).unwrap_or(&container.id) }
