use crate::cli;
use crate::collection::collect;
use crate::collection::collect::files::ProcFileHandles;
use crate::collection::collect::read::StatFileLayout;
use crate::collection::flush::{FlushLog, FlushLogger};
use crate::collection::perf_table::TableMetadata;
use crate::collection::system_info::SystemInfo;
use crate::shared::CollectionTarget;
use crate::util::{self, CgroupDriver, CgroupPath};
use csv::{Writer, WriterBuilder};
use failure::Error;
use serde::Serialize;
use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Contains the file handle for the open stats file as well as the file handles
/// for /proc virtual files used during reading the system stats. `active` is
/// used during difference resolution to mark inactive collectors for
/// teardown/removal.
pub struct Collector {
    pub writer:        Writer<FlushLogger<File>>,
    pub file_handles:  ProcFileHandles,
    pub active:        bool,
    pub memory_layout: StatFileLayout,
    pub target:        CollectionTarget,
}

/// Bundles together all information stored in log file headers
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
struct LogFileHeader<'a> {
    version:        &'static str,
    provider:       &'static str,
    metadata:       &'a Option<serde_yaml::Value>,
    perf_table:     &'a TableMetadata,
    system:         SystemInfo,
    cgroup:         &'a PathBuf,
    cgroup_driver:  &'a CgroupDriver,
    polled_at:      u128,
    initialized_at: u128,
}

impl Collector {
    /// Creates a new collector at the given log file destination, making all
    /// intermediate directories as necessary. Then, opens up all required
    /// read and write file handles and writes the file header for the log
    /// file.
    pub fn create(
        logs_location: &Path,
        target: CollectionTarget,
        cgroup: &CgroupPath,
        buffer_capacity: usize,
        perf_table: &Arc<TableMetadata>,
        event_log: Option<Arc<Mutex<FlushLog>>>,
    ) -> Result<Self, Error> {
        // Ensure directories exist before creating the collector
        fs::create_dir_all(logs_location)?;
        let path = construct_log_path(&target.id, logs_location)?;
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(path)?;
        let collector = Self::new(file, target, cgroup, buffer_capacity, perf_table, event_log)?;
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
    fn new(
        file: File,
        target: CollectionTarget,
        cgroup: &CgroupPath,
        buffer_capacity: usize,
        perf_table: &Arc<TableMetadata>,
        event_log: Option<Arc<Mutex<FlushLog>>>,
    ) -> Result<Self, Error> {
        let header = LogFileHeader {
            version: cli::VERSION.unwrap_or("unknown"),
            provider: target.provider,
            metadata: &target.metadata,
            system: SystemInfo::get(),
            cgroup: &cgroup.path,
            cgroup_driver: &cgroup.driver,
            polled_at: target.poll_time,
            initialized_at: util::nano_ts(),
            perf_table,
        };

        // Write the YAML header to the file before initializing the CSV writer
        let header_str = serde_yaml::to_string(&header)?;
        writeln!(&file, "{}", header_str)?;
        writeln!(&file, "---")?;

        // Initialize the CSV writer and then write the header row
        let mut writer = WriterBuilder::new()
            .buffer_capacity(buffer_capacity)
            .from_writer(FlushLogger::new(file, target.id.clone(), event_log));
        writer.write_byte_record(collect::get_header())?;

        let file_handles = ProcFileHandles::new(&cgroup.path);
        let memory_layout = collect::examine_memory(&file_handles);
        Ok(Self {
            writer,
            active: true,
            file_handles,
            memory_layout,
            target,
        })
    }
}

/// Constructs the log filepath for the given target id
fn construct_log_path(id: &str, logs_location: &Path) -> Result<String, io::Error> {
    // Construct filename
    let filename = format!("{}_{}.log", id.to_string(), util::second_ts().to_string());

    // Join paths
    let base = Path::new(logs_location);
    let filename_path = Path::new(&filename);
    match base.join(filename_path).into_os_string().into_string() {
        Ok(path) => Ok(path),
        Err(_) => Err(io::Error::new(
            ErrorKind::InvalidInput,
            format!("could not create log path in {:?}", logs_location),
        )),
    }
}
