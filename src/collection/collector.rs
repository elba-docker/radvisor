use crate::collection::collect;
use crate::collection::collect::files::ProcFileHandles;
use crate::collection::collect::read::StatFileLayout;
use crate::shared::ContainerMetadata;
use crate::util;
use crate::cli;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{Error, ErrorKind, Write};
use std::path::Path;

use csv::{Writer, WriterBuilder};

/// CSV writer buffer length
const BUFFER_LENGTH: usize = 64 * 1024;

/// Contains the file handle for the open stats file as well as the file handles
/// for /proc virtual files used during reading the system stats. `active` is used
/// during difference resolution to mark inactive collectors for teardown/removal.
pub struct Collector {
    pub writer: Writer<File>,
    pub file_handles: ProcFileHandles,
    pub active: bool,
    pub memory_layout: StatFileLayout,
}

impl Collector {
    /// Creates a new collector at the given log file destination, making all intermediate
    /// directories as neccessary. Then, opens up all required read and write file handles
    /// and writes the file header for the log file. 
    pub fn create(
        logs_location: &String,
        container: &ContainerMetadata,
    ) -> Result<Collector, Error> {
        // Ensure directories exist before creating the collector
        fs::create_dir_all(logs_location)?;
        let path = construct_log_path(&container.id, logs_location)?;
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(path)?;
        let collector = Collector::new(file, &container)?;
        Ok(collector)
    }

    /// Collects the current statistics for the given container, writing the CSV entries to
    /// the writer. Utilizes /proc and cgroups (Linux-only)
    pub fn collect(&mut self, working_buffers: &mut collect::WorkingBuffers) -> Result<(), csv::Error> {
        collect::run(self, working_buffers)
    }

    /// Initializes a new collector given the destination file and container metadata.
    /// Writes the file header and then opens up read file handles for all of the /proc
    /// cgroup virtual files
    fn new(file: File, container: &ContainerMetadata) -> Result<Self, Error> {
        let mut file = file;

        // Write the initial info to the file before initializing the CSV writer
        file.write(format!("# Version: {}\n", cli::VERSION.unwrap_or("unknown")).as_bytes())?;
        file.write(container.info.as_bytes())?;
        file.write(format!("# Initialized at: {}\n", util::nano_ts()).as_bytes())?;

        // Initialize the CSV writer and then write the header row
        let mut writer = WriterBuilder::new()
            .buffer_capacity(BUFFER_LENGTH)
            .from_writer(file);
        writer.write_byte_record(collect::get_header())?;

        let file_handles = ProcFileHandles::new(&container.id);
        let memory_layout = collect::examine_memory(&file_handles);
        Ok(Collector {
            writer: writer,
            active: true,
            file_handles,
            memory_layout
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
