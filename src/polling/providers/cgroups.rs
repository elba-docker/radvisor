use std::fs;

/// Docker cgroup driver used to orchestrate moving containers in and out of
/// cgroups
#[derive(Clone, Copy)]
pub enum CgroupDriver {
    Systemd,
    Cgroupfs,
}

/// Encapsulated behavior for lazy-resolution of Docker cgroup driver (systemd
/// or cgroupfs). Works for cgroups v1
pub struct CgroupManager {
    driver: Option<CgroupDriver>,
}

impl CgroupManager {
    /// Creates a new cgroup manager with an unknown driver type
    pub fn new() -> Self { CgroupManager { driver: None } }

    /// Joins together the given slices to make a target cgroup, performing
    /// formatting conversions as necessary to target the current cgroup
    /// driver. If no driver is currently set, then tries to detect the
    /// current driver by seeing if the resultant formatted cgroup path from
    /// any of the drivers currently exists in the cgroup filesystem. This
    /// existence check is also performed if the current driver is known; if the
    /// cgroup was constructed and exists, returns Some(constructed path), else
    /// None
    pub fn make_cgroup(&mut self, slices: &[&str]) -> Option<String> {
        match self.driver {
            Some(driver) => {
                let path = make(driver, slices);
                match cgroup_exists(&path) {
                    true => Some(path),
                    false => None,
                }
            },
            None => {
                // Try each driver
                if let Some(path) = self.try_resolve(CgroupDriver::Systemd, slices) {
                    println!("Identified systemd as cgroup driver");
                    return Some(path);
                }

                if let Some(path) = self.try_resolve(CgroupDriver::Cgroupfs, slices) {
                    println!("Identified cgroupfs as cgroup driver");
                    return Some(path);
                }

                None
            },
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
    /// Differs from `make_cgroup` in that it allows for different slices to be
    /// specified for each driver
    pub fn make_cgroup_divided(
        &mut self,
        systemd_slices: &[&str],
        cgroupfs_slices: &[&str],
    ) -> Option<String> {
        match self.driver {
            Some(driver) => {
                let path = match driver {
                    CgroupDriver::Systemd => make(driver, systemd_slices),
                    CgroupDriver::Cgroupfs => make(driver, cgroupfs_slices),
                };
                match cgroup_exists(&path) {
                    true => Some(path),
                    false => None,
                }
            },
            None => {
                // Try each driver
                if let Some(path) = self.try_resolve(CgroupDriver::Systemd, systemd_slices) {
                    println!("Identified systemd as cgroup driver");
                    return Some(path);
                }

                if let Some(path) = self.try_resolve(CgroupDriver::Cgroupfs, cgroupfs_slices) {
                    println!("Identified cgroupfs as cgroup driver");
                    return Some(path);
                }

                None
            },
        }
    }

    /// Attempts to resolve the cgroup driver, by making the cgroup path for the
    /// given driver and then testing whether it exists
    fn try_resolve(&mut self, driver: CgroupDriver, slices: &[&str]) -> Option<String> {
        let path = make(driver, slices);
        match cgroup_exists(&path) {
            true => {
                self.driver = Some(driver);
                Some(path)
            },
            false => None,
        }
    }
}

/// Constructs a cgroup absolute path according to the style expected by the
/// given driver
pub fn make(driver: CgroupDriver, slices: &[&str]) -> String {
    match driver {
        CgroupDriver::Cgroupfs => make_cgroupfs(slices),
        CgroupDriver::Systemd => make_systemd(slices),
    }
}

const SYSTEMD_SLICE_SUFFIX: &str = ".slice";

/// Converts a vec of slice names such as vec!["kubepods", "burstable",
/// "pod1234-5678"] into a systemd-style cgroup path such as "/kubepods.slice/
/// kubepods-burstable.slice/kubepods-burstable-pod1234_5678.slice"
/// see [`kubernetes/kubelet/cm/cgroup_manager_linux.go:ToSystemd()`](https://github.com/kubernetes/kubernetes/blob/bb5ed1b79709c865d9aa86008048f19331530041/pkg/kubelet/cm/cgroup_manager_linux.go#L87-L103)
fn make_systemd(slices: &[&str]) -> String {
    if slices.len() == 0 || slices.len() == 1 && slices[0].len() == 0 {
        return String::from("/");
    }

    // First, escape systemd slices
    let escaped = slices.iter().map(|&s| escape_systemd(s));

    // Aggregate each slice with all previous to build final path
    let mut path: String = String::new();
    // Previously accumulated slices like "kubepods-burstable-"
    let mut accumulator: String = String::new();
    for slice in escaped {
        // Add the current slice to the path
        path += "/";
        path += &accumulator;
        path += &slice;
        path += SYSTEMD_SLICE_SUFFIX;

        // Add the current slice to the accumulator
        accumulator += &slice;
        accumulator += "-";
    }

    return path;
}

/// Escapes a cgroup slice to be in the style of Systemd cgroups
/// see [`kubernetes/kubelet/cm/cgroup_manager_linux.go:escapeSystemdCgroupName()`](https://github.com/kubernetes/kubernetes/blob/bb5ed1b79709c865d9aa86008048f19331530041/pkg/kubelet/cm/cgroup_manager_linux.go#L74-L76)
pub fn escape_systemd(slice: &str) -> String { slice.replace("-", "_") }

/// Converts a vec of slice names such as vec!["kubepods", "burstable",
/// "pod1234-5678"] into a systemd-style cgroup path such as "/kubepods/
/// burstable/pod1234_5678"
/// see [`kubernetes/kubelet/cm/cgroup_manager_linux.go:ToCgroupfs()`](https://github.com/kubernetes/kubernetes/blob/bb5ed1b79709c865d9aa86008048f19331530041/pkg/kubelet/cm/cgroup_manager_linux.go#L116-L118)
fn make_cgroupfs(slices: &[&str]) -> String { "/".to_owned() + &slices.join("/") }

pub const INVALID_MOUNT_MESSAGE: &str = "rAdvisor expects cgroups to be mounted in \
                                         /sys/fs/cgroup. If this is\nthe case, make sure that the \
                                         'cpuacct' resource controller has not been disabled.";

/// Checks if cgroups are mounted in /sys/fs/cgroup and if the cpuacct subsystem
/// is enabled (necessary for proper driver detection)
pub fn cgroups_mounted_properly() -> bool {
    // Use the raw subsystem directory to see if the expected cgroup hierarchy
    // exists
    cgroup_exists("")
}

const LINUX_CGROUP_ROOT: &str = "/sys/fs/cgroup";

/// Determines whether the given (absolute) cgroup exists in the virtual
/// filesystem **Note**: fails if cgroups aren't mounted in /sys/fs/cgroup or if
/// the cpuacct subsystem isn't enabled.
fn cgroup_exists(path: &str) -> bool {
    let full_path: String = format!("{}/cpuacct{}", LINUX_CGROUP_ROOT, path);
    match fs::metadata(full_path) {
        Err(_) => false,
        // As long as it exists and is a directory, assume all is good
        Ok(metadata) => metadata.is_dir(),
    }
}
