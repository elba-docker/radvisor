pub mod collect;
pub mod collector;
pub mod system_info;
pub mod flush;

use crate::cli::CollectionOptions;
use crate::collection::collect::WorkingBuffers;
use crate::collection::collector::Collector;
use crate::collection::flush::{FlushLog};
use crate::shared::{CollectionEvent, CollectionMethod, IntervalWorkerContext};
use crate::shell::Shell;
use crate::timer::{Stoppable, Timer};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

/// Length of the buffer that contains buffer flush events
const EVENT_BUFFER_LENGTH: usize = 8 * 1024;

/// Synchronization status struct used to handle termination and buffer flushing
struct CollectStatus {
    terminating: bool,
    collecting:  bool,
}

/// Mutex-protected map of target ids to collector state
type CollectorMap = Arc<Mutex<HashMap<String, RefCell<Collector>>>>;

/// Thread function that collects all active targets and updates the active
/// list, if possible
pub fn run(
    rx: &Receiver<CollectionEvent>,
    context: IntervalWorkerContext,
    options: &CollectionOptions,
) {
    let location = &options.directory;

    context.shell.status(
        "Beginning",
        format!(
            "statistics collection with {} interval",
            humantime::Duration::from(context.interval)
        ),
    );

    let (timer, stop_handle) = Timer::new(context.interval);
    let collectors: CollectorMap = Arc::new(Mutex::new(HashMap::new()));

    // If we are monitoring events, initialize the event log
    let flush_log = match &options.flush_log {
        Some(log_path) => Some(Arc::new(Mutex::new(FlushLog::new(
            log_path,
            EVENT_BUFFER_LENGTH,
        )))),
        None => None,
    };

    // Track when the collector is running and when SIGTERM/SIGINT are being handled
    let status_mutex = Arc::new(Mutex::new(CollectStatus {
        terminating: false,
        collecting:  false,
    }));

    // Initialize the sigterm/sigint handler
    let collectors_c = Arc::clone(&collectors);
    let status_mutex_c = Arc::clone(&status_mutex);
    let shell_c = Arc::clone(&context.shell);
    let flush_log_c = flush_log.as_ref().map(|c| Arc::clone(c));
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
                flush_buffers(&collectors, &shell_c, flush_log_c);
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
        let flush_log_ref = flush_log.as_ref().map(|r| Arc::clone(r));
        for event in rx.try_iter() {
            handle_event(
                event,
                &mut collectors,
                location,
                &flush_log_ref,
                &context.shell,
            );
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
            let flush_log_ref = flush_log.map(|r| Arc::clone(&r));
            flush_buffers(&collectors, &context.shell, flush_log_ref);
            stop_handle.stop();
            break;
        } else {
            // If terminating, then don't signal the end of collecting. Otherwise,
            // end collecting before yielding to the sleep
            status.collecting = false;
        }
    }
}

/// Flushes the buffers for the given collectors.
/// This should only happen once (during teardown)
fn flush_buffers(
    collectors: &HashMap<String, RefCell<Collector>>,
    shell: &Arc<Shell>,
    flush_log_option: Option<Arc<Mutex<FlushLog>>>,
) {
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

    // Write the event log if it's enabled
    if let Some(flush_log_lock) = flush_log_option {
        let mut flush_log = flush_log_lock.lock().unwrap();
        let path_str = flush_log
            .path
            .clone()
            .into_os_string()
            .into_string()
            .ok()
            .unwrap_or_else(|| String::from("~"));
        match flush_log.write() {
            Ok(count) => shell.info(format!(
                "Wrote {} buffer flush events to {}",
                count, path_str
            )),
            Err(err) => shell.warn(format!(
                "Could not write buffer flush events to {}: {}",
                path_str, err
            )),
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
    flush_log: &Option<Arc<Mutex<FlushLog>>>,
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
                    let flush_log_c = flush_log.as_ref().map(|r| Arc::clone(r));
                    match Collector::create(logs_location, target, &path, flush_log_c) {
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
