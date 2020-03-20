use std::vec::Vec;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;

use jod_thread;
use clap::{Arg, App};

mod docker;
mod collect;

const DEFAULT_POLLING_INTERVAL: u32 = 1000;
const DEFAULT_COLLECT_INTERVAL: u32 = 50;

/// Parses CLI args, performs a health check to the docker daemon, and then
/// spawns two worker threads for:
///   a. polling the docker daemon and
///   b. collecting data on all active containers
fn main() {
    // Parse command line arguments
    let matches = App::new("rAdvisor")
        .version("0.1.0")
        .author("Joseph Azevedo and Bhanu Garg")
        .about("Monitors container resource utilization with high granularity and low overhead")
        .arg(Arg::with_name("interval")
                .short('i')
                .long("interval")
                .takes_value(true)
                .help("collection interval between log entries (ms)"))
        .arg(Arg::with_name("polling interval")
                .short('p')
                .long("poll")
                .takes_value(true)
                .help("interval between requests to docker to get containers (ms)"))
        .arg(Arg::with_name("directory")
                .short('d')
                .long("dir")
                .takes_value(true)
                .help("target directory to place log files in ({id}.log)"))
        .get_matches();
    
    // Extract arguments or get defaults
    let interval = matches.value_of("interval")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(DEFAULT_COLLECT_INTERVAL);
    let polling_interval = matches.value_of("polling interval")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(DEFAULT_POLLING_INTERVAL);
    let logs_directory = matches.value_of("directory")
        .unwrap_or("/var/logs/docker/stats")
        .to_owned();

    // Determine if the current process can connect to the dockerd daemon
    if !docker::can_connect() {
        eprintln!("Could not connect to the docker socket. Are you running radvisor as root?");
        eprintln!("If running on a non-standard URI, set DOCKER_HOST to the correct URL.");
        std::process::exit(1)
    }

    let (tx, rx): (Sender<Vec<String>>, Receiver<Vec<String>>) = mpsc::channel();
    let _update_thread: jod_thread::JoinHandle<()> = jod_thread::spawn(move || {
        docker::run(tx, polling_interval)
    });
    let _collect_thread: jod_thread::JoinHandle<()> = jod_thread::spawn(move || {
        collect::run(rx, interval, logs_directory)
    });
}
