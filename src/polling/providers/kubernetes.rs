use crate::polling::providers::{ConnectionError, FetchError, Provider};
use crate::shared::ContainerMetadata;
use std::cell::RefCell;
use std::future::Future;

use gethostname::gethostname;
use k8s_openapi::api::core::v1::Node;
use kube::api::{ListParams, Meta, Resource};
use kube::client::APIClient;
use kube::config;
use kube::runtime::Reflector;
use tokio::runtime::Runtime;

pub struct Kubernetes {
    runtime: RefCell<Runtime>,
    client: Option<APIClient>,
    node_name: Option<String>,
}

impl Kubernetes {
    pub fn new() -> Box<dyn Provider> {
        Box::new(Kubernetes {
            // Use single-threaded scheduler
            runtime: RefCell::new(Runtime::new().unwrap()),
            client: None,
            node_name: None,
        })
    }
}

impl Provider for Kubernetes {
    fn try_connect(&mut self) -> Option<ConnectionError> {
        // Get hostname to try to identify node name
        let hostname = match gethostname().into_string() {
            Ok(hostname) => hostname,
            Err(_) => {
                return Some(ConnectionError::new("Could not retrieve hostname to use for node detection: Invalid string returned"));
            }
        };

        // Load config
        let config = match self.exec(config::load_kube_config()) {
            Ok(config) => config,
            Err(_) => {
                return Some(ConnectionError::new("Could not load kubernetes config. Make sure the current machine is a part of a\ncluster and has the cluster configuration copied to the config directory."));
            }
        };

        // Initialize client
        self.client = Some(APIClient::new(config));

        // Get current node by hostname
        self.node_name =
            match self.get_node_by_hostname(&hostname) {
                Some(node) => Some(Meta::meta(&node).name.as_ref().unwrap().clone()),
                None => return Some(ConnectionError::new(
                    "Could not get the current node via the Kubernetes API.\nMake sure the current machine is running its own node.",
                )),
            };

        println!("Current node: {}", self.node_name.as_ref().unwrap());

        Some(ConnectionError::new("found node successfully"))
    }

    fn fetch(&mut self) -> Result<Vec<ContainerMetadata>, FetchError> {
        Ok(Vec::with_capacity(0))
    }
}

impl Kubernetes {
    /// Executes a future on the single-threaded runtime, blocking the current thread
    /// until it completes
    fn exec<F: Future>(&self, future: F) -> F::Output {
        let mut runtime = self.runtime.borrow_mut();
        runtime.block_on(future)
    }

    /// Tries to get a Node from the kubernetes API by the given hostname
    fn get_node_by_hostname(&self, target_hostname: &str) -> Option<Node> {
        if let Some(client) = &self.client {
            let resource = Resource::all::<Node>();
            let lp = ListParams::default().timeout(10);
            let reflector: Reflector<Node> =
                match self.exec(Reflector::new(client.clone(), lp, resource).init()) {
                    Ok(reflector) => reflector,
                    Err(_) => {
                        return None;
                    }
                };

            let nodes_iter = match self.exec(reflector.state()) {
                Ok(state) => state.into_iter(),
                Err(_) => {
                    return None;
                }
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
        }

        None
    }
}
