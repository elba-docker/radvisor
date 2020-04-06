//! Function interfaces that sit in front of system-specific implementations

/// Gets the nanosecond unix timestamp for a stat read
pub fn nano_ts() -> u128 { time::nano_ts() }

/// Gets the second unix timestamp for the stat filename
pub fn second_ts() -> u64 { time::second_ts() }

/// Gets the total number of cores on the system. On Linux, this includes
/// disabled ones
///
/// **Note**: Operates independently of the scheduling settings on the
/// collection process
pub fn num_cores() -> u64 { cpu::num_cores() }

/// Gets the number of available cores on the system. On Linux, this excludes
/// those that have been disabled.
///
/// **Note**: Operates independently of the scheduling settings on the
/// collection process
pub fn num_available_cores() -> u64 { cpu::num_available_cores() }

#[cfg(target_os = "linux")]
mod time {
    use libc::{clock_gettime, timespec, CLOCK_REALTIME};
    use std::mem;

    /// Invokes `clock_gettime` from time.h in libc to get a `timespec` struct
    fn get_time() -> timespec {
        let mut tp: timespec = unsafe { mem::zeroed() };
        unsafe {
            clock_gettime(CLOCK_REALTIME, &mut tp);
        }
        tp
    }

    pub fn nano_ts() -> u128 {
        let tp = get_time();
        (tp.tv_nsec as u128) + ((tp.tv_sec as u128) * 1_000_000_000)
    }

    pub fn second_ts() -> u64 { get_time().tv_sec as u64 }
}

#[cfg(target_os = "windows")]
mod time {
    use std::mem;
    use winapi::shared::minwindef::FILETIME;
    use winapi::um::sysinfoapi;

    /// Number of seconds between the start of the Windows epoch (Jan 1. 1601)
    /// and the start of the Unix epoch (Jan 1. 1970)
    const EPOCH_DIFFERENCE: u64 = 11644473600;
    /// Number of nanoseconds between the start of the Windows epoch (Jan 1.
    /// 1601) and the start of the Unix epoch (Jan 1. 1970)
    const NANO_EPOCH_DIFFERENCE: u128 = (EPOCH_DIFFERENCE as u128) * 1_000_000_000;
    /// Number of nanoseconds per tick
    const TICK_LENGTH: u128 = 100;
    /// Number of ticks per second
    const TICK: i64 = 1_000_000_000 / (TICK_LENGTH as i64);

    /// Executes a win32 call, returning a timestamp that represents the number
    /// of 100 ns intervals since January 1, 1601 (UTC). Invokes
    /// `GetSystemTimePreciseAsFileTime` from Sysinfoapi.h in
    /// [winapi](https://docs.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getsystemtimepreciseasfiletime)
    fn file_timestamp() -> i64 {
        let mut file_time: FILETIME = unsafe { mem::zeroed() };
        unsafe {
            sysinfoapi::GetSystemTimePreciseAsFileTime(&mut file_time);
        }

        (file_time.dwLowDateTime as i64) + (file_time.dwHighDateTime as i64) << 32
    }

    pub fn nano_ts() -> u128 { (file_timestamp() as u128) * TICK_LENGTH + NANO_EPOCH_DIFFERENCE }

    pub fn second_ts() -> u64 { (file_timestamp() / TICK) as u64 + EPOCH_DIFFERENCE }
}

#[cfg(target_os = "windows")]
mod cpu {
    use winapi::shared::minwindef::DWORD;
    use winapi::um::winbase::GetActiveProcessorCount;
    use winapi::um::winnt::ALL_PROCESSOR_GROUPS;

    /// Uses `GetActiveProcessorCount` from Winbase.h in
    /// [winapi](https://docs.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getactiveprocessorcount)
    fn num_processors() -> u64 {
        let count: DWORD = unsafe { GetActiveProcessorCount(GroupNumber: WORD) };
        count as u64
    }

    pub fn num_cores() -> u64 { num_processors() }

    pub fn num_available_cores() -> u64 { num_processors() }
}

#[cfg(target_os = "linux")]
mod cpu {
    use libc::{c_long, sysconf, _SC_NPROCESSORS_CONF, _SC_NPROCESSORS_ONLN};

    pub fn num_cores() -> u64 {
        let count: c_long = unsafe { sysconf(_SC_NPROCESSORS_CONF) };
        count as u64
    }

    pub fn num_available_cores() -> u64 {
        let count: c_long = unsafe { sysconf(_SC_NPROCESSORS_ONLN) };
        count as u64
    }
}
