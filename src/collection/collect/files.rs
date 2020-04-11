use std::fs::File;
use std::path::{Path, PathBuf};

const CGROUP_V1_ROOT: &str = "/sys/fs/cgroup";

/// File handles re-used for each target that read into the /proc VFS
pub struct ProcFileHandles {
    pub current_pids:               Option<File>,
    pub max_pids:                   Option<File>,
    pub cpu_stat:                   Option<File>,
    pub cpuacct_stat:               Option<File>,
    pub cpuacct_usage:              Option<File>,
    pub cpuacct_usage_sys:          Option<File>,
    pub cpuacct_usage_user:         Option<File>,
    pub cpuacct_usage_percpu:       Option<File>,
    pub memory_usage_in_bytes:      Option<File>,
    pub memory_max_usage_in_bytes:  Option<File>,
    pub memory_limit_in_bytes:      Option<File>,
    pub memory_soft_limit_in_bytes: Option<File>,
    pub memory_failcnt:             Option<File>,
    pub memory_stat:                Option<File>,
    pub blkio_io_service_bytes:     Option<File>,
    pub blkio_io_serviced:          Option<File>,
    pub blkio_io_service_time:      Option<File>,
    pub blkio_io_queued:            Option<File>,
    pub blkio_io_wait_time:         Option<File>,
    pub blkio_io_merged:            Option<File>,
    pub blkio_time:                 Option<File>,
    pub blkio_sectors:              Option<File>,
}

impl ProcFileHandles {
    /// Initializes all file handles to /proc files, utilizing them over the
    /// entire timeline of the target monitoring. If a handle fails to
    /// open, the struct field will be None
    #[must_use]
    pub fn new<C: AsRef<Path>>(cgroup: C) -> Self {
        Self {
            current_pids:               open_proc_file(&cgroup, "pids", "pids.current"),
            max_pids:                   open_proc_file(&cgroup, "pids", "pids.max"),
            cpu_stat:                   open_proc_file(&cgroup, "cpu", "cpu.stat"),
            cpuacct_stat:               open_proc_file(&cgroup, "cpuacct", "cpuacct.stat"),
            cpuacct_usage:              open_proc_file(&cgroup, "cpuacct", "cpuacct.usage"),
            cpuacct_usage_sys:          open_proc_file(&cgroup, "cpuacct", "cpuacct.usage_sys"),
            cpuacct_usage_user:         open_proc_file(&cgroup, "cpuacct", "cpuacct.usage_user"),
            cpuacct_usage_percpu:       open_proc_file(&cgroup, "cpuacct", "cpuacct.usage_percpu"),
            memory_usage_in_bytes:      open_proc_file(&cgroup, "memory", "memory.usage_in_bytes"),
            memory_max_usage_in_bytes:  open_proc_file(
                &cgroup,
                "memory",
                "memory.max_usage_in_bytes",
            ),
            memory_limit_in_bytes:      open_proc_file(&cgroup, "memory", "memory.limit_in_bytes"),
            memory_soft_limit_in_bytes: open_proc_file(
                &cgroup,
                "memory",
                "memory.soft_limit_in_bytes",
            ),
            memory_failcnt:             open_proc_file(&cgroup, "memory", "memory.failcnt"),
            memory_stat:                open_proc_file(&cgroup, "memory", "memory.stat"),
            blkio_io_service_bytes:     open_proc_file(
                &cgroup,
                "blkio",
                "blkio.io_service_bytes_recursive",
            ),
            blkio_io_serviced:          open_proc_file(
                &cgroup,
                "blkio",
                "blkio.io_serviced_recursive",
            ),
            blkio_io_service_time:      open_proc_file(
                &cgroup,
                "blkio",
                "blkio.io_service_time_recursive",
            ),
            blkio_io_queued:            open_proc_file(
                &cgroup,
                "blkio",
                "blkio.io_queued_recursive",
            ),
            blkio_io_wait_time:         open_proc_file(
                &cgroup,
                "blkio",
                "blkio.io_wait_time_recursive",
            ),
            blkio_io_merged:            open_proc_file(
                &cgroup,
                "blkio",
                "blkio.io_merged_recursive",
            ),
            blkio_time:                 open_proc_file(&cgroup, "blkio", "blkio.time_recursive"),
            blkio_sectors:              open_proc_file(&cgroup, "blkio", "blkio.sectors_recursive"),
        }
    }
}

/// Opens a stats file in /proc for the cgroup corresponding to the given
/// relative cgroup in the given subsystem
#[must_use]
fn open_proc_file<C: AsRef<Path>>(cgroup: C, subsystem: &str, file: &str) -> Option<File> {
    let mut path: PathBuf = PathBuf::from(CGROUP_V1_ROOT);
    path.push(subsystem);
    path.push(cgroup);
    path.push(file);
    File::open(path).ok()
}
