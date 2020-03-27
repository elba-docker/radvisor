use crate::polling::providers::{FetchError, Provider};
use crate::shared::ContainerMetadata;

pub struct Kubernetes {}

impl Kubernetes {
    pub fn new() -> Box<dyn Provider> {
        Box::new(Kubernetes {})
    }
}

impl Provider for Kubernetes {
    fn can_connect(&self) -> bool {
        false
    }

    fn connection_error_message(&self) -> String {
        String::from("")
    }

    fn fetch(&self) -> Result<Vec<ContainerMetadata>, FetchError> {
        Ok(Vec::with_capacity(0))
    }
}
