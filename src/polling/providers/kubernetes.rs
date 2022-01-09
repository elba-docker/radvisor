use crate::cli::RunCommand;
use crate::polling::providers::{InitializationError, KubernetesOptions, Provider};
use crate::shared::{CollectionEvent, CollectionMethod, CollectionTarget};
use crate::shell::Shell;
use crate::util::{self, CgroupManager, CgroupPath, ItemPool};
use failure::Error;
use gethostname::gethostname;
use k8s_openapi::api::core::v1::{Node, Pod};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Time;
use kube::api::{Api, ListParams, Meta};
use kube::client::Client;
use kube::config;
use serde::Serialize;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::future::Future;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use strum_macros::{EnumString, IntoStaticStr};

/// String representation for "None"
const NONE_STR: &str = "~";

/// Root cgroup for kubernetes pods to fall under
const ROOT_CGROUP: &str = "kubepods";

const PROVIDER_TYPE: &str = "kubernetes";

pub struct Kubernetes {
    cgroup_manager: CgroupManager,
    pod_uid_pool:   ItemPool<String>,
    runtime:        RefCell<tokio_02::runtime::Runtime>,
    pod_client:     Option<Api<Pod>>,
    node_client:    Option<Api<Node>>,
    shell:          Option<Arc<Shell>>,
    hostname:       Option<String>,
    node_name:      Option<String>,
}

/// Possible errors that can occur during Kubernetes provider initialization
#[derive(Debug)]
enum KubernetesInitError {
    InvalidCgroupMount,
    InvalidHostnameError(std::ffi::OsString),
    ConfigLoadError(Error),
    NodeDetectionError,
    NodeFetchError(Error),
    MissingNodeNameError,
}

impl From<KubernetesInitError> for InitializationError {
    fn from(other: KubernetesInitError) -> Self {
        match other {
            KubernetesInitError::InvalidCgroupMount => Self {
                original:   None,
                suggestion: String::from(util::INVALID_CGROUP_MOUNT_MESSAGE),
            },
            KubernetesInitError::InvalidHostnameError(hostname) => Self {
                original:   None,
                suggestion: format!(
                    "Could not retrieve hostname to use for node detection: Invalid string '{:?}' \
                     returned",
                    hostname
                ),
            },
            KubernetesInitError::ConfigLoadError(error) => Self {
                original:   Some(error),
                suggestion: String::from(
                    "Could not load kubernetes config. Make sure the current machine is a part of \
                     a cluster \nand has the cluster configuration copied to the config directory.",
                ),
            },
            KubernetesInitError::NodeDetectionError => Self {
                original:   None,
                suggestion: String::from(
                    "Could not get the current node via the Kubernetes API. \nMake sure the \
                     current machine is running its own node.",
                ),
            },
            KubernetesInitError::NodeFetchError(error) => Self {
                original:   Some(error),
                suggestion: String::from("Could not get list of nodes in the Kubernetes cluster"),
            },
            KubernetesInitError::MissingNodeNameError => Self {
                original:   None,
                suggestion: String::from(
                    "The node running on the current host lacks a Name field. \nThe pod polling \
                     cannot function without this.",
                ),
            },
        }
    }
}

/// Quality of service classes for pods. For more information, see
/// [the Kubernetes docs](https://kubernetes.io/docs/tasks/configure-pod-container/quality-service-pod/#qos-classes)
#[derive(EnumString, IntoStaticStr, Clone, Copy, Debug, PartialEq, Serialize)]
#[strum(serialize_all = "lowercase")]
enum QualityOfService {
    BestEffort,
    Guaranteed,
    Burstable,
}

impl QualityOfService {
    /// Attempts to extract the quality of service value from a pod's status
    fn from_pod(pod: &Pod) -> Option<Self> {
        pod.status
            .as_ref()
            .and_then(|status| status.qos_class.as_ref())
            // Use strum_macro's `EnumString::from_str` implementation here
            .and_then(|qos| Self::from_str(&qos.to_lowercase()).ok())
    }
}

/// Possible error that can occur during Kubernetes pod collection target
/// initialization
#[derive(Debug)]
enum StartCollectionError {
    MetadataSerializationError(Error),
    CgroupNotFound,
    MissingPodUid,
    FailedQosParse,
}

