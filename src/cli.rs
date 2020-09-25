use crate::polling::providers::ProviderType;
use std::error;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use clap::Clap;

type ShellOptions = crate::shell::Options;

/// CLI version loaded from Cargo, or none if not build with cargo
pub const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

/// Parses and resolves defaults for all CLI arguments. Additionally, handles
/// displaying help/version text if specified.
#[allow(clippy::must_use_candidate)]
pub fn load() -> Opts {
    // Parse command line arguments (let clap fold in defaults)
    Opts::parse()
}

/// Auto-parsed CLI options for rAdvisor, generated via clap
#[derive(Clap)]
#[clap(
    version = VERSION.unwrap_or("unknown"),
    author = "Joseph Azevedo and Bhanu Garg",
    about = "Monitors container resource utilization with high granularity and low overhead"
)]
pub struct Opts {
    // Shell output-related options
    #[clap(flatten)]
    pub shell_options: ShellOptions,

    /// Polling provider to use (docker or kubernetes)
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Clap)]
pub struct RunCommand {
    #[clap(help = "provider to use to generate collection targets (such as containers/pods)")]
    pub provider: ProviderType,

    // Polling-related options
    #[clap(flatten)]
    pub polling: PollingOptions,

    // Collection-related options
    #[clap(flatten)]
    pub collection: CollectionOptions,
}

#[derive(Clap)]
pub struct CollectionOptions {
    /// Collection interval between log entries
    #[clap(
        parse(try_from_str = parse_duration),
        short = "i",
        long = "interval",
        help = "collection interval between log entries",
        default_value = "50ms",
        global = true
    )]
    pub interval: Duration,

    /// Target directory to place log files in ({id}_{timestamp}.log)
    #[clap(
        parse(from_os_str),
        short = "d",
        long = "directory",
        help = "target directory to place log files in ({id}_{timestamp}.log)",
        default_value = "/var/log/radvisor/stats",
        global = true
    )]
    pub directory: PathBuf,

    /// Target location to write an event log that contains buffer flushes
    #[clap(
        parse(from_os_str),
        short = "e",
        long = "event-log",
        help = "target location to write an event log that contains buffer flushes",
        global = true
    )]
    pub event_log: Option<PathBuf>,
}

#[derive(Clap)]
pub struct PollingOptions {
    /// Interval between requests to providers to get targets
    #[clap(
        parse(try_from_str = parse_duration),
        short = "p",
        long = "poll",
        help = "interval between requests to providers to get targets",
        default_value = "1000ms",
        global = true
    )]
    pub interval: Duration,
}

pub use command::Command;
mod command {
    // There seems to be a bug around Clap macro expansion that creates unused
    // braces around enum wrapper variants, so we include Command in its own
    // private module
    #![allow(unused_braces)]

    use super::RunCommand;
    use clap::Clap;

    #[derive(Clap)]
    pub enum Command {
        #[clap(
            about = "runs a collection thread that writes resource statistics to output CSV files"
        )]
        Run(RunCommand),
    }
}

#[derive(Debug, Clone)]
pub struct ParseFailure {
    field: String,
    given: String,
}

impl ParseFailure {
    #[must_use]
    pub const fn new(field: String, given: String) -> Self { Self { field, given } }
}

impl fmt::Display for ParseFailure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid {}: {}", self.field, self.given)
    }
}

impl error::Error for ParseFailure {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> { None }
}

fn parse_duration(raw: &str) -> Result<Duration, humantime::DurationError> {
    humantime::Duration::from_str(raw).map(|d| d.into())
}
