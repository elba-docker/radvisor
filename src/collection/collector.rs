use crate::cli;
use crate::collection::collect;
use crate::collection::collect::files::ProcFileHandles;
use crate::collection::collect::read::StatFileLayout;
use crate::collection::system_info::SystemInfo;
use crate::shared::TargetMetadata;
use crate::util;
use std::fs::{self, File, OpenOptions};
use std::io::{Error, ErrorKind, Write};
use std::path::{Path, PathBuf};

use csv::{Writer, WriterBuilder};

/// CSV writer buffer length
const BUFFER_LENGTH: usize = 64 * 1024;

/// Contains the file handle for the open stats file as well as the file handles
/// for /proc virtual files used during reading the system stats. `active` is
/// used during difference resolution to mark inactive collectors for
/// teardown/removal.
pub struct Collector {
    pub writer:        Writer<File>,
    pub file_handles:  ProcFileHandles,
    pub active:        bool,
    pub memory_layout: StatFileLayout,
    pub id:            String,
    pub provider_type: &'static str,
}

impl Collector {
    /// Creates a new collector at the given log file destination, making all
    /// intermediate directories as necessary. Then, opens up all required
    /// read and write file handles and writes the file header for the log
    /// file.
    pub fn create(logs_location: &PathBuf, target: &TargetMetadata) -> Result<Collector, Error> {
        // Ensure directories exist before creating the collector
        fs::create_dir_all(logs_location)?;
        let path = construct_log_path(&target.id, logs_location)?;
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(path)?;
        let collector = Collector::new(file, &target)?;
        Ok(collector)
    }

    /// Collects the current statistics for the given target, writing the CSV
    /// entries to the writer. Utilizes /proc and cgroups (Linux-only)
    pub fn collect(
        &mut self,
        working_buffers: &mut collect::WorkingBuffers,
    ) -> Result<(), csv::Error> {
        collect::run(self, working_buffers)
    }

    /// Initializes a new collector given the destination file and target
    /// metadata. Writes the file header and then opens up read file handles
    /// for all of the /proc cgroup virtual files
    fn new(file: File, target: &TargetMetadata) -> Result<Self, Error> {
        let system_info: String = textwrap::indent(&SystemInfo::get().as_yaml(), "  ");

        // Write the YAML header to the file before initializing the CSV writer
        writeln!(&file, "---")?;
        writeln!(&file, "Version: {}", cli::VERSION.unwrap_or("unknown"))?;
        writeln!(&file, "Provider: {}", target.provider_type)?;
        write!(&file, "{}", target.info)?;
        write!(&file, "System:\n{}", system_info)?;
        writeln!(&file, "Cgroup: {}", target.cgroup.path.display())?;
        writeln!(&file, "CgroupDriver: {}", target.cgroup.driver)?;
        writeln!(&file, "InitializedAt: {}", util::nano_ts())?;
        writeln!(&file, "---")?;

        // Initialize the CSV writer and then write the header row
        let mut writer = WriterBuilder::new()
            .buffer_capacity(BUFFER_LENGTH)
            .from_writer(file);
        writer.write_byte_record(collect::get_header())?;

        let file_handles = ProcFileHandles::new(&target.cgroup.path);
        let memory_layout = collect::examine_memory(&file_handles);
        Ok(Collector {
            writer,
            active: true,
            file_handles,
            memory_layout,
            id: target.id.clone(),
            provider_type: target.provider_type,
        })
    }
}

/// Constructs the log filepath for the given target id
fn construct_log_path(id: &str, logs_location: &PathBuf) -> Result<String, Error> {
    // Construct filename
    let filename = format!("{}_{}.log", id.to_string(), util::second_ts().to_string());

    // Join paths
    let base = Path::new(logs_location);
    let filename_path = Path::new(&filename);
    match base.join(filename_path).into_os_string().into_string() {
        Ok(path) => Ok(path),
        Err(_) => Err(Error::new(
            ErrorKind::InvalidInput,
            format!("could not create log path in {:?}", logs_location),
        )),
    }
}
