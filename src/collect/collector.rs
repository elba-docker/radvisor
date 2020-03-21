use crate::types::ContainerMetadata;
use crate::util;
use crate::collect::container;
use std::fs;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::io::{Error, Write, ErrorKind};

use csv::{Writer, WriterBuilder};

const BUFFER_LENGTH: usize = 64 * 1024;

/// Contains the file handle for the open stats file as well as the buffer to use
/// when writing. `active` is used during difference resolution to mark inactive
/// collectors for teardown/removal.
pub struct Collector {
    pub writer: Writer<File>,
    pub file_handles: ProcFileHandles,
    pub active: bool,
}

/// File handles re-used for each container that read into the /proc VFS
pub struct ProcFileHandles {
    /// Current # of processes
    pub current_pids: Option<File>,
    /// Maximum # of processes
    pub max_pids: Option<File>
}

impl Collector {
    pub fn create(
        container: &ContainerMetadata,
        logs_location: &String,
    ) -> Result<Collector, Error> {
        // Ensure directories exist before creating the collector
        fs::create_dir_all(logs_location)?;
        let path = construct_log_path(&container.id, logs_location)?;
        let collector = Collector::new(&path, &container)?;
        Ok(collector)
    }

    /// Initializes a new collector and opens up a file handle at its corresponding
    /// log filepath. Writes
    fn new(log_path: &str, container: &ContainerMetadata) -> Result<Collector, Error> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(log_path)?;

        // Write the initial info to the file before initializing the CSV writer
        file.write(container.info.as_bytes())?;
        file.write(format!("# Initialized at: {}\n", util::nano_ts()).as_bytes())?;

        let mut writer = WriterBuilder::new()
            .buffer_capacity(BUFFER_LENGTH)
            .from_writer(file);
            writer.write_byte_record(container::get_header())?;
        Ok(Collector {
            writer,
            active: true,
            file_handles: container::initialize_file_handles(&container.id)
        })
    }
}

/// Constructs the log filepath for the given container id
fn construct_log_path(id: &str, logs_location: &str) -> Result<String, Error> {
    // Construct filename
    let filename = format!("{}_{}.log", id.to_string(), util::second_ts().to_string());

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
