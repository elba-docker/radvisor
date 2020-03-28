pub mod collect;
pub mod collector;

use crate::collection::collect::WorkingBuffers;
use crate::collection::collector::Collector;
use crate::shared::{ContainerMetadata, IntervalWorkerContext};
use crate::timer::{Stoppable, Timer};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::vec::Vec;

/// Synchronization status struct used to handle termination and buffer flushing
struct CollectStatus {
    terminating: bool,
    collecting: bool,
}

/// Mutex-protected map of container ids to collector state
type CollectorMap = Arc<Mutex<HashMap<String, RefCell<Collector>>>>;

/// Thread function that collects all active containers and updates the active list,
/// if possible
pub fn run(
    rx: Receiver<Vec<ContainerMetadata>>,
    context: IntervalWorkerContext,
    location: String,
) -> () {
    let (timer, stop_handle) = Timer::new(context.interval);
    let collectors: CollectorMap = Arc::new(Mutex::new(HashMap::new()));

    // Track when the collector is running and when SIGTERM/SIGINT are being handled
    let status_mutex = Arc::new(Mutex::new(CollectStatus {
        terminating: false,
        collecting: false,
    }));

    // Initialize the sigterm/sigint handler
    let collectors_c = Arc::clone(&collectors);
    let status_mutex_c = Arc::clone(&status_mutex);
    let stop_handle_c = stop_handle.clone();
    let mut term_rx = context.term_rx;
    std::thread::spawn(move || {
        term_rx.recv().unwrap();
        let mut status = status_mutex_c.lock().unwrap();
        match status.collecting {
            true => {
                // Set terminating and let the collector thread flush the buffers
                // as it ends collection for the current tick
                status.terminating = true;
            }
            false => {
                // The collection thread is yielding to the sleep; flush the buffers now
                let collectors = collectors_c.lock().unwrap();
                flush_buffers(&collectors);
                stop_handle_c.stop();
            }
        }
    });

    // Re-use working buffers
    let mut working_buffers = WorkingBuffers::new();

    for _ in timer {
        // Update status
        let mut status = status_mutex.lock().unwrap();
        if status.terminating {
            // If already terminating, skip loop iteration
            continue;
        }
        status.collecting = true;
        // Drop the lock early
        drop(status);

        let mut collectors = collectors.lock().unwrap();

        // Check to see if update thread has any new ids
        if let Some(new_containers) = rx.try_iter().last() {
            update_collectors(new_containers, &mut *collectors, &location);
        }

        // Loop over active container ids and run collection
        for (id, c) in collectors.iter() {
            let mut collector = c.borrow_mut();
            match collector.collect(&mut working_buffers) {
                Ok(_) => (),
                Err(err) => {
                    eprintln!(
                        "Error: could not run collector for container {}: {}",
                        id, err
                    );
                }
            };
        }

        // Update status
        let mut status = status_mutex.lock().unwrap();
        if status.terminating {
            // If termination signaled during collection, then the collection thread
            // needs to tear down the buffers
            flush_buffers(&collectors);
            stop_handle.stop();
            break;
        } else {
            // If terminating, then don't signal the end of collecting. Otherwise,
            // end collecting before yielding to the sleep
            status.collecting = false;
        }
    }
}

/// Flushes the buffers for the given
fn flush_buffers(collectors: &HashMap<String, RefCell<Collector>>) {
    println!("Stopping collecting and flushing buffers");

    for (id, c) in collectors.iter() {
        let mut collector = c.borrow_mut();
        if let Err(err) = collector.writer.flush() {
            eprintln!(
                "Error: could not flush buffer on termination for container {}: {}",
                id, err
            );
        }
    }
}

/// Applies the collector update algorithm that finds all inactive container collectors
/// and tears them down. In addition, it will initialize collectors for newly monitored
/// containers
fn update_collectors(
    containers: Vec<ContainerMetadata>,
    collectors: &mut HashMap<String, RefCell<Collector>>,
    logs_location: &String,
) -> () {
    // Set active to false on all entries
    for c in collectors.values() {
        let mut c = c.borrow_mut();
        c.active = false;
    }

    // Set active to true on all entries with id in list
    for container in containers {
        match collectors.get(&container.id) {
            Some(collector) => {
                // Already is in collectors map
                let mut c = collector.borrow_mut();
                c.active = true;
            }
            None => match Collector::create(logs_location, &container) {
                Ok(new_collector) => {
                    collectors.insert(container.id, RefCell::new(new_collector));
                }
                Err(err) => {
                    // Back off until next iteration if the container is still running
                    eprintln!(
                        "Error: could not initialize collector for cid {}: {}",
                        container.id, err
                    );
                }
            },
        }
    }

    // Drop all entries not marked as active
    collectors.retain(|_, value| {
        let mut c = value.borrow_mut();
        if !c.active {
            // Flush the buffer before dropping it
            let _result = c.writer.flush();
        }
        c.active
    })
}
