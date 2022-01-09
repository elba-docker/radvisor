mod all;
mod cgroup_v1;

use crate::cli;
use crate::collection::buffers::WorkingBuffers;
use crate::collection::flush::{FlushLog, FlushLogger};
use crate::collection::perf_table::TableMetadata;
use crate::collection::system_info::SystemInfo;
use crate::shared::CollectionTarget;
use crate::util;
use anyhow::Error;
use csv::WriterBuilder;
use serde::Serialize;
use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub use all::CollectorImpl;

pub type StatWriter = csv::Writer<FlushLogger<File>>;

pub trait Collector {
    fn metadata(&mut self) -> Option<serde_yaml::Value>;
    fn table_metadata(&mut self) -> TableMetadata;
    fn get_type(&self) -> &'static str;
    fn init(&mut self) -> Result<(), Error>;
    fn write_header(&mut self, writer: &mut StatWriter) -> Result<(), csv::Error>;
    fn collect(
        &mut self,
        writer: &mut StatWriter,
        working_buffers: &mut WorkingBuffers,
    ) -> Result<(), csv::Error>;
}

/// Wraps a concrete implementation of Collector,
/// handling setting up the log file as needed.
pub struct Handle {
    pub collector: CollectorImpl,
    pub writer:    StatWriter,
    pub target:    CollectionTarget,
    /// `active` is used during difference resolution
    /// to mark inactive collectors for teardown/removal.
    pub active:    bool,
}

/// Bundles together all information stored in log file headers
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
struct LogFileHeader<'a> {
    version:            &'static str,
    provider:           &'static str,
    metadata:           &'a Option<serde_yaml::Value>,
    perf_table:         &'a TableMetadata,
    system:             SystemInfo,
    collector_type:     &'static str,
    collector_metadata: &'a Option<serde_yaml::Value>,
    polled_at:          u128,
    initialized_at:     u128,
}

impl Handle {
    /// Creates a new collector at the given log file destination,
    /// making all intermediate directories as necessary.
    /// Then, opens up all required read and write file handles
    /// and writes the file header for the log file.
    pub fn new(
        logs_location: &Path,
        target: CollectionTarget,
        collector: CollectorImpl,
        buffer_capacity: usize,
        event_log: Option<Arc<Mutex<FlushLog>>>,
    ) -> Result<Self, Error> {
        let mut collector = collector;

        // Ensure directories exist before creating the collector
        fs::create_dir_all(logs_location)?;
        let path = construct_log_path(&target.id, logs_location)?;
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(path)?;

        let collector_metadata = collector.metadata();
        let perf_table = collector.table_metadata();
        let header = LogFileHeader {
            version:            cli::VERSION.unwrap_or("unknown"),
            provider:           target.provider,
            metadata:           &target.metadata,
            system:             SystemInfo::get(),
            collector_type:     collector.get_type(),
            collector_metadata: &collector_metadata,
            polled_at:          target.poll_time,
            initialized_at:     util::nano_ts(),
            perf_table:         &perf_table,
        };

        // Write the YAML header to the file before initializing the CSV writer
        let header_str = serde_yaml::to_string(&header)?;
        writeln!(&file, "{}", header_str)?;
        writeln!(&file, "---")?;

        // Initialize the CSV writer and then write the header row
        let mut writer = WriterBuilder::new()
            .buffer_capacity(buffer_capacity)
            .from_writer(FlushLogger::new(file, target.id.clone(), event_log));
        collector.write_header(&mut writer)?;

        // Let the collector initialize inner state
        collector.init()?;

        Ok(Self {
            collector,
            writer,
            target,
            active: true,
        })
    }

    /// Collects the current statistics for the given target,
    /// writing the CSV entries to the writer.
    pub fn collect(&mut self, working_buffers: &mut WorkingBuffers) -> Result<(), csv::Error> {
        self.collector.collect(&mut self.writer, working_buffers)
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
