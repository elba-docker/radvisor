use crate::types::ContainerMetadata;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::vec::Vec;

use bus::Bus;
use cli::ResolvedOpts;

mod collect;
mod docker;
mod cli;
mod timer;
mod types;
mod util;

/// Parses CLI args, performs a health check to the docker daemon, and then
/// spawns two worker threads for:
///   a. polling the docker daemon and
///   b. collecting data on all active containers
fn main() {
    // Parse command line arguments
    let opts: ResolvedOpts = cli::load();

    // Determine if the current process can connect to the dockerd daemon
    if !docker::can_connect() {
        eprintln!("Could not connect to the docker socket. Are you running rAdvisor as root?");
        eprintln!("If running at a non-standard URL, set DOCKER_HOST to the correct URL.");
        std::process::exit(1)
    }

    run(opts);
}

/// Bootstraps the two worker threads, preparing the neccessary communication between them
fn run(opts: ResolvedOpts) -> () {
    // Used to send container metadata lists from the polling thread to the collection thread
    let (tx, rx): (
        Sender<Vec<ContainerMetadata>>,
        Receiver<Vec<ContainerMetadata>>,
    ) = mpsc::channel();

    // Handle termination by broadcasting to all worker threads
    let term_bus_lock = Arc::new(Mutex::new(Bus::new(1)));
    let mut term_bus = term_bus_lock.lock().unwrap();
    let update_handle = term_bus.add_rx();
    let collect_handle = term_bus.add_rx();
    drop(term_bus);

    ctrlc::set_handler(move || {
        let mut term_bus = term_bus_lock.lock().unwrap();
        term_bus.broadcast(());
    })
    .expect("Error: could not create SIGINT handler");

    let polling_interval = opts.polling_interval;
    let interval = opts.interval;
    let directory = opts.directory;

    // Create both threads
    let update_thread: thread::JoinHandle<()> =
        thread::spawn(move || docker::run(tx, update_handle, polling_interval));
    let collect_thread: thread::JoinHandle<()> =
        thread::spawn(move || collect::run(rx, collect_handle, interval, directory));

    // Join the threads, which automatically exit upon termination
    collect_thread
        .join()
        .expect("Error: collect thread resulted in panic");
    update_thread
        .join()
        .expect("Error: polling thread resulted in panic");
    println!("Exiting");
}
