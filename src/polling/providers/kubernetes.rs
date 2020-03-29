use crate::cli::Opts;
use crate::polling::providers::cgroups::{self, CgroupManager};
use crate::polling::providers::{FetchError, InitializationError, Provider};
use crate::shared::TargetMetadata;
use crate::util;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::future::Future;
use std::time::Duration;

use colored::*;
use gethostname::gethostname;
use k8s_openapi::api::core::v1::{Node, Pod};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Time;
use kube::api::{ListParams, Meta, Resource};
use kube::client::APIClient;
use kube::config;
use kube::runtime::Reflector;
use lru_time_cache::LruCache;
use serde::Serialize;
use serde_yaml;
use tokio::runtime::Runtime;

/// Number of polling blocks that need to elapse before container info strings
/// are evicted from the time-based LRU cache
const POLLING_BLOCK_EXPIRY: u64 = 5;

/// Root cgroup for kubernetes pods to fall under
const ROOT_CGROUP: &str = "kubepods";

pub struct Kubernetes {
    runtime:        RefCell<Runtime>,
    client:         Option<APIClient>,
    cgroup_manager: CgroupManager,
    /// Collection of fields that must be initialized after successful provider
    /// initialization occurs. This condition is invariant
    invariants:     Option<InitializationInvariants>,
}

pub struct InitializationInvariants {
    pod_reflector: Reflector<Pod>,
    info_cache:    RefCell<LruCache<String, String>>,
}

impl Kubernetes {
    pub fn new() -> Box<dyn Provider> {
        Box::new(Kubernetes {
            runtime:        RefCell::new(Runtime::new().unwrap()),
            client:         None,
            invariants:     None,
            cgroup_manager: CgroupManager::new(),
        })
    }

    /// Unwraps the inner initialization invariants option field, panicking if
    /// the invariant has not been satisfied
    fn unwrap(&self) -> &InitializationInvariants {
        self.invariants.as_ref().expect(
            "Invariant violated: Kubernetes inner values must be non-None after initialization",
        )
    }
}

/// Quality of service classes for pods. For more information, see
/// [the Kubernetes docs](https://kubernetes.io/docs/tasks/configure-pod-container/quality-service-pod/#qos-classes)
enum QualityOfService {
    BestEffort,
    Guaranteed,
    Burstable,
}

impl QualityOfService {
    /// Attempts to extract the quality of service value from a pod's status
    fn from_pod(pod: &Pod) -> Option<Self> {
        let qos_class: Option<&String> = match &pod.status {
            Some(status) => status.qos_class.as_ref(),
            None => None,
        };

        qos_class.and_then(|raw_str| match &raw_str.to_lowercase()[..] {
            "besteffort" => Some(QualityOfService::BestEffort),
            "guaranteed" => Some(QualityOfService::Guaranteed),
            "burstable" => Some(QualityOfService::Burstable),
            _ => None,
        })
    }

    /// Converts the quality of service to its cgroup slice representation
    fn to_cgroup(&self) -> String {
        String::from(match self {
            QualityOfService::BestEffort => "besteffort",
            QualityOfService::Guaranteed => "guaranteed",
            QualityOfService::Burstable => "burstable",
        })
    }
}

impl Provider for Kubernetes {
    fn initialize(&mut self, opts: &Opts) -> Option<InitializationError> {
        println!("Initializing Kubernetes API provider");

        // Get hostname to try to identify node name
        let hostname = match gethostname().into_string() {
            Ok(hostname) => hostname,
            Err(_) => {
                return Some(InitializationError::new(
                    "Could not retrieve hostname to use for node detection: Invalid string \
                     returned",
                ));
            },
        };

        // Load config
        let config = match self.exec(config::load_kube_config()) {
            Ok(config) => config,
            Err(_) => {
                return Some(InitializationError::new(
                    "Could not load kubernetes config. Make sure the current machine is a part of \
                     a\ncluster and has the cluster configuration copied to the config directory.",
                ));
            },
        };

        // Make sure cgroups are mounted properly
        if !cgroups::cgroups_mounted_properly() {
            return Some(InitializationError::new(cgroups::INVALID_MOUNT_MESSAGE));
        }

        // Initialize client
        self.client = Some(APIClient::new(config));

        self.invariants = match self.build_invariants(&hostname, opts) {
            Ok(invariants) => Some(invariants),
            Err(err) => {
                return Some(err);
            },
        };

        None
    }

