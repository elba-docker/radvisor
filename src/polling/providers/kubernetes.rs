use crate::cli::RunCommand;
use crate::polling::providers::{InitializationError, Provider};
use crate::shared::{CollectionEvent, CollectionMethod, CollectionTarget};
use crate::shell::Shell;
use crate::util::{self, CgroupManager, CgroupPath, ItemPool};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::future::Future;
use std::str::FromStr;
use std::sync::Arc;

use failure::Error;
use gethostname::gethostname;
use k8s_openapi::api::core::v1::{Node, Pod};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Time;
use kube::api::{ListParams, Meta, Resource};
use kube::client::Client;
use kube::config;
use kube::runtime::Reflector;
use serde::Serialize;
use strum_macros::{EnumString, IntoStaticStr};
use tokio::runtime::Runtime;

/// String representation for "None"
const NONE_STR: &str = "~";

/// Root cgroup for kubernetes pods to fall under
const ROOT_CGROUP: &str = "kubepods";

const PROVIDER_TYPE: &str = "kubernetes";

pub struct Kubernetes {
    cgroup_manager: CgroupManager,
    pod_uid_pool:   ItemPool<String>,
    runtime:        RefCell<Runtime>,
    pod_reflector:  Option<Reflector<Pod>>,
    client:         Option<Client>,
    shell:          Option<Arc<Shell>>,
}

impl Default for Kubernetes {
    fn default() -> Self {
        Kubernetes {
            cgroup_manager: CgroupManager::new(),
            pod_uid_pool:   ItemPool::new(),
            runtime:        RefCell::new(Runtime::new().unwrap()),
            pod_reflector:  None,
            client:         None,
            shell:          None,
        }
    }
}

/// Possible errors that can occur during Kubernetes provider initialization
#[derive(Debug)]
enum KubernetesInitError {
    InvalidCgroupMount,
    InvalidHostnameError,
    ConfigLoadError,
    NodeDetectionError,
    NodeFetchError(Error),
    InitialPodFetchError,
    MissingNodeNameError,
}