impl StartCollectionError {
    fn display(&self, pod: &Pod) -> String {
        let pod_display: &str = name_option(pod)
            .or_else(|| uid_option(pod))
            .unwrap_or(NONE_STR);
        match self {
            Self::CgroupNotFound => format!(
                "Could not create pod metadata for pod {}: cgroup path could not be constructed \
                 or does not exist",
                pod_display
            ),
            Self::MetadataSerializationError(cause) => format!(
                "Could not serialize metadata for pod {}: {}",
                pod_display, cause
            ),
            Self::MissingPodUid => format!(
                "Could not get uid for node {}! This shouldn't happen",
                pod_display
            ),
            Self::FailedQosParse => format!(
                "Could not parse quality of service class for pod {}: invalid value '{}'",
                pod_display,
                pod.status
                    .as_ref()
                    .and_then(|s| s.qos_class.as_deref())
                    .unwrap_or(NONE_STR)
            ),
        }
    }
}

impl Provider for Kubernetes {
    fn initialize(
        &mut self,
        opts: &RunCommand,
        shell: Arc<Shell>,
    ) -> Result<(), InitializationError> {
        self.shell = Some(Arc::clone(&shell));
        self.shell()
            .status("Initializing", "Kubernetes API provider");

        let inner_opts: KubernetesOptions = opts.provider.clone().into_inner_kubernetes();
        match self.try_init(inner_opts.kube_config) {
            Ok(_) => Ok(()),
            Err(init_err) => Err(init_err.into()),
        }
    }

    fn poll(&mut self) -> Result<Vec<CollectionEvent>, Error> {
        let pods = self.get_pods()?;

        let original_num = pods.len();
        let pods_map: BTreeMap<String, Pod> = pods
            .into_iter()
            .filter_map(|p| {
                let uid = uid_option(&p);
                uid.map(ToOwned::to_owned).map(|id| (id, p))
            })
            .collect::<BTreeMap<_, _>>();

        let uids = pods_map.keys().map(String::clone);
        let mut events: Vec<CollectionEvent> = Vec::new();
        let (added, removed) = self.pod_uid_pool.update(uids);

        let removed_len = removed.len();
        events.reserve_exact(added.len() + removed.len());
        // Add all removed Ids as Stop events
        events.extend(removed.into_iter().map(CollectionEvent::Stop));

        // Add all added Ids as Start events
        let start_events = added
            .into_iter()
            .filter_map(|uid| {
                // It shouldn't be possible to have a Uid that doesn't exist in the map, but
                // check anyways
                let pod: &Pod = match pods_map.get(uid.as_str()) {
                    Some(pod) => pod,
                    None => {
                        self.shell().error(format!(
                            "Processed Uid from ItemPool added result that was not in fetched pod \
                             list. This is a bug!\nUid: {}",
                            uid
                        ));
                        return None;
                    },
                };

                match self.make_start_event(pod) {
                    Ok(start) => Some(start),
                    Err(error) => {
                        self.shell().warn(error.display(pod));
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
                    "Received {} -> {} (+{}, -{}) containers from the Kubernetes API",
                    original_num,
                    pods_map.len(),
                    processed_num,
                    removed_len
                ));
            });
        }

        Ok(events)
    }
}

impl Default for Kubernetes {
    fn default() -> Self { Self::new() }
}

impl Kubernetes {
    #[must_use]
    pub fn new() -> Self {
        // Use a single-threaded runtime so that Tokio doesn't create
        // a thread pool and instead executes futures in the current thread
        // (emulating synchronous I/O)
        let runtime = tokio_02::runtime::Builder::new()
            .basic_scheduler()
            .enable_all()
            .build()
            .unwrap();
        Self {
            cgroup_manager: CgroupManager::new(),
            pod_uid_pool:   ItemPool::new(),
            runtime:        RefCell::new(runtime),
            pod_client:     None,
            node_client:    None,
            hostname:       None,
            node_name:      None,
            shell:          None,
        }
    }

