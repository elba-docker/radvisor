use crate::cli::RunCommand;
use crate::polling::providers::{InitializationError, Provider};
use crate::shared::{CollectionEvent, CollectionMethod, CollectionTarget};
use crate::shell::Shell;
use crate::util::{self, CgroupManager, CgroupPath, CgroupSlices, GetCgroupError, ItemPool};
use anyhow::Error;
use shiplift::builder::ContainerListOptions;
use shiplift::rep::Container;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;

const PROVIDER_TYPE: &str = "docker";

pub struct Docker {
    container_id_pool: ItemPool<String>,
    cgroup_manager:    CgroupManager,
    client:            shiplift::Docker,
    shell:             Option<Arc<Shell>>,
    runtime:           Runtime,
}

/// Possible errors that can occur during Docker provider initialization
#[derive(Debug)]
enum DockerInitError {
    ConnectionFailed(shiplift::Error),
    InvalidCgroupMount,
}

impl From<DockerInitError> for InitializationError {
    fn from(other: DockerInitError) -> Self {
        match other {
            DockerInitError::ConnectionFailed(error) => Self {
                original:   Some(error.into()),
                suggestion: String::from(
                    "Could not connect to the docker socket. Are you running rAdvisor as \
                     root?\nIf running at a non-standard URL, set DOCKER_HOST to the correct URL.",
                ),
            },
            DockerInitError::InvalidCgroupMount => Self {
                original:   None,
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
    CgroupVersionDetectionFailed,
    CgroupNotFound(PathBuf),
}

impl Provider for Docker {
    fn initialize(
        &mut self,
        _opts: &RunCommand,
        shell: Arc<Shell>,
    ) -> Result<(), InitializationError> {
        self.shell = Some(Arc::clone(&shell));
        self.shell().status("Initializing", "Docker API provider");

        match self.try_init() {
            Ok(_) => Ok(()),
            Err(init_err) => Err(init_err.into()),
        }
    }

    fn poll(&mut self) -> Result<Vec<CollectionEvent>, Error> {
        let containers = self.client.containers();
        let container_options = ContainerListOptions::default();
        let future = containers.list(&container_options);
        let containers = self.runtime.block_on(future)?;

        let original_num = containers.len();
        let to_collect: BTreeMap<String, Container> = containers
            .into_iter()
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
            .filter_map(|id| {
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
                            StartCollectionError::CgroupNotFound(path) => {
                                self.shell().warn(format!(
                                    "Could not start collection for container {}: cgroup path \
                                     '{:?}' does not exist on system",
                                    container_display, path,
                                ));
                            },
                            StartCollectionError::MetadataSerializationError(cause) => {
                                self.shell().warn(format!(
                                    "Could not start collection for container {}: failed to \
                                     serialize container metadata: {}",
                                    container_display, cause
                                ));
                            },
                            StartCollectionError::CgroupVersionDetectionFailed => {
                                self.shell().warn(format!(
                                    "Could not start collection for container {}: failed to \
                                     detect the currently running cgroup version (are cgroups \
                                     mounted in /sys/fs/cgroup?)",
                                    container_display
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

        if processed_num != 0 || removed_len != 0 {
            self.shell().verbose(|sh| {
                sh.info(format!(
                    "Received {} -> {} (+{}, -{}) containers from the Docker API",
                    original_num,
                    to_collect.len(),
                    processed_num,
                    removed_len
                ));
            });
        }

        Ok(events)
    }
}

impl Default for Docker {
    fn default() -> Self { Self::new() }
}

impl Docker {
    #[must_use]
    pub fn new() -> Self {
        // Use a single-threaded runtime so that Tokio doesn't create
        // a thread pool and instead executes futures in the current thread
        // (emulating synchronous I/O)
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .enable_io()
            .build()
            .unwrap();
        Self {
            container_id_pool: ItemPool::new(),
            cgroup_manager: CgroupManager::new(),
            client: shiplift::Docker::new(),
            shell: None,
            runtime,
        }
    }

    /// Attempts to initialize the Docker provider, failing if the connection
    /// check to the Docker daemon failed or if the needed cgroups aren't
    /// mounted properly
    fn try_init(&mut self) -> Result<(), DockerInitError> {
        // Ping the Docker API to make sure the current process can connect
        let future = self.client.ping();
        self.runtime
            .block_on(future)
            .map_err(DockerInitError::ConnectionFailed)?;

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
                provider:  PROVIDER_TYPE,
                metadata:  Some(metadata),
                name:      container.names.get(0).unwrap_or(&container.id).clone(),
                poll_time: util::nano_ts(),
                id:        container.id.clone(),
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
            Ok(cgroup) => match cgroup.version {
                util::CgroupVersion::V1 => Ok(CollectionMethod::LinuxCgroupV1(cgroup)),
                util::CgroupVersion::V2 => Ok(CollectionMethod::LinuxCgroupV2(cgroup)),
            },
            Err(GetCgroupError::VersionDetectionFailed) => {
                Err(StartCollectionError::CgroupVersionDetectionFailed)
            },
            Err(GetCgroupError::NotFound(path)) => Err(StartCollectionError::CgroupNotFound(path)),
            Err(GetCgroupError::CgroupV1NotEnabled) => unreachable!(),
        }
    }

    /// Gets the group path for the given container, printing out a
    /// message upon the first successful cgroup resolution
    fn get_cgroup(&mut self, c: &Container) -> Result<CgroupPath, GetCgroupError> {
        // Determine if the manager had a resolved version or driver beforehand
        let had_driver = self.cgroup_manager.driver().is_some();
        let had_version = self.cgroup_manager.version().is_some();

        // Container cgroups are under the dockerd parent, and are in leaf
        // cgroups by (full) container ID.
        // Cgroup path depends on the driver used and version enabled.
        // According to https://docs.docker.com/engine/reference/commandline/dockerd/#default-cgroup-parent ,
        // "[container cgroups are mounted at] `/docker` for fs cgroup driver and
        // `system.slice` for systemd cgroup driver."
        let result = self.cgroup_manager.get_cgroup(CgroupSlices {
            systemd:  &["system.slice", &format!("docker-{}.scope", &c.id)],
            cgroupfs: &["docker", &c.id],
        });

        if !had_driver {
            if let Some(driver) = self.cgroup_manager.driver() {
                self.shell()
                    .info(format!("Identified {} as cgroup driver", driver));
            }
        }

        if !had_version {
            if let Some(version) = self.cgroup_manager.version() {
                self.shell()
                    .info(format!("Identified {} as cgroup version", version));
            }
        }

        result
    }

    /// Gets a reference to the current shell
    fn shell(&self) -> &Shell {
        self.shell
            .as_ref()
            .expect("Shell must be initialized: invariant violated")
    }
}

/// Gets a human-readable representation of the container, attempting to use the
/// name before using the Id as a fallback
fn display(container: &Container) -> &str { container.names.get(0).unwrap_or(&container.id) }
