use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// Docker cgroup driver used to orchestrate moving containers in and out of
/// cgroups
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

/// Encapsulated behavior for lazy-resolution of Docker cgroup driver (systemd
/// or cgroupfs). Works for cgroups v1
pub struct CgroupManager {
    driver: Option<CgroupDriver>,
}

/// Resolved and existing cgroup path constructed from the construction methods
/// on `CgroupManager`
pub struct CgroupPath {
    pub path:   PathBuf,
    pub driver: CgroupDriver,
}

impl Default for CgroupManager {
    fn default() -> Self { Self::new() }
}

impl CgroupManager {
    /// Creates a new cgroup manager with an unknown driver type
    #[must_use]
    pub const fn new() -> Self { Self { driver: None } }

    /// Joins together the given slices to make a target cgroup, performing
    /// formatting conversions as necessary to target the current cgroup
    /// driver. If no driver is currently set, then tries to detect the
    /// current driver by seeing if the resultant formatted cgroup path from
    /// any of the drivers currently exists in the cgroup filesystem. This
    /// existence check is also performed if the current driver is known; if the
    /// cgroup was constructed and exists, returns Some(constructed path), else
    /// None.
    ///
    /// To use the direct slices without transforming their formatting in any
    /// way, set construct to false
    pub fn get_cgroup(&mut self, slices: &[&str], construct: bool) -> Option<CgroupPath> {
        match self.driver {
            Some(driver) => {
                let path = make(driver, slices, construct);
                match cgroup_exists(&path) {
                    true => Some(CgroupPath { path, driver }),
                    false => None,
                }
            },
            None => self
                .try_resolve(CgroupDriver::Systemd, slices, construct)
                .or_else(|| self.try_resolve(CgroupDriver::Cgroupfs, slices, construct)),
        }
    }

    /// Joins together the given slices to make a target cgroup, performing
    /// formatting conversions as necessary to target the current cgroup
    /// driver. If no driver is currently set, then tries to detect the
    /// current driver by seeing if the resultant formatted cgroup path from
    /// any of the drivers currently exists in the cgroup filesystem. This
    /// existence check is also performed if the current driver is known; if the
    /// cgroup was constructed and exists, returns `Some(path)`, else
    /// `None`
    ///
    /// Differs from `get_cgroup` in that it allows for different slices to be
    /// specified for each driver
    ///
    /// To use the direct slices without transforming their formatting in any
    /// way, set construct to false
    pub fn get_cgroup_divided(
        &mut self,
        systemd_slices: &[&str],
        cgroupfs_slices: &[&str],
        construct: bool,
    ) -> Option<CgroupPath> {
        match self.driver {
            Some(driver) => {
                let path = match driver {
                    CgroupDriver::Systemd { .. } => make(driver, systemd_slices, construct),
                    CgroupDriver::Cgroupfs => make(driver, cgroupfs_slices, construct),
                };
                match cgroup_exists(&path) {
                    true => Some(CgroupPath { path, driver }),
                    false => None,
                }
            },
            None => self
                .try_resolve(CgroupDriver::Systemd, systemd_slices, construct)
                .or_else(|| self.try_resolve(CgroupDriver::Cgroupfs, cgroupfs_slices, construct)),
        }
    }

    /// Attempts to resolve the cgroup driver, by making the cgroup path for the
    /// given driver and then testing whether it exists
    ///
    /// To use the direct slices without transforming their formatting in any
    /// way, set construct to false
    fn try_resolve(
        &mut self,
        driver: CgroupDriver,
        slices: &[&str],
        construct: bool,
    ) -> Option<CgroupPath> {
        let path = make(driver, slices, construct);
        match cgroup_exists(&path) {
            false => None,
            true => {
                self.driver = Some(driver);
                Some(CgroupPath { path, driver })
            },
        }
    }

    /// Gets the current resolved driver for the manager
    #[must_use]
    pub const fn driver(&self) -> Option<CgroupDriver> { self.driver }
}

/// Constructs a cgroup absolute path according to the style expected by the
/// given driver
///
/// To use the direct slices without transforming their formatting in any
/// way, set construct to false
#[must_use]
pub fn make(driver: CgroupDriver, slices: &[&str], construct: bool) -> PathBuf {
    match driver {
        CgroupDriver::Cgroupfs => make_cgroupfs(slices),
        CgroupDriver::Systemd => make_systemd(slices, construct),
    }
}

