use std::fs::File;
use std::path::{Path, PathBuf};

const CGROUP_V1_ROOT: &str = "/sys/fs/cgroup";

/// File handles re-used for each target that read into the /proc VFS
pub struct ProcFileHandles {
    pub current_pids:                    Option<File>,
    pub max_pids:                        Option<File>,
    pub cpu_stat:                        Option<File>,
    pub cpuacct_stat:                    Option<File>,
    pub cpuacct_usage:                   Option<File>,
    pub cpuacct_usage_sys:               Option<File>,
    pub cpuacct_usage_user:              Option<File>,
    pub cpuacct_usage_percpu:            Option<File>,
    pub memory_usage_in_bytes:           Option<File>,
    pub memory_max_usage_in_bytes:       Option<File>,
    pub memory_limit_in_bytes:           Option<File>,
    pub memory_soft_limit_in_bytes:      Option<File>,
    pub memory_failcnt:                  Option<File>,
    pub memory_stat:                     Option<File>,
    pub blkio_io_service_bytes:          Option<File>,
    pub blkio_io_serviced:               Option<File>,
    pub blkio_io_service_time:           Option<File>,
    pub blkio_io_queued:                 Option<File>,
    pub blkio_io_wait_time:              Option<File>,
    pub blkio_io_merged:                 Option<File>,
    pub blkio_time:                      Option<File>,
    pub blkio_sectors:                   Option<File>,
    pub blkio_throttle_io_service_bytes: Option<File>,
    pub blkio_throttle_io_serviced:      Option<File>,
    pub blkio_bfq_io_service_bytes:      Option<File>,
    pub blkio_bfq_io_serviced:           Option<File>,
}

impl ProcFileHandles {
    /// Initializes all file handles to /proc files, utilizing them over the
    /// entire timeline of the target monitoring. If a handle fails to
    /// open, the struct field will be None
    #[must_use]
    pub fn new<C: AsRef<Path>>(cgroup: C) -> Self {
        Self {
            current_pids:                    o(&cgroup, "pids", "pids.current"),
            max_pids:                        o(&cgroup, "pids", "pids.max"),
            cpu_stat:                        o(&cgroup, "cpu", "cpu.stat"),
            cpuacct_stat:                    o(&cgroup, "cpuacct", "cpuacct.stat"),
            cpuacct_usage:                   o(&cgroup, "cpuacct", "cpuacct.usage"),
            cpuacct_usage_sys:               o(&cgroup, "cpuacct", "cpuacct.usage_sys"),
            cpuacct_usage_user:              o(&cgroup, "cpuacct", "cpuacct.usage_user"),
            cpuacct_usage_percpu:            o(&cgroup, "cpuacct", "cpuacct.usage_percpu"),
            memory_usage_in_bytes:           o(&cgroup, "memory", "memory.usage_in_bytes"),
            memory_max_usage_in_bytes:       o(&cgroup, "memory", "memory.max_usage_in_bytes"),
            memory_limit_in_bytes:           o(&cgroup, "memory", "memory.limit_in_bytes"),
            memory_soft_limit_in_bytes:      o(&cgroup, "memory", "memory.soft_limit_in_bytes"),
            memory_failcnt:                  o(&cgroup, "memory", "memory.failcnt"),
            memory_stat:                     o(&cgroup, "memory", "memory.stat"),
            blkio_io_service_bytes:          o(&cgroup, "blkio", "blkio.io_service_bytes"),
            blkio_io_serviced:               o(&cgroup, "blkio", "blkio.io_serviced"),
            blkio_io_service_time:           o(&cgroup, "blkio", "blkio.io_service_time"),
            blkio_io_queued:                 o(&cgroup, "blkio", "blkio.io_queued"),
            blkio_io_wait_time:              o(&cgroup, "blkio", "blkio.io_wait_time"),
            blkio_io_merged:                 o(&cgroup, "blkio", "blkio.io_merged"),
            blkio_time:                      o(&cgroup, "blkio", "blkio.time"),
            blkio_sectors:                   o(&cgroup, "blkio", "blkio.sectors"),
            blkio_throttle_io_service_bytes: o(&cgroup, "blkio", "blkio.throttle.io_service_bytes"),
            blkio_throttle_io_serviced:      o(&cgroup, "blkio", "blkio.throttle.io_serviced"),
            blkio_bfq_io_service_bytes:      o(&cgroup, "blkio", "blkio.bfq.io_service_bytes"),
            blkio_bfq_io_serviced:           o(&cgroup, "blkio", "blkio.bfq.io_serviced"),
        }
    }
}

/// Opens a stats file in /proc for the cgroup corresponding to the given
/// relative cgroup in the given subsystem
#[must_use]
fn o<C: AsRef<Path>>(cgroup: C, subsystem: &str, file: &str) -> Option<File> {
    let mut path: PathBuf = PathBuf::from(CGROUP_V1_ROOT);
    path.push(subsystem);
    path.push(cgroup);
    path.push(file);
    File::open(path).ok()
}
