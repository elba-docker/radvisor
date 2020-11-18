use crate::polling::providers::ProviderType;
use std::error;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use byte_unit::{Byte, ByteError};
use clap::{Clap, ValueHint};

type ShellOptions = crate::shell::Options;

/// CLI version loaded from Cargo, or none if not build with cargo
pub const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

lazy_static::lazy_static! {
    /// Authors loaded from Cargo, or none if not build with cargo
    pub static ref AUTHORS: Option<String> = option_env!("CARGO_PKG_AUTHORS")
        .map(|s| s.split(':').collect::<Vec<&str>>().join(", "));
}

/// Parses and resolves defaults for all CLI arguments. Additionally, handles
/// displaying help/version text if specified.
#[allow(clippy::must_use_candidate)]
pub fn load() -> Opts {
    // Parse command line arguments (let clap fold in defaults)
    Opts::parse()
}

/// Auto-parsed CLI options for rAdvisor, generated via clap
#[derive(Clap, Clone)]
#[clap(
    version = VERSION.unwrap_or("unknown"),
    author = AUTHORS.as_deref().unwrap_or("contributors"),
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

#[derive(Clap, Clone)]
pub enum Command {
    #[clap(
        version = VERSION.unwrap_or("unknown"),
        author = AUTHORS.as_deref().unwrap_or("contributors"),
        about = "Runs a collection thread that writes resource statistics to output CSV files"
    )]
    Run(RunCommand),
}

#[derive(Clap, Clone)]
pub struct RunCommand {
    #[clap(subcommand)]
    /// Provider to use to generate collection targets (such as containers/pods)
    pub provider: ProviderType,
}

#[derive(Clap, Clone, Debug, PartialEq)]
pub struct CollectionOptions {
    /// Collection interval between log entries
    #[clap(
        parse(try_from_str = parse_duration),
        name = "interval",
        short = 'i',
        long = "interval",
        default_value = "50ms",
        global = true,
        value_hint = ValueHint::Other
    )]
    pub interval: Duration,

    /// Target directory to place log files in ({id}_{timestamp}.log)
    #[clap(
        parse(from_os_str),
        short = 'd',
        long = "directory",
        default_value = "/var/log/radvisor/stats",
        global = true,
        value_hint = ValueHint::DirPath
    )]
    pub directory: PathBuf,

    /// (optional) Target location to write an buffer flush event log
    #[clap(
        parse(from_os_str),
        short = 'f',
        long = "flush-log",
        global = true,
        value_hint = ValueHint::FilePath
    )]
    pub flush_log: Option<PathBuf>,

    /// Size (in bytes) of the heap-allocated buffer to use to write collection
    /// records in
    #[clap(
        parse(try_from_str = parse_byte),
        short = 'b',
        long = "buffer",
        default_value = "16MiB",
        global = true,
        value_hint = ValueHint::Other
    )]
    pub buffer_size: Byte,
}

#[derive(Clap, Clone, Debug, PartialEq)]
pub struct PollingOptions {
    /// Interval between requests to providers to get targets
    #[clap(
        parse(try_from_str = parse_duration),
        name = "polling-interval",
        short = 'p',
        long = "poll",
        default_value = "1000ms",
        global = true,
        value_hint = ValueHint::Other
    )]
    pub interval: Duration,
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

fn parse_byte(raw: &str) -> Result<Byte, ByteError> { Byte::from_str(raw) }