const SYSTEMD_SLICE_SUFFIX: &str = ".slice";

/// Converts a vec of slice names such as vec!["kubepods", "burstable",
/// "pod1234-5678"] into a systemd-style cgroup path such as "/kubepods.slice/
/// kubepods-burstable.slice/kubepods-burstable-pod1234_5678.slice"
/// see [`kubernetes/kubelet/cm/cgroup_manager_linux.go:ToSystemd()`](https://github.com/kubernetes/kubernetes/blob/bb5ed1b79709c865d9aa86008048f19331530041/pkg/kubelet/cm/cgroup_manager_linux.go#L87-L103)
///
/// To use the direct slices without transforming their formatting in any
/// way, set construct to false
#[must_use]
fn make_systemd(slices: &[&str], construct: bool) -> PathBuf {
    if slices.is_empty() || slices.len() == 1 && slices[0].is_empty() {
        return PathBuf::from("");
    }

    // First, escape systemd slices
    let escaped = slices.iter().map(|&s| escape_systemd(s));

    match construct {
        false => escaped.collect::<PathBuf>(),
        true => {
            // Aggregate each slice with all previous to build final path
            let mut path: PathBuf = PathBuf::new();
            // Previously accumulated slices like "kubepods-burstable-"
            let mut accumulator: String = String::new();
            // Re-usable working buffer
            let mut working: String = String::new();
            for slice in escaped {
                // Add the current slice to the path
                working += &accumulator;
                working += &slice;
                working += SYSTEMD_SLICE_SUFFIX;
                path.push(&working);
                working.clear();

                // Add the current slice to the accumulator
                accumulator += &slice;
                accumulator += "-";
            }

            path
        },
    }
}

/// Escapes a cgroup slice to be in the style of Systemd cgroups
/// see [`kubernetes/kubelet/cm/cgroup_manager_linux.go:escapeSystemdCgroupName()`](https://github.com/kubernetes/kubernetes/blob/bb5ed1b79709c865d9aa86008048f19331530041/pkg/kubelet/cm/cgroup_manager_linux.go#L74-L76)
#[must_use]
pub fn escape_systemd(slice: &str) -> String { slice.replace("-", "_") }

/// Converts a vec of slice names such as vec!["kubepods", "burstable",
/// "pod1234-5678"] into a systemd-style cgroup path such as
/// `/kubepods/burstable/pod1234_5678` see [`kubernetes/kubelet/cm/cgroup_manager_linux.go:ToCgroupfs()`](https://github.com/kubernetes/kubernetes/blob/bb5ed1b79709c865d9aa86008048f19331530041/pkg/kubelet/cm/cgroup_manager_linux.go#L116-L118)
#[must_use]
fn make_cgroupfs(slices: &[&str]) -> PathBuf { slices.iter().collect() }

pub const INVALID_CGROUP_MOUNT_MESSAGE: &str =
    "rAdvisor expects cgroups to be mounted in /sys/fs/cgroup. If this is\nthe case, make sure \
     that the 'cpuacct' resource controller has not been disabled.";

/// Checks if cgroups are mounted in /sys/fs/cgroup and if the cpuacct subsystem
/// is enabled (necessary for proper driver detection)
#[must_use]
pub fn cgroups_mounted_properly() -> bool {
    // Use the raw subsystem directory to see if the expected cgroup hierarchy
    // exists
    cgroup_exists("")
}

pub const LINUX_CGROUP_ROOT: &str = "/sys/fs/cgroup";

/// Determines whether the given (absolute) cgroup exists in the virtual
/// filesystem **Note**: fails if cgroups aren't mounted in /sys/fs/cgroup or if
/// the cpuacct subsystem isn't enabled.
#[must_use]
fn cgroup_exists<C: AsRef<Path>>(path: C) -> bool {
    let mut full_path: PathBuf = [LINUX_CGROUP_ROOT, "cpuacct"].iter().collect();
    full_path.push(path);
    match fs::metadata(full_path) {
        Err(_) => false,
        // As long as it exists and is a directory, assume all is good
        Ok(metadata) => metadata.is_dir(),
    }
}
