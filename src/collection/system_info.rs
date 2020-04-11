use crate::util;
use gethostname::gethostname;
use serde::Serialize;

/// Represents mostly-static metadata about a system and its network/hardware
/// configuration
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SystemInfo {
    pub os_type:          Option<String>,
    pub os_release:       Option<String>,
    pub distribution:     Option<Distribution>,
    pub memory_total:     Option<u64>,
    pub swap_total:       Option<u64>,
    pub hostname:         Option<String>,
    pub cpu_count:        u64,
    pub cpu_online_count: u64,
    pub cpu_speed:        Option<u64>,
}

impl SystemInfo {
    /// Gets the current system info, requesting fresh values for each field.
    #[must_use]
    pub fn get() -> Self {
        let mem_info = sys_info::mem_info();
        Self {
            os_type:          sys_info::os_type().ok(),
            os_release:       sys_info::os_release().ok(),
            distribution:     Distribution::try_get(),
            memory_total:     mem_info.as_ref().map(|m| m.total).ok(),
            swap_total:       mem_info.as_ref().map(|m| m.swap_total).ok(),
            hostname:         gethostname().into_string().ok(),
            cpu_count:        util::num_cores(),
            cpu_online_count: util::num_available_cores(),
            cpu_speed:        sys_info::cpu_speed().ok(),
        }
    }
}

/// Represents metadata about a Linux distribution, compliant with
/// [`os-release`](https://www.freedesktop.org/software/systemd/man/os-release.html)
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Distribution {
    pub id:               Option<String>,
    pub id_like:          Option<String>,
    pub name:             Option<String>,
    pub pretty_name:      Option<String>,
    pub version:          Option<String>,
    pub version_id:       Option<String>,
    pub version_codename: Option<String>,
    pub cpe_name:         Option<String>,
    pub build_id:         Option<String>,
    pub variant:          Option<String>,
    pub variant_id:       Option<String>,
}

impl Distribution {
    /// Attempts to get the Linux distribution metadata, succeeding only on
    /// Linux and if the values can be retrieved properly
    #[must_use]
    pub fn try_get() -> Option<Self> { Self::get_inner() }

    #[cfg(not(target_os = "linux"))]
    fn get_inner() -> Option<Self> { None }

    #[cfg(target_os = "linux")]
    fn get_inner() -> Option<Self> {
        match sys_info::linux_os_release() {
            Err(_) => None,
            Ok(info) => {
                let sys_info::LinuxOSReleaseInfo {
                    id,
                    id_like,
                    name,
                    pretty_name,
                    version,
                    version_id,
                    version_codename,
                    cpe_name,
                    build_id,
                    variant,
                    variant_id,
                    ..
                } = info;
                Some(Self {
                    id,
                    id_like,
                    name,
                    pretty_name,
                    version,
                    version_id,
                    version_codename,
                    cpe_name,
                    build_id,
                    variant,
                    variant_id,
                })
            },
        }
    }
}
