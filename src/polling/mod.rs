pub mod providers;

use crate::polling::providers::Provider;
use crate::shared::{ContainerMetadata, IntervalWorkerContext};
use crate::timer::{Stoppable, Timer};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;

use colored::*;

/// Thread function that updates the container list each second by default
pub fn run(
    tx: Sender<Vec<ContainerMetadata>>,
    context: IntervalWorkerContext,
    provider: Box<dyn Provider>,
) -> () {
    println!("Beginning statistics collection");

    let (timer, stop_handle) = Timer::new(context.interval);
    let has_stopped = Arc::new(AtomicBool::new(false));

    // Handle SIGINT/SIGTERMs by stopping the timer
    let mut term_rx = context.term_rx;
    let has_stopped_c = Arc::clone(&has_stopped);
    std::thread::spawn(move || {
        term_rx.recv().unwrap();
        println!("Stopping polling");
        stop_handle.stop();
        has_stopped_c.store(true, Ordering::SeqCst);
    });
    // Move to mutable
    let mut provider = provider;

    for _ in timer {
        let containers: Vec<ContainerMetadata> = match provider.fetch() {
            Ok(vec) => vec,
            Err(err) => {
                eprintln!("{}", format!("Fetch error: {}", err).red());
                Vec::with_capacity(0)
            },
        };

        // Make sure the collection hasn't been stopped
        if !has_stopped.load(Ordering::SeqCst) {
            // If sending fails, then stop the collection thread
            if let Err(err) = tx.send(containers) {
                eprintln!(
                    "{}",
                    format!(
                        "Error: could not send polled docker data to collector thread: {}",
                        err
                    )
                    .red()
                );
                break;
            }
        }
    }
}
