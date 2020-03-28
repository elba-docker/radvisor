use std::error;
use std::fmt;

use clap::Clap;

/// CLI version loaded from Cargo, or none if not build with cargo
pub const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
pub struct InvalidMode {
    given: String,
}

impl fmt::Display for InvalidMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid mode: {}", self.given)
    }
}

impl error::Error for InvalidMode {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> { None }
}

impl std::str::FromStr for Mode {
    type Err = InvalidMode;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "docker" => Ok(Mode::Docker),
            "kubernetes" => Ok(Mode::Kubernetes),
            _ => Err(InvalidMode {
                given: s.to_owned(),
            }),
        }
    }
}

/// Auto-parsed CLI options for rAdvisor, generated via clap
#[derive(Clap)]
#[clap(
    version = VERSION.unwrap_or("unknown"),
    author = "Joseph Azevedo and Bhanu Garg",
    about = "Monitors container resource utilization with high granularity and low overhead"
)]
pub struct Opts {
    /// Collection interval between log entries (ms)
    #[clap(
        short = "i",
        long = "interval",
        help = "collection interval between log entries (ms)",
        default_value = "50"
    )]
    pub interval: u64,

    /// Interval between requests to docker to get containers (ms)
    #[clap(
        short = "p",
        long = "poll",
        help = "interval between requests to docker to get containers (ms)",
        default_value = "1000"
    )]
    pub polling_interval: u64,

    /// Target directory to place log files in ({id}.log)
    #[clap(
        short = "d",
        long = "directory",
        help = "target directory to place log files in ({id}.log)",
        default_value = "/var/log/docker/stats"
    )]
    pub directory: String,

    /// Polling provider to use (docker or kubernetes)
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Clap)]

pub enum Command {
    #[clap(about = "runs a collection thread that writes resource statistics to output CSV files")]
    Run {
        #[clap(subcommand)]
        mode: Mode,
    },
}

#[derive(Clap, Clone, Copy)]
pub enum Mode {
    #[clap(
        about = "runs collection using docker as the target backend; collecting stats for each \
                 container"
    )]
    Docker,

    #[clap(
        about = "runs collection using kubernetes as the target backend; collecting stats for \
                 each pod"
    )]
    Kubernetes,
}

/// Parses and resolves defaults for all CLI arguments. Additionally, handles
/// displaying help/version text if specified.
pub fn load() -> Opts {
    // Parse command line arguments (let clap fold in defaults)
    Opts::parse()
}
