use std::vec::Vec;
use std::sync::mpsc::{Receiver, TryIter};
use std::io::{BufWriter, Write, Error, ErrorKind};
use std::path::Path;
use std::fs::File;
use std::fs;
use std::collections::HashMap;
use std::cell::RefCell;

use eventual::Timer;

mod container;

const BUFFER_LENGTH: usize = 64 * 1024;

/// Contains the file handle for the open stats file as well as the buffer to use
/// when writing. `active` is used during difference resolution to mark inactive
/// collectors for teardown/removal.
struct Collector {
    buffer: BufWriter<File>,
    active: bool
}

impl Collector {
    /// Initializes a new collector and opens up a file handle at its corresponding
    /// log filepath
    pub fn create(log_path: &str) -> Result<Collector, Error> {
        let file = File::create(log_path)?;
        Ok(Collector {
            buffer: BufWriter::with_capacity(BUFFER_LENGTH, file),
            active: true,
        })
    }
}

/// Thread function that collects all active containers and updates the active list,
/// if possible
pub fn run(rx: Receiver<Vec<String>>, interval: u32, location: String) -> () {
    let timer = Timer::new();
    let ticks = timer.interval_ms(interval).iter();
    let mut collectors: HashMap<String, RefCell<Collector>> = HashMap::new();

    for _ in ticks {
        // Check to see if update thread has any new ids
        let next_ids_list = get_last(&mut rx.try_iter());
        if next_ids_list.is_some() {
            let new_ids: Vec<String> = next_ids_list.unwrap();
            update_collectors(new_ids, &mut collectors, &location);
        }

        // Loop over active container ids and run collection
        for (id, c) in &collectors {
            let mut collector = c.borrow_mut();
            match container::collect(&id, &mut collector.buffer) {
                Ok(_)    => (),
                Err(err) => {
                    eprintln!("Could not run collector for container {}: {}", id, err);
                },
            };
        }
    }
}

/// Gets the last element of a TryIter iterator returned from Receiver::try_iter
fn get_last<T>(iter: &mut TryIter<T>) -> Option<T> {
    let mut peekable = iter.peekable();
    let mut curr: Option<T> = None;
    while peekable.peek().is_some() {
        curr = peekable.next();
    }
    curr
}

/// Initializes a new collector struct for the given string
fn update_collectors(ids: Vec<String>,
                     collectors: &mut HashMap<String, RefCell<Collector>>,
                     logs_location: &String) -> () {
    // Set active to false on all entries
    for c in collectors.values() {
        let mut c = c.borrow_mut();
        c.active = false;
    }
    
    // Set active to true on all entries with id in list
    for id in ids {
        match collectors.get(&id) {
            Some(collector) => {
                // Already is in collectors map
                let mut c = collector.borrow_mut();
                c.active = true;
            },
            None => {
                // Ensure directories exist before creating the collector
                let result = fs::create_dir_all(logs_location)
                    .and_then(|_| construct_log_path(&id, &logs_location))
                    .and_then(|path| Collector::create(&path));
                match result {
                    Ok(new_collector) => {
                        collectors.insert(id, RefCell::new(new_collector));
                    },
                    Err(err) => {
                        // Back off until next iteration if the container is still running
                        eprintln!("Could not initialize collector for cid {}: {}", id, err);
                    }
                };   
            }
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
        Err(_)   => Err(Error::new(
            ErrorKind::InvalidInput,
            format!("could not create log path in {}", logs_location)
        ))
    }
}