    fn fetch(&mut self) -> Result<Vec<TargetMetadata>, FetchError> {
        let invariants = self.unwrap();
        let reflector = &invariants.pod_reflector;

        // Poll the reflector to update the pods; ignore errors
        let _ = self.exec(reflector.poll());

        match self.get_pods() {
            None => Err(FetchError::new(None)),
            Some(pods) => {
                Ok(pods
                    .into_iter()
                    .filter_map(|pod| {
                        let meta = Meta::meta(&pod);
                        let uid: &str = match &meta.uid {
                            None => return None,
                            Some(uid) => &uid,
                        };

                        let pod_slice = String::from("pod") + &uid;
                        let qos_class: QualityOfService = match QualityOfService::from_pod(&pod) {
                            None => return None,
                            Some(qos_class) => qos_class,
                        };

                        // Construct the cgroup path from the UID and QoS class
                        // from the metadata, and make sure it exists/is mounted
                        let cgroup_option: Option<String> = self.cgroup_manager.get_cgroup(&[
                            ROOT_CGROUP,
                            &qos_class.to_cgroup(),
                            &pod_slice,
                        ]);

                        match cgroup_option {
                            None => None,
                            Some(cgroup) => Some(TargetMetadata {
                                id: String::from(uid),
                                info: self.get_info(&pod, &cgroup),
                                cgroup,
                            }),
                        }
                    })
                    .collect::<Vec<_>>())
            },
        }
    }
}

impl Kubernetes {
    /// Executes a future on the internal runtime, blocking the current thread
    /// until it completes
    fn exec<F: Future>(&self, future: F) -> F::Output {
        let mut runtime = self.runtime.borrow_mut();
        runtime.block_on(future)
    }

    /// Try to get info from LRU cache before re-serializing it
    fn get_info(&mut self, pod: &Pod, cgroup: &str) -> String {
        let meta = Meta::meta(pod);
        let uid_default = String::from("");
        let uid: &String = meta.uid.as_ref().unwrap_or(&uid_default);
        let mut info_cache = self.unwrap().info_cache.borrow_mut();
        match info_cache.get(uid) {
            Some(info) => info.clone(),
            None => {
                let info = format_info(&pod, &cgroup);
                info_cache.insert(uid.clone(), info.clone());
                info
            },
        }
    }

    /// Creates instances of everything that will be guaranteed to be defined
    /// after the initialization phase
    fn build_invariants(
        &self,
        hostname: &str,
        opts: &Opts,
    ) -> Result<InitializationInvariants, InitializationError> {
        // Get current node by hostname and store in provider
        let node_name_option: Option<String> = self
            .get_node_by_hostname(&hostname)
            .and_then(|node| Meta::meta(&node).name.as_ref().map(|s| s.clone()));

        let node_name: String = match node_name_option {
            Some(name) => name,
            None => {
                return Err(InitializationError::new(
                    "Could not get the current node via the Kubernetes API.\nMake sure the \
                     current machine is running its own node.",
                ))
            },
        };

        // Initialize the pod reflector
        let reflector = self.initialize_pod_reflector(&node_name)?;

        Ok(InitializationInvariants {
            pod_reflector: reflector,
            info_cache:    RefCell::new(LruCache::with_expiry_duration(Duration::from_millis(
                opts.polling_interval * POLLING_BLOCK_EXPIRY,
            ))),
        })
    }

