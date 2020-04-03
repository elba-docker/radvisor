use crate::cli::Opts;
use crate::polling::providers::cgroups::{self, CgroupManager, CgroupPath};
use crate::polling::providers::{FetchError, InitializationError, Provider};
use crate::shared::TargetMetadata;
use crate::shell::Shell;
use crate::util;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::future::Future;
use std::sync::Arc;

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
const POLLING_BLOCK_EXPIRY: u32 = 5u32;

/// String representation for "None"
const NONE_STR: &'static str = "~";

/// Root cgroup for kubernetes pods to fall under
const ROOT_CGROUP: &str = "kubepods";

const PROVIDER_TYPE: &str = "kubernetes";

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
    shell:         Arc<Shell>,
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
    fn initialize(&mut self, opts: &Opts, shell: Arc<Shell>) -> Result<(), InitializationError> {
        shell.status("Initializing", "Kubernetes API provider");

        // Get hostname to try to identify node name
        let hostname = match gethostname().into_string() {
            Ok(hostname) => hostname,
            Err(_) => {
                return Err(InitializationError::new(
                    "Could not retrieve hostname to use for node detection: Invalid string \
                     returned",
                ))
            },
        };

        // Load config
        let config = match self.exec(config::load_kube_config()) {
            Ok(config) => config,
            Err(_) => {
                return Err(InitializationError::new(
                    "Could not load kubernetes config. Make sure the current machine is a part of \
                     a cluster and has the cluster configuration copied to the config directory.",
                ))
            },
        };

        // Make sure cgroups are mounted properly
        if !cgroups::cgroups_mounted_properly() {
            return Err(InitializationError::new(cgroups::INVALID_MOUNT_MESSAGE));
        }

        // Initialize client
        self.client = Some(APIClient::new(config));

        self.invariants = match self.build_invariants(&hostname, opts, shell) {
            Ok(invariants) => Some(invariants),
            Err(err) => return Err(err),
        };

        Ok(())
    }

    fn fetch(&mut self) -> Result<Vec<TargetMetadata>, FetchError> {
        let invariants = self.unwrap();
        let reflector = &invariants.pod_reflector;

        // Poll the reflector to update the pods; ignore errors
        let _ = self.exec(reflector.poll());

        let pods = self.get_pods()?;
        let original_num = pods.len();
        let processed = pods
            .into_iter()
            .filter_map(|pod| {
                let meta = Meta::meta(&pod);
                let uid: &str = match &meta.uid {
                    Some(uid) => &uid,
                    None => {
                        self.shell().verbose(|sh| {
                            sh.warn("Could not get uid for node! This shouldn't happen")
                        });
                        return None;
                    },
                };

                let qos_class: QualityOfService = match QualityOfService::from_pod(&pod) {
                    Some(qos_class) => qos_class,
                    None => {
                        self.shell().verbose(|sh| {
                            sh.warn(format!(
                                "Could not parse quality of service class for pod {}: invalid \
                                 value '{}'",
                                name(&pod),
                                pod.status
                                    .as_ref()
                                    .and_then(|s| s.qos_class.as_deref())
                                    .unwrap_or(NONE_STR)
                            ))
                        });
                        return None;
                    },
                };

                // Construct the cgroup path from the UID and QoS class
                // from the metadata, and make sure it exists/is mounted
                let cgroup_option = self.get_cgroup(&uid, qos_class);

                match cgroup_option {
                    None => None,
                    Some(cgroup) => Some(TargetMetadata {
                        id: String::from(uid),
                        info: self.get_info(&pod),
                        cgroup,
                        provider_type: PROVIDER_TYPE,
                    }),
                }
            })
            .collect::<Vec<_>>();

        self.shell().verbose(|sh| {
            sh.info(format!(
                "Received {} -> {} containers from the Kubernetes API",
                original_num,
                processed.len()
            ))
        });

        Ok(processed)
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
    fn get_info(&mut self, pod: &Pod) -> String {
        let uid: String = String::from(uid(pod));
        let mut info_cache = self.unwrap().info_cache.borrow_mut();
        match info_cache.get(&uid) {
            Some(info) => info.clone(),
            None => {
                let info = self.format_info(&pod);
                info_cache.insert(uid, info.clone());
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
        shell: Arc<Shell>,
    ) -> Result<InitializationInvariants, InitializationError> {
        // Get current node by hostname and store in provider
        let node_name_option: Option<String> = self
            .get_node_by_hostname(&hostname)
            .and_then(|node| Meta::meta(&node).name.as_ref().map(|s| s.clone()));

        let node_name: String = match node_name_option {
            Some(name) => name,
            None => {
                return Err(InitializationError::new(
                    "Could not get the current node via the Kubernetes API. Make sure the current \
                     machine is running its own node.",
                ))
            },
        };

        // Initialize the pod reflector
        let reflector = self.initialize_pod_reflector(&node_name)?;

        Ok(InitializationInvariants {
            shell,
            pod_reflector: reflector,
            info_cache: RefCell::new(LruCache::with_expiry_duration(
                opts.polling_interval * POLLING_BLOCK_EXPIRY,
            )),
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
                        Err(err) => {
                            self.shell().warn(format!(
                                "Could not fetch list of nodes in the Kubernetes cluster: {}",
                                err
                            ));
                            return None;
                        },
                    };

                let nodes_iter = match self.exec(reflector.state()) {
                    Ok(state) => state.into_iter(),
                    Err(err) => {
                        self.shell().warn(format!(
                            "Could not get list of nodes in the Kubernetes cluster: {}",
                            err
                        ));
                        return None;
                    },
                };

                // Try to get a node with the given hostname
                let shell = self.shell();
                return nodes_iter
                    .filter(|node| match &Meta::meta(node).labels {
                        None => false,
                        Some(labels) => match labels.get("kubernetes.io/hostname") {
                            Some(hostname) => hostname == target_hostname,
                            None => {
                                shell.verbose(|sh| {
                                    sh.warn(format!(
                                        "Node lacks 'kubernetes.io/hostname' label: {}",
                                        name(node)
                                    ))
                                });
                                false
                            },
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
                        "Could not get the list of pods running on the current machine. Make sure \
                         the node can access the API.",
                    )),
                }
            },
        }
    }

    /// Tries to get all pods that are running on the current node
    fn get_pods(&self) -> Result<Vec<Pod>, FetchError> {
        let invariants = self.unwrap();
        let pod_reflector = &invariants.pod_reflector;

        match self.exec(pod_reflector.state()) {
            Ok(state) => Ok(state.into_iter().collect::<Vec<_>>()),
            Err(err) => Err(FetchError::new(Some(Box::new(err)))),
        }
    }

    /// Gets the group path for the given UID and QoS class, printing out a
    /// message upon the first successful cgroup resolution
    fn get_cgroup(&mut self, uid: &str, qos_class: QualityOfService) -> Option<CgroupPath> {
        let pod_slice = String::from("pod") + &uid;
        // Determine if the manager had a resolved group beforehand
        let had_driver = self.cgroup_manager.driver().is_some();

        let cgroup_option: Option<CgroupPath> =
            self.cgroup_manager
                .get_cgroup(&[ROOT_CGROUP, &qos_class.to_cgroup(), &pod_slice]);

        if !had_driver {
            if let Some(driver) = self.cgroup_manager.driver() {
                self.shell()
                    .info(format!("Identified {} as cgroup driver", driver));
            }
        }

        cgroup_option
    }

    /// Formats pod info headers to YAML for display at the top of each CSV
    /// file. Uses serde-yaml to serialize the PodInfo struct to YAML
    fn format_info(&self, pod: &Pod) -> String {
        match try_format_info(pod) {
            Ok(yaml) => yaml,
            Err(err) => {
                self.shell().error(format!(
                    "Could not serialize pod info for pod {}: {}",
                    uid(pod),
                    err
                ));
                String::from("")
            },
        }
    }

    /// Gets a reference to the current shell
    fn shell(&self) -> &Shell { &self.unwrap().shell }
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
fn try_format_info(pod: &Pod) -> Result<String, Box<dyn std::error::Error>> {
    let pod_info = PodInfo::new(pod);
    let serde_output = serde_yaml::to_string(&pod_info)?;
    // Remove top ---
    Ok(String::from(serde_output.trim_start_matches("---\n")) + "\n")
}

fn name<'a, O: Meta>(obj: &'a O) -> &'a str { Meta::meta(obj).name.as_deref().unwrap_or(NONE_STR) }

fn uid<'a, O: Meta>(obj: &'a O) -> &'a str { Meta::meta(obj).uid.as_deref().unwrap_or(NONE_STR) }
