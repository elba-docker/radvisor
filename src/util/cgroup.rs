use serde::Serialize;
use std::fmt;
use std::path::{Path, PathBuf};

/// Docker cgroup driver used to orchestrate
/// moving containers in and out of cgroups
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CgroupDriver {
    Systemd,
    Cgroupfs,
}

impl fmt::Display for CgroupDriver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Systemd => write!(f, "systemd"),
            Self::Cgroupfs => write!(f, "cgroupfs"),
        }
    }
}

/// Linux cgroup version
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CgroupVersion {
    V1,
    V2,
}

impl fmt::Display for CgroupVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::V1 => write!(f, "v1"),
            Self::V2 => write!(f, "v2"),
        }
    }
}

pub const CGROUP_V2_CHECK_PATH: &str = "/sys/fs/cgroup/cgroup.controllers";

impl CgroupVersion {
    fn try_resolve() -> Option<Self> {
        if Path::new(CGROUP_V2_CHECK_PATH).exists() {
            return Some(Self::V2);
        }

        if cgroup_exists(None::<String>, Self::V1) {
            return Some(Self::V1);
        }

        None
    }
}

pub struct CgroupSlices<'c, 's, C, S>
where
    C: AsRef<str>,
    S: AsRef<str>,
{
    pub cgroupfs: &'c [C],
    pub systemd:  &'s [S],
}

impl<C, S> CgroupSlices<'_, '_, C, S>
where
    C: AsRef<str>,
    S: AsRef<str>,
{
    #[must_use]
    fn pick_and_join(&self, driver: CgroupDriver) -> PathBuf {
        match driver {
            CgroupDriver::Cgroupfs => join_slices(self.cgroupfs),
            CgroupDriver::Systemd => join_slices(self.systemd),
        }
    }
}

#[must_use]
fn join_slices(s: &[impl AsRef<str>]) -> PathBuf {
    let mut path_buf = PathBuf::new();
    for s in s {
        path_buf.push(s.as_ref());
    }
    path_buf
}

/// Encapsulated behavior for lazy-resolution of Docker cgroup driver (systemd
/// or cgroupfs). Works for cgroup v1 and v2
pub struct CgroupManager {
    driver:  Option<CgroupDriver>,
    version: Option<CgroupVersion>,
}

/// Resolved and existing cgroup path constructed from the construction methods
/// on `CgroupManager`
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CgroupPath {
    pub path:    PathBuf,
    pub driver:  CgroupDriver,
    pub version: CgroupVersion,
}

impl Default for CgroupManager {
    fn default() -> Self { Self::new() }
}

pub enum GetCgroupError {
    CgroupV1NotEnabled,
    VersionDetectionFailed,
    NotFound(PathBuf),
}

impl CgroupManager {
    /// Creates a new cgroup manager with an unknown driver type and version
    #[must_use]
    pub const fn new() -> Self {
        Self {
            driver:  None,
            version: None,
        }
    }

    /// Joins together the given slices to make a target cgroup,
    /// selecting the appropriate list of slices depending on the driver.
    /// Ensures that the cgroup path exists before returning it.
    /// If either the driver or version hasn't been detected yet,
    /// then this function also tries to detect them.
    ///
    /// Only works if cgroups are enabled,
    /// and mounted in the filesystem at `LINUX_CGROUP_ROOT`;
    /// otherwise returns `Err`.
    #[allow(clippy::needless_pass_by_value)]
    pub fn get_cgroup<C, S>(
        &mut self,
        slices: CgroupSlices<'_, '_, C, S>,
    ) -> Result<CgroupPath, GetCgroupError>
    where
        C: AsRef<str>,
        S: AsRef<str>,
    {
        let version = self
            .get_version_or_resolve()
            .ok_or(GetCgroupError::VersionDetectionFailed)?;

        match self.driver {
            Some(driver) => {
                // Pick the appropriate list of slices for the driver,
                // and join them together to make the path.
                let path: PathBuf = slices.pick_and_join(driver);

                // Make sure the cgroup exists before returning it
                match cgroup_exists(Some(&path), version) {
                    true => Ok(CgroupPath {
                        path,
                        driver,
                        version,
                    }),
                    false => Err(GetCgroupError::NotFound(path)),
                }
            },
            None => {
                // Try to see if the systemd cgroup exists
                let systemd_cgroup = join_slices(slices.systemd);
                if cgroup_exists(Some(&systemd_cgroup), version) {
                    self.driver = Some(CgroupDriver::Systemd);
                    return Ok(CgroupPath {
                        path: systemd_cgroup,
                        driver: CgroupDriver::Systemd,
                        version,
                    });
                }

                // Otherwise, try to see if the cgroupfs cgroup exists
                let cgroupfs_cgroup = join_slices(slices.cgroupfs);
                if cgroup_exists(Some(&cgroupfs_cgroup), version) {
                    self.driver = Some(CgroupDriver::Cgroupfs);
                    return Ok(CgroupPath {
                        path: cgroupfs_cgroup,
                        driver: CgroupDriver::Cgroupfs,
                        version,
                    });
                }

                Err(GetCgroupError::NotFound(systemd_cgroup))
            },
        }
    }