    /// Tries to get a Node from the kubernetes API by the given hostname
    fn get_node_by_hostname(&self, target_hostname: &str) -> Option<Node> {
        match &self.client {
            None => None,
            Some(client) => {
                let resource = Resource::all::<Node>();
                let lp = ListParams::default().timeout(10);
                let reflector: Reflector<Node> =
                    match self.exec(Reflector::new(client.clone(), lp, resource).init()) {
                        Ok(reflector) => reflector,
                        Err(_) => {
                            return None;
                        },
                    };

                let nodes_iter = match self.exec(reflector.state()) {
                    Ok(state) => state.into_iter(),
                    Err(_) => {
                        return None;
                    },
                };

                // Try to get a node with the given hostname
                return nodes_iter
                    .filter(|o| match &Meta::meta(o).labels {
                        None => false,
                        Some(labels) => match labels.get("kubernetes.io/hostname") {
                            None => false,
                            Some(hostname) => hostname == target_hostname,
                        },
                    })
                    .nth(0);
            },
        }
    }

    /// Initializes the pod reflector and gets its initial state from the
    /// Kubernetes API
    fn initialize_pod_reflector(
        &self,
        node_name: &str,
    ) -> Result<Reflector<Pod>, InitializationError> {
        match &self.client {
            None => Err(InitializationError::new(
                "Client was not initialized properly: invariant violated.",
            )),
            Some(client) => {
                // Create reflector for pods scheduled on the current node
                let resource = Resource::all::<Pod>();
                let selector: String = format!("spec.nodeName={}", node_name);
                let lp = ListParams::default().fields(&selector).timeout(10);

                match self.exec(Reflector::new(client.clone(), lp, resource).init()) {
                    Ok(reflector) => Ok(reflector),
                    Err(_) => Err(InitializationError::new(
                        "Could not get the list of pods running on the current machine.\n Make \
                         sure the node can access the API.",
                    )),
                }
            },
        }
    }

    /// Tries to get all pods that are running on the current node
    fn get_pods(&self) -> Option<Vec<Pod>> {
        let invariants = self.unwrap();
        let pod_reflector = &invariants.pod_reflector;

        let pods_iter = match self.exec(pod_reflector.state()) {
            Ok(state) => state.into_iter(),
            Err(_) => {
                return None;
            },
        };

        Some(pods_iter.collect::<Vec<_>>())
    }
}

/// Pod info struct that gets included with each log file
#[derive(Clone, Debug, PartialEq, Serialize)]
struct PodInfo<'a> {
    uid:        &'a Option<String>,
    name:       &'a Option<String>,
    created_at: &'a Option<Time>,
    labels:     &'a Option<BTreeMap<String, String>>,
    namespace:  &'a Option<String>,
    node_name:  &'a Option<String>,
    host_ip:    &'a Option<String>,
    phase:      &'a Option<String>,
    pod_ip:     &'a Option<String>,
    qos_class:  &'a Option<String>,
    started_at: &'a Option<Time>,
    cgroup:     &'a str,
    polled_at:  u128,
}

impl<'a> PodInfo<'a> {
    /// Attempts to extract all state/metadata from the given pod, and collects
    /// it in a single pod info struct
    fn new(p: &'a Pod, cgroup: &'a str) -> Self {
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

        let (host_ip, phase, pod_ip, qos_class, started_at) = match p.status.as_ref() {
            None => (&None, &None, &None, &None, &None),
            Some(status) => (
                &status.host_ip,
                &status.phase,
                &status.pod_ip,
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
            pod_ip,
            qos_class,
            started_at,
            cgroup,
            polled_at: util::nano_ts(),
        }
    }
}

/// Formats pod info headers to YAML for display at the top of each CSV file.
/// Uses serde-yaml to serialize the PodInfo struct to YAML
fn format_info(pod: &Pod, cgroup: &str) -> String {
    let pod_info = PodInfo::new(pod, cgroup);
    match serde_yaml::to_string(&pod_info) {
        // Remove top ---
        Ok(yaml) => String::from(yaml.trim_start_matches("---\n")) + "\n",
        Err(err) => {
            let uid_default = String::from("");
            let uid: &String = Meta::meta(pod).uid.as_ref().unwrap_or(&uid_default);
            eprintln!(
                "Could not serialize pod info for pod {}:\n{}",
                uid.red(),
                format!("{:?}", err).red()
            );
            String::from("")
        },
    }
}