impl Into<InitializationError> for KubernetesInitError {
    fn into(self) -> InitializationError {
        // Convert various Kubernetes init errors to their CLI suggestion message
        InitializationError {
            suggestion: match self {
                KubernetesInitError::InvalidCgroupMount => {
                    util::INVALID_CGROUP_MOUNT_MESSAGE.to_owned()
                },
                KubernetesInitError::InvalidHostnameError => {
                    "Could not retrieve hostname to use for node detection: Invalid string returned"
                        .to_owned()
                },
                KubernetesInitError::ConfigLoadError => {
                    "Could not load kubernetes config. Make sure the current machine is a part of \
                     a cluster and has the cluster configuration copied to the config directory."
                        .to_owned()
                },
                KubernetesInitError::NodeDetectionError => {
                    "Could not get the current node via the Kubernetes API. Make sure the current \
                     machine is running its own node."
                        .to_owned()
                },
                KubernetesInitError::NodeFetchError(err) => format!(
                    "Could not get list of nodes in the Kubernetes cluster: {}",
                    err
                ),
                KubernetesInitError::InitialPodFetchError => {
                    "Could not get the list of pods running on the current machine. Make sure the \
                     node can access the API."
                        .to_owned()
                },
                KubernetesInitError::MissingNodeNameError => {
                    "The node running on the current host lacks a Name field. The pod polling \
                     cannot function without this."
                        .to_owned()
                },
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
            // Use EnumString macro's `from_str` implementation here
            .and_then(|qos| Self::from_str(&qos).ok())
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
            StartCollectionError::CgroupNotFound => format!(
                "Could not create pod metadata for pod {}: cgroup path could not be constructed \
                 or does not exist",
                pod_display
            ),
            StartCollectionError::MetadataSerializationError(cause) => format!(
                "Could not serialize metadata for pod {}: {}",
                pod_display, cause
            ),
            StartCollectionError::MissingPodUid => format!(
                "Could not get uid for node {}! This shouldn't happen",
                pod_display
            ),
            StartCollectionError::FailedQosParse => format!(
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
        _opts: &RunCommand,
        shell: Arc<Shell>,
    ) -> Result<(), InitializationError> {
        self.shell = Some(Arc::clone(&shell));
        self.shell()
            .status("Initializing", "Kubernetes API provider");

        match self.try_init() {
            Ok(_) => Ok(()),
            Err(init_err) => Err(init_err.into()),
        }
    }

    fn poll(&mut self) -> Result<Vec<CollectionEvent>, Error> {
        let pods = self.get_pods()?;

        let original_num = pods.len();
        let pods_map: BTreeMap<String, Pod> = pods
            .into_iter()
            .flat_map(|p| {
                let uid = uid_option(&p);
                uid.map(|s| s.to_owned()).map(|id| (id, p))
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
            .flat_map(|uid| {
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

        self.shell().verbose(|sh| {
            sh.info(format!(
                "Received {} -> {} (+{}, -{}) containers from the Docker API",
                original_num,
                pods_map.len(),
                processed_num,
                removed_len
            ))
        });

        Ok(events)
    }
}

impl Kubernetes {
    pub fn new() -> Self { Default::default() }

    /// Executes a future on the internal runtime, blocking the current thread
    /// until it completes
    fn exec<F: Future>(&self, future: F) -> F::Output {
        let mut runtime = self.runtime.borrow_mut();
        runtime.block_on(future)
    }

    /// Attempts to initialize the Kubernetes provider, failing if one of the
    /// following conditions happens:   1. Invalid hostname from system
    ///   2. Can't load Kubernetes config from filesystem
    ///   3. Cgroups mounted unexpectedly/improperly
    ///   4. API server/Node can't be communicated with
    fn try_init(&mut self) -> Result<(), KubernetesInitError> {
        if !util::cgroups_mounted_properly() {
            return Err(KubernetesInitError::InvalidCgroupMount);
        }

        let config = self
            .exec(config::load_kube_config())
            .map_err(|_| KubernetesInitError::ConfigLoadError)?;
        self.client = Some(Client::from(config));

        // Get current node by hostname and store in provider
        let node = self.get_current_node()?;
        let node_name = name_option(&node).ok_or(KubernetesInitError::MissingNodeNameError)?;
        self.pod_reflector = Some(self.initialize_pod_reflector(node_name)?);

        Ok(())
    }

    /// Tries to get the current Node from the kubernetes API by its hostname
    fn get_current_node(&self) -> Result<Node, KubernetesInitError> {
        let resource = Resource::all::<Node>();
        let lp = ListParams::default().timeout(10);
        let reflector: Reflector<Node> = self
            .exec(Reflector::new(self.client().clone(), lp, resource).init())
            .map_err(|err| KubernetesInitError::NodeFetchError(Error::from(err)))?;

        let nodes = self
            .exec(reflector.state())
            .map_err(|err| KubernetesInitError::NodeFetchError(Error::from(err)))?;

        let hostname = gethostname()
            .into_string()
            .map_err(|_| KubernetesInitError::InvalidHostnameError)?;

        // Try to get a node with the given hostname
        nodes
            .into_iter()
            .find(|node| self.hostname_eq(&node, &hostname))
            .ok_or(KubernetesInitError::NodeDetectionError)
    }

    /// Initializes the pod reflector and gets its initial state from the
    /// Kubernetes API
    fn initialize_pod_reflector(
        &self,
        node_name: &str,
    ) -> Result<Reflector<Pod>, KubernetesInitError> {
        // Create reflector for pods scheduled on the current node
        let resource = Resource::all::<Pod>();
        let selector: String = format!("spec.nodeName={}", node_name);
        let lp = ListParams::default().fields(&selector).timeout(10);

        self.exec(Reflector::new(self.client().clone(), lp, resource).init())
            .map_err(|_| KubernetesInitError::InitialPodFetchError)
    }

    /// Determines whether the node's hostname is equal to the given hostname
    fn hostname_eq(&self, node: &Node, hostname: &str) -> bool {
        match &Meta::meta(node).labels {
            None => false,
            Some(labels) => match labels.get("kubernetes.io/hostname") {
                Some(hs) => hs == hostname,
                None => {
                    self.shell().verbose(|sh| {
                        sh.warn(format!(
                            "Node lacks 'kubernetes.io/hostname' label: {}",
                            name(node)
                        ))
                    });
                    false
                },
            },
        }
    }

    /// Tries to get all pods that are running on the current node, polling the
    /// Kubernetes API backend to get a fresh list
    fn get_pods(&self) -> Result<Vec<Pod>, Error> {
        self.exec(self.pod_reflector().poll())?;
        let pods = self.exec(self.pod_reflector().state())?;
        Ok(pods.into_iter().collect::<Vec<_>>())
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
                provider: PROVIDER_TYPE,
                metadata: Some(metadata),
                name:     name(pod).to_owned(),
                id:       uid.to_owned(),
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

    /// Gets the group path for the given UID and QoS class, printing out a
    /// message upon the first successful cgroup resolution
    fn get_cgroup(&mut self, uid: &str, qos_class: QualityOfService) -> Option<CgroupPath> {
        let pod_slice = String::from("pod") + uid;
        // Determine if the manager had a resolved group beforehand
        let had_driver = self.cgroup_manager.driver().is_some();

        let cgroup_option: Option<CgroupPath> =
            self.cgroup_manager
                .get_cgroup(&[ROOT_CGROUP, qos_class.into(), &pod_slice]);

        if !had_driver {
            if let Some(driver) = self.cgroup_manager.driver() {
                self.shell()
                    .info(format!("Identified {} as cgroup driver", driver));
            }
        }

        cgroup_option
    }

    /// Gets a reference to the current Kubernetes API client
    fn client(&self) -> &Client {
        self.client
            .as_ref()
            .expect("Client must be initialized: invariant violated")
    }

    /// Gets a reference to the current pod reflector
    fn pod_reflector(&self) -> &Reflector<Pod> {
        self.pod_reflector
            .as_ref()
            .expect("Pod reflector must be initialized: invariant violated")
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
    polled_at:  u128,
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
            polled_at: util::nano_ts(),
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