    /// Joins together the given slices to make a target cgroup,
    /// selecting the appropriate list of slices depending on the driver.
    /// Ensures that the cgroup path exists before returning it.
    /// If either the driver or version hasn't been detected yet,
    /// then this function also tries to detect them.
    ///
    /// Only works if cgroup v1 is enabled,
    /// and mounted in the filesystem at `LINUX_CGROUP_ROOT`;
    /// otherwise returns `Err`.
    pub fn get_cgroup_v1<C, S>(
        &mut self,
        slices: CgroupSlices<'_, '_, C, S>,
    ) -> Result<CgroupPath, GetCgroupError>
    where
        C: AsRef<str>,
        S: AsRef<str>,
    {
        let version = self
            .get_version_or_resolve()
            .ok_or(GetCgroupError::VersionDetectionFailed)?;

        if version != CgroupVersion::V1 {
            return Err(GetCgroupError::CgroupV1NotEnabled);
        }

        self.get_cgroup(slices)
    }

    fn get_version_or_resolve(&mut self) -> Option<CgroupVersion> {
        match self.version {
            Some(version) => Some(version),
            None => {
                // Attempt to detect the current version
                let version = CgroupVersion::try_resolve()?;
                self.version = Some(version);
                Some(version)
            },
        }
    }

    /// Gets the current resolved driver for the manager
    #[must_use]
    pub const fn driver(&self) -> Option<CgroupDriver> { self.driver }

    /// Gets the current resolved cgroup version for the manager
    #[must_use]
    pub const fn version(&self) -> Option<CgroupVersion> { self.version }
}

/// Converts a vec of slice names such as:
/// ```rs
/// vec!["kubepods", "burstable", "pod1234-5678"]
/// ```
/// into a systemd-style list of slice names such as:
/// ```rs
/// vec![
///   "kubepods.slice",
///   "kubepods-burstable.slice",
///   "kubepods-burstable-pod1234_5678.slice",
/// ]
/// ```
/// see [`kubernetes/kubelet/cm/cgroup_manager_linux.go:ToSystemd()`](https://github.com/kubernetes/kubernetes/blob/bb5ed1b79709c865d9aa86008048f19331530041/pkg/kubelet/cm/cgroup_manager_linux.go#L87-L103)
pub fn build_systemd_cgroup_hierarchy(slices: &[impl AsRef<str>]) -> Vec<String> {
    if slices.is_empty() || slices.len() == 1 && slices[0].as_ref().is_empty() {
        return vec![];
    }

    // Aggregate each slice with all previous to build the hierarchy:
    // Previously accumulated slices like "kubepods-burstable-"
    let mut accumulator: String = String::new();
    let mut hierarchy: Vec<String> = Vec::with_capacity(slices.len());
    for base_slice in slices {
        // Escape each slice before processing it
        let base_slice = base_slice.as_ref();
        let escaped_slice = escape_systemd(base_slice);

        // Add the current slice to the list
        hierarchy.push(format!("{}{}.slice", &accumulator, &escaped_slice));

        // Add the current slice to the accumulator
        accumulator += &escaped_slice;
        accumulator += "-";
    }

    hierarchy
}

/// Escapes a cgroup slice to be in the style of Systemd cgroups
/// see [`kubernetes/kubelet/cm/cgroup_manager_linux.go:escapeSystemdCgroupName()`](https://github.com/kubernetes/kubernetes/blob/bb5ed1b79709c865d9aa86008048f19331530041/pkg/kubelet/cm/cgroup_manager_linux.go#L74-L76)
#[must_use]
fn escape_systemd(slice: &str) -> String { slice.replace("-", "_") }

pub const INVALID_CGROUP_MOUNT_MESSAGE: &str =
    "rAdvisor expects cgroups to be enabled and mounted in /sys/fs/cgroup.";

/// Checks if cgroups are mounted in /sys/fs/cgroup
/// (for both cgroup v1 and v2)
#[must_use]
pub fn cgroups_mounted_properly() -> bool { Path::new(STANDARD_CGROUP_MOUNT_ROOT).exists() }

// From https://man7.org/linux/man-pages/man7/cgroups.7.html
pub const STANDARD_CGROUP_MOUNT_ROOT: &str = "/sys/fs/cgroup";

// From https://man7.org/linux/man-pages/man7/cgroups.7.html
pub const CGROUP_V1_SUBSYSTEMS: &[&str] = &[
    // Place the cpuacct subsystem first,
    // since it is most likely to exist
    // (so it will be checked first in `cgroup_v1_exists`).
    "cpuacct",
    "cpu",
    "cpuset",
    "memory",
    "devices",
    "freezer",
    "net_cls",
    "blkio",
    "perf_event",
    "net_prio",
    "hugetlb",
    "pids",
    "rdma",
];

/// Determines whether the given (absolute) cgroup
/// exists in the virtual filesystem at the standard mount point.
#[must_use]
fn cgroup_exists<C: AsRef<Path>>(path: Option<C>, version: CgroupVersion) -> bool {
    match version {
        CgroupVersion::V1 => {
            // See if any of the cgroup v1 subsystems are mounted
            for subsystem in CGROUP_V1_SUBSYSTEMS {
                let mut full_path = PathBuf::new();
                full_path.push(STANDARD_CGROUP_MOUNT_ROOT);
                full_path.push(subsystem);
                if let Some(p) = &path {
                    full_path.push(p.as_ref());
                }

                if full_path.exists() {
                    return true;
                }
            }
        },
        CgroupVersion::V2 => {
            let mut full_path = PathBuf::new();
            full_path.push(STANDARD_CGROUP_MOUNT_ROOT);
            if let Some(p) = path {
                full_path.push(p.as_ref());
            }

            return full_path.exists();
        },
    }

    false
}
