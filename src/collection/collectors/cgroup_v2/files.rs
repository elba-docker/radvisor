use std::fs::File;
use std::path::{Path, PathBuf};

const CGROUP_V2_ROOT: &str = "/sys/fs/cgroup";

/// File handles re-used for each target that read into the /proc VFS
pub struct ProcFileHandles {
    pub pids_current:   Option<File>,
    pub pids_max:       Option<File>,
    pub cpu_stat:       Option<File>,
    pub memory_current: Option<File>,
    pub memory_high:    Option<File>,
    pub memory_max:     Option<File>,
    pub memory_stat:    Option<File>,
    pub io_stat:        Option<File>,
}

impl ProcFileHandles {
    /// Initializes all file handles to /proc files, utilizing them over the
    /// entire timeline of the target monitoring. If a handle fails to
    /// open, the struct field will be None
    #[must_use]
    pub fn new<C: AsRef<Path>>(cgroup: C) -> Self {
        Self {
            pids_current:   o(&cgroup, "pids.current"),
            pids_max:       o(&cgroup, "pids.max"),
            cpu_stat:       o(&cgroup, "cpu.stat"),
            memory_current: o(&cgroup, "memory.current"),
            memory_high:    o(&cgroup, "memory.high"),
            memory_max:     o(&cgroup, "memory.max"),
            memory_stat:    o(&cgroup, "memory.stat"),
            io_stat:        o(&cgroup, "io.stat"),
        }
    }
}

/// Opens a stats file in /proc for the cgroup corresponding to the given
/// relative cgroup
#[must_use]
fn o<C: AsRef<Path>>(cgroup: C, file: &str) -> Option<File> {
    let mut path: PathBuf = PathBuf::from(CGROUP_V2_ROOT);
    path.push(cgroup);
    path.push(file);
    File::open(path).ok()
}
