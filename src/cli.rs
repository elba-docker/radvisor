use crate::polling::providers::ProviderType;
use crate::shell::ColorMode;
use std::error;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use clap::Clap;

/// CLI version loaded from Cargo, or none if not build with cargo
pub const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
pub struct ParseFailure {
    field: String,
    given: String,
}

impl ParseFailure {
    pub fn new(field: String, given: String) -> Self { ParseFailure { field, given } }
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

/// Auto-parsed CLI options for rAdvisor, generated via clap
#[derive(Clap)]
#[clap(
    version = VERSION.unwrap_or("unknown"),
    author = "Joseph Azevedo and Bhanu Garg",
    about = "Monitors container resource utilization with high granularity and low overhead"
)]
pub struct Opts {
    #[clap(
        short = "q",
        long = "quiet",
        help = "whether to run in quiet mode (minimal output)",
        global = true
    )]
    pub quiet: bool,

    #[clap(
        short = "v",
        long = "verbose",
        help = "whether to run in verbose mode (maximum output)",
        global = true
    )]
    pub verbose: bool,

    /// Mode of the color output of the process
    #[clap(
        short = "c",
        long = "color",
        help = "color display mode for stdout/stderr output",
        default_value = "auto",
        global = true
    )]
    pub color_mode: ColorMode,

    /// Polling provider to use (docker or kubernetes)
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Clap)]
pub struct RunCommand {
    #[clap(subcommand)]
    pub mode: ProviderType,

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

    /// Interval between requests to providers to get targets
    #[clap(
        parse(try_from_str = parse_duration),
        short = "p",
        long = "poll",
        help = "interval between requests to providers to get targets",
        default_value = "1000ms",
        global = true
    )]
    pub polling_interval: Duration,

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
}

#[derive(Clap)]
pub enum Command {
    #[clap(about = "runs a collection thread that writes resource statistics to output CSV files")]
    Run(RunCommand),
}

/// Parses and resolves defaults for all CLI arguments. Additionally, handles
/// displaying help/version text if specified.
pub fn load() -> Opts {
    // Parse command line arguments (let clap fold in defaults)
    Opts::parse()
}
