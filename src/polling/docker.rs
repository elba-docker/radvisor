use tokio::runtime::Runtime;

// Re-export type to hide implementation
pub type Docker = shiplift::Docker;
pub type Container = shiplift::rep::Container;
pub type Error = shiplift::Error;

pub trait Client {
    fn get_containers(&self) -> Result<Vec<Container>, Error>;
}

impl Client for Docker {
    /// Gets all containers running locally on the docker daemon via shiplift
    fn get_containers(&self) -> Result<Vec<Container>, shiplift::errors::Error> {
        let future = self.containers().list(&Default::default());
        // Block on the future and return the result
        Runtime::new().unwrap().block_on(future)
    }
}

pub fn new() -> Docker {
    Docker::new()
}

/// Determines whether the monitoring process can connect to the dockerd socket
pub fn can_connect() -> bool {
    let docker = Docker::new();
    let future = docker.ping();
    // Block on the future and match on the result
    match Runtime::new().unwrap().block_on(future) {
        Ok(_) => true,
        Err(_) => false,
    }
}
