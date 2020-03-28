use std::error;
use std::fmt;

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

/// An error that occurred during provider initialization/connection check,
/// including a suggestion message printed to stdout
#[derive(Debug)]
pub struct InitializationError {
    pub message: String,
}

impl InitializationError {
    /// Creates a new connection error
    pub fn new(message: &str) -> Self {
        InitializationError {
            message: message.to_owned(),
        }
    }
}

impl fmt::Display for InitializationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "could not connect to the provider resources: {}",
            self.message
        )
    }
}

impl error::Error for InitializationError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}
