use crate::timer::{Stoppable, Timer};
use crate::types::ContainerMetadata;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Error, ErrorKind, Write};
use std::path::Path;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::vec::Vec;

use bus::BusReader;

mod container;

const BUFFER_LENGTH: usize = 64 * 1024;

/// Contains the file handle for the open stats file as well as the buffer to use
/// when writing. `active` is used during difference resolution to mark inactive
/// collectors for teardown/removal.
struct Collector {
    buffer: BufWriter<File>,
    active: bool,
}

impl Collector {
    pub fn create(
        container: &ContainerMetadata,
        logs_location: &String,
    ) -> Result<Collector, Error> {
        // Ensure directories exist before creating the collector
        fs::create_dir_all(logs_location)?;
        let path = construct_log_path(&container.id, logs_location)?;
        let mut collector = Collector::new(&path)?;
        collector.initialize(&container.info)?;
        Ok(collector)
    }

    /// Initializes a new collector and opens up a file handle at its corresponding
    /// log filepath
    fn new(log_path: &str) -> Result<Collector, Error> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(log_path)?;
        Ok(Collector {
            buffer: BufWriter::with_capacity(BUFFER_LENGTH, file),
            active: true,
        })
    }

    /// Writes the given info string and CSV header to the underlying file
    fn initialize(&mut self, info: &String) -> Result<(), Error> {
        self.buffer.write(info.as_bytes())?;
        Ok(())
    }
}

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
    term_rx: BusReader<()>,
    interval: u64,
    location: String,
) -> () {
    let (timer, stop_handle) = Timer::new(Duration::from_millis(interval));
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
    let mut term_rx = term_rx;
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

    for _ in timer {
        // Update status
        let mut status = status_mutex.lock().unwrap();
        if status.terminating {
            // If already termiating, skip loop iteration
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
            match container::collect(&id, &mut collector.buffer) {
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
            // If terminating, then don't signal the end of collecing. Otherwise,
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
        if let Err(err) = collector.buffer.flush() {
            eprintln!(
                "Error: could not flush buffer on termination for container {}: {}",
                id, err
            );
        }
    }
}

/// Initializes a new collector struct for the given string
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
            None => match Collector::create(&container, logs_location) {
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
            let _result = c.buffer.flush();
        }
        c.active
    })
}

/// Constructs the log filepath for the given container id
fn construct_log_path(id: &str, logs_location: &str) -> Result<String, Error> {
    // Construct filename
    let mut filename = id.to_string();
    let suffix = ".log".to_string();
    filename.push_str(&suffix);

    // Join paths
    let base = Path::new(logs_location);
    let filename_path = Path::new(&filename);
    match base.join(filename_path).into_os_string().into_string() {
        Ok(path) => Ok(path),
        Err(_) => Err(Error::new(
            ErrorKind::InvalidInput,
            format!("could not create log path in {}", logs_location),
        )),
    }
}