    /// Executes a future on the internal runtime, blocking the current thread
    /// until it completes
    fn exec<F: Future>(&self, future: F) -> F::Output {
        let mut rt = self.runtime.borrow_mut();
        rt.block_on(future)
    }

    /// Attempts to initialize the Kubernetes provider, failing if one of the
    /// following conditions happens:
    ///   1. Invalid hostname from system
    ///   2. Can't load Kubernetes config from filesystem
    ///   3. Cgroups mounted unexpectedly/improperly
    ///   4. API server/Node can't be communicated with
    fn try_init(&mut self, kube_config: Option<PathBuf>) -> Result<(), KubernetesInitError> {
        if !util::cgroups_mounted_properly() {
            return Err(KubernetesInitError::InvalidCgroupMount);
        }

        // Load the config using the given kubeconfig file if given,
        // otherwise use the standard series of potential sources
        let config_load_result = match kube_config {
            None => self.exec(config::Config::infer()),
            Some(kube_config) => self.exec(config::Config::from_custom_kubeconfig(
                config::Kubeconfig::read_from(kube_config)
                    .map_err(|err| KubernetesInitError::ConfigLoadError(Error::from(err)))?,
                &config::KubeConfigOptions::default(),
            )),
        };
        let config = config_load_result
            .map_err(|err| KubernetesInitError::ConfigLoadError(Error::from(err)))?;

        // Initialize the API clients
        let client = Client::new(config);
        self.pod_client = Some(Api::all(client.clone()));
        self.node_client = Some(Api::all(client));

        // Load the hostname of the machine
        let hostname = gethostname()
            .into_string()
            .map_err(KubernetesInitError::InvalidHostnameError)?;
        self.hostname = Some(hostname);

        // Get current node by hostname and store in provider
        let node = self.get_current_node()?;
        let node_name = name_option(&node).ok_or(KubernetesInitError::MissingNodeNameError)?;
        self.node_name = Some(String::from(node_name));

        Ok(())
    }

    /// Tries to get the current Node from the kubernetes API by its hostname
    fn get_current_node(&self) -> Result<Node, KubernetesInitError> {
        // Attempt to find a node with the kubernetes.io/hostname label set
        let lp =
            ListParams::default().labels(&format!("kubernetes.io/hostname={}", self.hostname()));
        let future = self.node_client().list(&lp);
        self.exec(future)
            .map_err(|err| KubernetesInitError::NodeFetchError(Error::from(err)))?
            .into_iter()
            .next()
            .ok_or(KubernetesInitError::NodeDetectionError)
    }

    /// Tries to get all pods that are running on the current node, polling the
    /// Kubernetes API backend to get a fresh list
    fn get_pods(&self) -> Result<Vec<Pod>, Error> {
        let lp = ListParams::default().fields(&format!("spec.nodeName={}", self.node_name()));
        let future = self.pod_client().list(&lp);
        let pods = self.exec(future)?.into_iter().collect::<Vec<_>>();
        Ok(pods)
    }

    /// Converts a pod to a collection start event, preparing all
    /// serialization/cgroup checks needed
    fn make_start_event(&mut self, pod: &Pod) -> Result<CollectionEvent, StartCollectionError> {
        let uid: &str = uid_option(pod).ok_or(StartCollectionError::MissingPodUid)?;
        let method = self.get_collection_method(pod, uid)?;
        let metadata = match serialize_pod_info(pod) {
            Ok(metadata) => metadata,
            Err(err) => {
                return Err(StartCollectionError::MetadataSerializationError(err));
            },
        };

        Ok(CollectionEvent::Start {
            method,
            target: CollectionTarget {
                provider:  PROVIDER_TYPE,
                metadata:  Some(metadata),
                name:      name(pod).to_owned(),
                poll_time: util::nano_ts(),
                id:        uid.to_owned(),
            },
        })
    }

    /// Gets the collection method struct for the pod, resolving the
    /// proper collection method
    fn get_collection_method(
        &mut self,
        pod: &Pod,
        uid: &str,
    ) -> Result<CollectionMethod, StartCollectionError> {
        // Only one type of CollectionMethod currently
        let qos_class: QualityOfService =
            QualityOfService::from_pod(pod).ok_or(StartCollectionError::FailedQosParse)?;

        // Construct the cgroup path from the UID and QoS class
        // from the metadata, and make sure it exists/is mounted
        match self.get_cgroup(uid, qos_class) {
            Some(cgroup) => Ok(CollectionMethod::LinuxCgroups(cgroup)),
            None => Err(StartCollectionError::CgroupNotFound),
        }
    }

