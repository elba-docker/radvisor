pub mod collect;
pub mod collector;
pub mod system_info;

use crate::collection::collect::WorkingBuffers;
use crate::collection::collector::Collector;
use crate::shared::{CollectionEvent, CollectionMethod, IntervalWorkerContext};
use crate::shell::Shell;
use crate::timer::{Stoppable, Timer};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

/// Synchronization status struct used to handle termination and buffer flushing
struct CollectStatus {
    terminating: bool,
    collecting:  bool,
}

/// Mutex-protected map of target ids to collector state
type CollectorMap = Arc<Mutex<HashMap<String, RefCell<Collector>>>>;

/// Thread function that collects all active targets and updates the active
/// list, if possible
pub fn run(rx: &Receiver<CollectionEvent>, context: IntervalWorkerContext, location: &Path) {
    context.shell.status(
        "Beginning",
        format!(
            "statistics collection with {} interval",
            humantime::Duration::from(context.interval)
        ),
    );

    let (timer, stop_handle) = Timer::new(context.interval);
    let collectors: CollectorMap = Arc::new(Mutex::new(HashMap::new()));

    // Track when the collector is running and when SIGTERM/SIGINT are being handled
    let status_mutex = Arc::new(Mutex::new(CollectStatus {
        terminating: false,
        collecting:  false,
    }));

    // Initialize the sigterm/sigint handler
    let collectors_c = Arc::clone(&collectors);
    let status_mutex_c = Arc::clone(&status_mutex);
    let shell_c = Arc::clone(&context.shell);
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
                shell_c.verbose(|sh| {
                    sh.info(
                        "Currently collecting: stopping & flushing buffers at the end of the next \
                         collector tick",
                    )
                });
            },
            false => {
                shell_c.verbose(|sh| {
                    sh.info("Currently yielding: stopping & flushing buffers right now")
                });

                // The collection thread is yielding to the sleep; flush the buffers now
                let collectors = collectors_c.lock().unwrap();
                flush_buffers(&collectors, &shell_c);
                stop_handle_c.stop();
            },
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

        // Check to see if update thread has sent any new start/stop events
        for event in rx.try_iter() {
            handle_event(event, &mut collectors, location, &context.shell);
        }

        // Loop over active target ids and run collection
        for (id, c) in collectors.iter() {
            let mut collector = c.borrow_mut();
            match collector.collect(&mut working_buffers) {
                Ok(_) => (),
                Err(err) => {
                    context.shell.error(format!(
                        "Could not run collector for target {}: {}",
                        id, err
                    ));
                },
            };
        }

        // Update status
        let mut status = status_mutex.lock().unwrap();
        if status.terminating {
            context.shell.verbose(|sh| {
                sh.info(
                    "Received termination flag from term handler thread; stopping and flushing \
                     buffers now",
                )
            });

            // If termination signaled during collection, then the collection thread
            // needs to tear down the buffers
            flush_buffers(&collectors, &context.shell);
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
fn flush_buffers(collectors: &HashMap<String, RefCell<Collector>>, shell: &Arc<Shell>) {
    shell.status("Stopping", "collecting and flushing buffers");

    for (id, c) in collectors.iter() {
        let mut collector = c.borrow_mut();
        if let Err(err) = collector.writer.flush() {
            shell.warn(format!(
                "Could not flush buffer on termination for target {}: {}",
                id, err
            ));
        }
    }
}

/// Applies the collector update algorithm that finds all inactive target
/// collectors and tears them down. In addition, it will initialize collectors
/// for newly monitored targets
fn handle_event(
    event: CollectionEvent,
    collectors: &mut HashMap<String, RefCell<Collector>>,
    logs_location: &Path,
    shell: &Shell,
) {
    match event {
        CollectionEvent::Start { target, method } => {
            shell.verbose(|sh| {
                sh.info(format!(
                    "Received start event for target '{}' from the collection thread",
                    target.name
                ))
            });

            match method {
                CollectionMethod::LinuxCgroups(path) => {
                    let id = target.id.clone();
                    match Collector::create(logs_location, target, &path) {
                        Ok(new_collector) => {
                            collectors.insert(id, RefCell::new(new_collector));
                        },
                        Err(err) => {
                            // Back off until next iteration if the target is still running
                            shell.error(format!(
                                "Could not initialize collector for target id {}: {}",
                                id, err
                            ));
                        },
                    }
                },
            }
        },
        CollectionEvent::Stop(id) => {
            shell.verbose(|sh| {
                sh.info(format!(
                    "Received stop event for target '{}' from the collection thread",
                    collectors
                        .get(&id)
                        .map(|c| c.borrow().target.name.clone())
                        .as_ref()
                        .unwrap_or(&id)
                ))
            });

            let collector = collectors.remove(&id);
            drop(collector);
        },
    }
}
