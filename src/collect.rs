use std::vec::Vec;
use std::sync::mpsc::Receiver;
use std::io::BufWriter;
use std::fs::File;
use std::collections::HashMap;
use std::cell::RefCell;
// use std::mem::drop;
// use std::io::Write;

use eventual::Timer;

/// Contains the file handle for the open stats file as well as the buffer to use
/// when writing. `active` is used during difference resolution to mark inactive
/// collectors for teardown/removal.
struct Collector<'f> {
    file: &'f File,
    buffer: BufWriter<File>,
    active: bool
}

const BUFFER_LENGTH: usize = 64 * 1024;

impl Collector<'_> {
    /// Initializes a new collector and opens up a file handle at its corresponding
    /// log filepath
    pub fn create(id: &String) -> Result<Collector, std::io::Error> {
        let file = File::create(construct_log_path(id))?;
        Ok(Collector {
            file: &file,
            buffer: BufWriter::with_capacity(BUFFER_LENGTH, file),
            active: true,
        })
    }
}

/// Thread function that collects all active containers and updates the active list,
/// if possible
pub fn run(rx: Receiver<Vec<String>>, interval: u32) -> () {
    let timer = Timer::new();
    let ticks = timer.interval_ms(interval).iter();
    let mut collectors: HashMap<String, RefCell<Collector>> = HashMap::new();

    for _ in ticks {
        // Check to see if update thread has any new ids
        let new_ids_result = rx.try_recv();
        if new_ids_result.is_ok() {
            let new_ids: Vec<String> = new_ids_result.unwrap();
            // Set active to false on all entries
            for c in collectors.values() {
                let mut c = c.borrow_mut();
                c.active = false;
            }
            
            for id in new_ids {
                match collectors.get(&id) {
                    Some(collector) => {
                        // Already is in collectors map
                        let mut c = collector.borrow_mut();
                        c.active = true;
                    },
                    None => {
                        // // New entry: initialize a new collector
                        // match Collector::create(&id) {
                        //     Ok(new_collector) => {
                        //         collectors.insert(id, RefCell::new(new_collector));
                        //     },
                        //     Err(err) => {
                        //         // Back off until next iteration
                        //         eprintln!("Could not initialize collector for cid {}: {}", id, err);
                        //     }
                        // };                        
                    }
                }
            }

            // collectors.retain(|_, value| {
            //     let c = value.borrow_mut();
            //     if c.active {
            //         return true;
            //     }

            //     let _result = c.buffer.flush();
            //     drop(c.file);
            //     false
            // })
        }

        // loop over container ids
        // for each one, look at cgroup, read from vfs, write to buffer,
        // flush/write to file if full
        
    }
}

/// Constructs the log filepath for the given container id
fn construct_log_path(id: &String) -> String {
    format!("/var/logs/docker/stats/{}.log", id)
}

fn collect(_id: &String, _buffer: &BufWriter<File>) -> Result<(), ()> {
    Ok(())
}