    /// Gets the group path for the given UID and quality of service class,
    /// printing out a message upon the first successful cgroup resolution
    fn get_cgroup(&mut self, uid: &str, qos_class: QualityOfService) -> Option<CgroupPath> {
        let pod_slice = String::from("pod") + uid;
        // Determine if the manager had a resolved group beforehand
        let had_driver = self.cgroup_manager.driver().is_some();

        let cgroup_option: Option<CgroupPath> = self
            .cgroup_manager
            .get_cgroup(&[ROOT_CGROUP, qos_class.into(), &pod_slice], true);

        if !had_driver {
            if let Some(driver) = self.cgroup_manager.driver() {
                self.shell()
                    .info(format!("Identified {} as cgroup driver", driver));
            }
        }

        cgroup_option
    }

    /// Gets the current node's hostname
    fn hostname(&self) -> &str {
        self.hostname
            .as_ref()
            .expect("Hostname must be initialized: invariant violated")
    }

    /// Gets the current node's name
    fn node_name(&self) -> &str {
        self.node_name
            .as_ref()
            .expect("Node name must be initialized: invariant violated")
    }

    /// Gets a reference to the current pod client
    fn pod_client(&self) -> &Api<Pod> {
        self.pod_client
            .as_ref()
            .expect("Pod client must be initialized: invariant violated")
    }

    /// Gets a reference to the current node client
    fn node_client(&self) -> &Api<Node> {
        self.node_client
            .as_ref()
            .expect("Node client must be initialized: invariant violated")
    }

    /// Gets a reference to the current shell
    fn shell(&self) -> &Shell {
        self.shell
            .as_ref()
            .expect("Shell must be initialized: invariant violated")
    }
}

/// Pod info struct that gets included with each log file
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
struct PodInfo<'a> {
    uid:        &'a Option<String>,
    name:       &'a Option<String>,
    created_at: &'a Option<Time>,
    labels:     &'a Option<BTreeMap<String, String>>,
    namespace:  &'a Option<String>,
    node_name:  &'a Option<String>,
    host_ip:    &'a Option<String>,
    phase:      &'a Option<String>,
    qos_class:  &'a Option<String>,
    started_at: &'a Option<Time>,
}

impl<'a> PodInfo<'a> {
    /// Attempts to extract all state/metadata from the given pod, and collects
    /// it in a single pod info struct
    fn new(p: &'a Pod) -> Self {
        let meta = Meta::meta(p);
        let (uid, name, created_at, labels, namespace) = (
            &meta.uid,
            &meta.name,
            &meta.creation_timestamp,
            &meta.labels,
            &meta.namespace,
        );

        let node_name = match p.spec.as_ref() {
            None => &None,
            Some(spec) => &spec.node_name,
        };

        let (host_ip, phase, qos_class, started_at) = match p.status.as_ref() {
            None => (&None, &None, &None, &None),
            Some(status) => (
                &status.host_ip,
                &status.phase,
                &status.qos_class,
                &status.start_time,
            ),
        };

        PodInfo {
            uid,
            name,
            created_at,
            labels,
            namespace,
            node_name,
            host_ip,
            phase,
            qos_class,
            started_at,
        }
    }
}

/// Attempts to format pod info, potentially failing to do so
fn serialize_pod_info(pod: &Pod) -> Result<serde_yaml::Value, Error> {
    let pod_info = PodInfo::new(pod);
    let serde_output = serde_yaml::to_value(&pod_info)?;
    Ok(serde_output)
}

fn uid_option<O: Meta>(obj: &O) -> Option<&str> { Meta::meta(obj).uid.as_deref() }

fn name_option<O: Meta>(obj: &O) -> Option<&str> { Meta::meta(obj).name.as_deref() }

fn name<O: Meta>(obj: &O) -> &str { name_option(obj).unwrap_or(NONE_STR) }
