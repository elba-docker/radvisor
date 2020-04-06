pub mod collect;
pub mod collector;
pub mod system_info;

use crate::collection::collect::WorkingBuffers;
use crate::collection::collector::Collector;
use crate::shared::{IntervalWorkerContext, TargetMetadata};
use crate::shell::Shell;
use crate::timer::{Stoppable, Timer};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::vec::Vec;

/// Synchronization status struct used to handle termination and buffer flushing
struct CollectStatus {
    terminating: bool,
    collecting:  bool,
}

/// Mutex-protected map of target ids to collector state
type CollectorMap = Arc<Mutex<HashMap<String, RefCell<Collector>>>>;

/// Thread function that collects all active targets and updates the active
/// list, if possible
pub fn run(rx: Receiver<Vec<TargetMetadata>>, context: IntervalWorkerContext, location: PathBuf) {
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
                flush_buffers(&collectors, shell_c);
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

        // Check to see if update thread has any new ids
        if let Some(new_targets) = rx.try_iter().last() {
            context.shell.verbose(|sh| {
                sh.info(format!(
                    "Received {} new targets from the collection thread",
                    new_targets.len()
                ))
            });

            update_collectors(new_targets, &mut *collectors, &location, &context.shell);
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
            flush_buffers(&collectors, context.shell);
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
fn flush_buffers(collectors: &HashMap<String, RefCell<Collector>>, shell: Arc<Shell>) {
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
fn update_collectors(
    targets: Vec<TargetMetadata>,
    collectors: &mut HashMap<String, RefCell<Collector>>,
    logs_location: &PathBuf,
    shell: &Shell,
) {
    // Set active to false on all entries
    for c in collectors.values() {
        let mut c = c.borrow_mut();
        c.active = false;
    }

    // Set active to true on all entries with id in list
    for target in targets {
        match collectors.get(&target.id) {
            Some(collector) => {
                // Already is in collectors map
                let mut c = collector.borrow_mut();
                c.active = true;
            },
            None => {
                shell.verbose(|sh| {
                    sh.info(format!(
                        "Initializing new collector for {} (type={}) at {}",
                        target.id,
                        target.provider_type,
                        logs_location.display()
                    ))
                });

                match Collector::create(logs_location, &target) {
                    Ok(new_collector) => {
                        collectors.insert(target.id, RefCell::new(new_collector));
                    },
                    Err(err) => {
                        // Back off until next iteration if the target is still running
                        shell.error(format!(
                            "Could not initialize collector for target id {}: {}",
                            target.id, err
                        ));
                    },
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

            shell.verbose(|sh| {
                sh.info(format!(
                    "Tearing down old collector for {} (type={})",
                    c.id, c.provider_type
                ))
            });
        }
        c.active
    })
}
