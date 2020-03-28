use crate::cli::Opts;
use crate::polling::providers::cgroups;
use crate::polling::providers::{FetchError, InitializationError, Provider};
use crate::shared::ContainerMetadata;
use std::cell::RefCell;
use std::future::Future;

use gethostname::gethostname;
use k8s_openapi::api::core::v1::{Node, Pod};
use kube::api::{ListParams, Meta, Resource};
use kube::client::APIClient;
use kube::config;
use kube::runtime::Reflector;
use tokio::runtime::Runtime;

pub struct Kubernetes {
    runtime:    RefCell<Runtime>,
    client:     Option<APIClient>,
    /// Collection of fields that must be initialized after successful provider
    /// initialization occurs. This condition is invariant
    invariants: Option<InitializationInvariants>,
}

pub struct InitializationInvariants {
    pod_reflector: Reflector<Pod>,
}

impl Kubernetes {
    pub fn new() -> Box<dyn Provider> {
        Box::new(Kubernetes {
            runtime:    RefCell::new(Runtime::new().unwrap()),
            client:     None,
            invariants: None,
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

impl Provider for Kubernetes {
    fn initialize(&mut self, _opts: &Opts) -> Option<InitializationError> {
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

        self.invariants = match self.build_invariants(&hostname) {
            Ok(invariants) => Some(invariants),
            Err(err) => {
                return Some(err);
            },
        };

        None
    }

    fn fetch(&mut self) -> Result<Vec<ContainerMetadata>, FetchError> {
        let invariants = self.unwrap();
        let reflector = &invariants.pod_reflector;

        // Poll the reflector to update the pods; ignore errors
        let _ = self.exec(reflector.poll());

        match self.get_pods() {
            None => Err(FetchError::new(None)),
            Some(pods) => {
                // TODO implement
                // 2. get all IDs from pod metadata, build 3 paths to look at for them
                // 3. look at filesystem for the cgroup slices, see if 1/3 of them exist
                //    - cgroup slice: kubepods-{1}.slice/kubepods-{1}-pod{2}.slice where 1 in
                //      ["besteffort", "burstable", "guaranteed"] 2 = pod.ID.replace("-", "_")
                // 4. if the slice exists by using IO call, write metadata info to string,
                //    store existing cgroup, pass to collection thread
                Ok(Vec::with_capacity(0))
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

    /// Creates instances of everything that will be guaranteed to be defined
    /// after the initialization phase
    fn build_invariants(
        &self,
        hostname: &str,
    ) -> Result<InitializationInvariants, InitializationError> {
        // Get current node by hostname and store in provider
        let node_name: String = match self.get_node_by_hostname(&hostname) {
            Some(node) => Meta::meta(&node).name.as_ref().unwrap().clone(),
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
