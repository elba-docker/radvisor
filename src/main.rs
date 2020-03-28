#![feature(const_generics)]
#![allow(incomplete_features)]

mod cli;
mod collection;
mod polling;
mod shared;
mod timer;
mod util;

use crate::cli::{Command, Opts};
use crate::polling::providers::{self, Provider};
use crate::shared::{ContainerMetadata, IntervalWorkerContext};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::vec::Vec;

use bus::Bus;
use colored::*;

/// Parses CLI args and runs the correct procedure depending on the subcommand
fn main() {
    // Parse command line arguments
    let opts: Opts = cli::load();

    match opts.command {
        Command::Run { mode } => {
            // Resolve container metadata provider
            let mut provider: Box<dyn Provider> = providers::for_mode(mode);

            // Determine if the current process can connect to the provider source
            if let Some(err) = provider.initialize(&opts) {
                eprintln!("{}", err.message.red().bold());
                std::process::exit(1);
            }

            run(opts, provider);
        },
    }
}

/// Bootstraps the two worker threads, preparing the necessary communication
/// between them
fn run(opts: Opts, provider: Box<dyn Provider>) -> () {
    // Used to send container metadata lists from the polling thread to the
    // collection thread
    let (tx, rx): (
        Sender<Vec<ContainerMetadata>>,
        Receiver<Vec<ContainerMetadata>>,
    ) = mpsc::channel();

    // Handle termination by broadcasting to all worker threads
    let term_bus = Arc::new(Mutex::new(Bus::new(1)));
    let term_bus_c = Arc::clone(&term_bus);
    ctrlc::set_handler(move || handle_termination(&term_bus_c))
        .expect("Error: could not create SIGINT handler");

    // Create the thread worker contexts using the term bus lock
    let mut term_bus_handle = term_bus.lock().unwrap();
    let polling_context = IntervalWorkerContext {
        interval: Duration::from_millis(opts.polling_interval),
        term_rx:  term_bus_handle.add_rx(),
    };
    let collection_context = IntervalWorkerContext {
        interval: Duration::from_millis(opts.interval),
        term_rx:  term_bus_handle.add_rx(),
    };
    drop(term_bus_handle);

    // Unwrap the directory from the resolved opts to prevent the move of opts
    let directory = opts.directory;

    // Spawn both threads
    let polling_thread: thread::JoinHandle<()> =
        thread::spawn(move || polling::run(tx, polling_context, provider));
    let collection_thread: thread::JoinHandle<()> =
        thread::spawn(move || collection::run(rx, collection_context, directory));

    // Join the threads, which automatically exit upon termination
    collection_thread
        .join()
        .expect("Error: collection thread resulted in panic");
    polling_thread
        .join()
        .expect("Error: polling thread resulted in panic");
    println!("Exiting");
}

/// Handles program termination by broadcasting an empty message on a special
/// termination bus that each thread listens to
fn handle_termination(bus_lock: &Arc<Mutex<Bus<()>>>) -> ! {
    let mut bus = bus_lock.lock().unwrap();
    bus.broadcast(());

    // Try again to tear down the program
    thread::sleep(Duration::from_millis(2000));
    println!("Trying again...");
    bus.broadcast(());
    thread::sleep(Duration::from_millis(1000));
    println!("Could not shut down gracefully");
    std::process::exit(2);
}
