use clap::Clap;

const DEFAULT_POLLING_INTERVAL: u64 = 1000;
const DEFAULT_COLLECT_INTERVAL: u64 = 50;
const DEFAULT_LOGS_DIRECTORY: &str = "/var/log/docker/stats";

/// Auto-parsed CLI options for rAdvisor, generated via clap
#[derive(Clap)]
#[clap(
    version = "0.1.1",
    author = "Joseph Azevedo and Bhanu Garg",
    about = "Monitors container resource utilization with high granularity and low overhead"
)]
struct Opts {
    /// Collection interval between log entries (ms)
    #[clap(
        short = "i",
        long = "interval",
        help = "collection interval between log entries (ms)"
    )]
    interval: Option<u64>,

    /// Interval between requests to docker to get containers (ms)
    #[clap(
        short = "p",
        long = "poll",
        help = "interval between requests to docker to get containers (ms)"
    )]
    polling_interval: Option<u64>,

    /// Target directory to place log files in ({id}.log)
    #[clap(
        short = "d",
        long = "directory",
        help = "target directory to place log files in ({id}.log)"
    )]
    directory: Option<String>,
}

/// Resolved version of Opts, with each value having a default folded in
pub struct ResolvedOpts {
    pub interval: u64,
    pub polling_interval: u64,
    pub directory: String,
}

/// Parses and resolves defaults for all CLI arguments. Additionally, handles displaying
/// help/version text if specified.
pub fn load() -> ResolvedOpts {
    // Parse command line arguments
    let opts: Opts = Opts::parse();

    // Extract arguments or get defaults
    let interval = opts.interval.unwrap_or(DEFAULT_COLLECT_INTERVAL);
    let polling_interval = opts.polling_interval.unwrap_or(DEFAULT_POLLING_INTERVAL);
    let directory = opts.directory.unwrap_or(DEFAULT_LOGS_DIRECTORY.to_owned());

    ResolvedOpts {
        interval,
        polling_interval,
        directory,
    }
}
