use crate::cli::Mode;
use crate::shared::ContainerMetadata;
use std::error;
use std::fmt;

pub mod docker;
pub mod kubernetes;

/// An error that occurred during container metadata fetching
#[derive(Debug)]
pub struct FetchError {
    cause: Option<Box<dyn error::Error>>,
}

impl FetchError {
    /// Creates a new fetch error, optionally using an error instance
    pub fn new(cause: Option<Box<dyn error::Error>>) -> Self {
        FetchError { cause }
    }
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.cause {
            None => write!(f, "could not successfully fetch container metadata list"),
            Some(ref e) => {
                write!(f, "could not successfully fetch container metadata list: ")?;
                e.fmt(f)
            }
        }
    }
}

impl error::Error for FetchError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match &self.cause {
            None => None,
            Some(e) => Some(e.as_ref()),
        }
    }
}

/// A container metadata provider
pub trait Provider {
    /// Performs a connection check to see if the current process can access
    /// the necessary resources to later retrieve lists of container metadata
    fn can_connect(&self) -> bool;
    /// Gets the message for connection errors at the start of the program before
    /// it quits
    fn connection_error_message(&self) -> String;
    /// Attempts to get a list of containers, returning a FetchError if it failed
    fn fetch(&self) -> Result<Vec<ContainerMetadata>, FetchError>;
}

/// Gets the corresponding provider for the CLI polling mode
pub fn for_mode(mode: Mode) -> Box<dyn Provider> {
    match mode {
        Mode::Docker => docker::Docker::new(),
        // TODO fix
        Mode::Kubernetes => docker::Docker::new(),
    }
}
