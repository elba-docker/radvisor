//! Function interfaces that sit in front of system-specific implementations

use std::convert::TryFrom;

/// Gets the nanosecond unix timestamp for a stat read
#[must_use]
pub fn nano_ts() -> u128 { time::nano_ts() }

/// Gets the second unix timestamp for the stat filename
#[must_use]
pub fn second_ts() -> u64 { time::second_ts() }

/// Gets the total number of cores on the system. On Linux, this includes
/// disabled ones
///
/// **Note**: Operates independently of the scheduling settings on the
/// collection process
#[must_use]
pub fn num_cores() -> u64 { cpu::num_cores() }

/// Gets the number of available cores on the system. On Linux, this excludes
/// those that have been disabled.
///
/// **Note**: Operates independently of the scheduling settings on the
/// collection process
#[must_use]
pub fn num_available_cores() -> u64 { cpu::num_available_cores() }

/// Attempts to get the width of the given terminal type (in characters),
/// returning None if no applicable width can be found
#[must_use]
pub fn terminal_width(stream: atty::Stream) -> Option<usize> { terminal::width(stream) }

/// Attempts to map a number range into another, falling back to 0 if the
/// conversion failed
pub fn remap<S, D: TryFrom<S> + From<bool>>(num: S) -> D {
    match D::try_from(num) {
        Ok(d) => d,
        Err(_) => D::from(false),
    }
}

#[cfg(target_os = "linux")]
mod time {
    use super::remap;
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
        remap::<_, u128>(tp.tv_nsec) + (remap::<_, u128>(tp.tv_sec) * 1_000_000_000)
    }

    pub fn second_ts() -> u64 { remap::<_, u64>(get_time().tv_sec) }
}

#[cfg(target_os = "windows")]
mod time {
    use super::remap;
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

    pub fn nano_ts() -> u128 {
        remap::<_, u128>(file_timestamp()) * TICK_LENGTH + NANO_EPOCH_DIFFERENCE
    }

    pub fn second_ts() -> u64 { remap::<_, u64>(file_timestamp() / TICK) + EPOCH_DIFFERENCE }
}

#[cfg(target_os = "windows")]
mod cpu {
    use winapi::shared::minwindef::DWORD;
    use winapi::um::winbase::GetActiveProcessorCount;
    use winapi::um::winnt::ALL_PROCESSOR_GROUPS;

    /// Uses `GetActiveProcessorCount` from Winbase.h in
    /// [winapi](https://docs.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getactiveprocessorcount)
    fn num_processors() -> u64 {
        let count: DWORD = unsafe { GetActiveProcessorCount(ALL_PROCESSOR_GROUPS) };
        count as u64
    }

    pub fn num_cores() -> u64 { num_processors() }

    pub fn num_available_cores() -> u64 { num_processors() }
}

#[cfg(target_os = "linux")]
mod cpu {
    use super::remap;
    use libc::{c_long, sysconf, _SC_NPROCESSORS_CONF, _SC_NPROCESSORS_ONLN};

    pub fn num_cores() -> u64 {
        let count: c_long = unsafe { sysconf(_SC_NPROCESSORS_CONF) };
        remap::<_, u64>(count)
    }

    pub fn num_available_cores() -> u64 {
        let count: c_long = unsafe { sysconf(_SC_NPROCESSORS_ONLN) };
        remap::<_, u64>(count)
    }
}

#[cfg(target_os = "linux")]
mod terminal {
    use std::mem;

    pub fn width(stream: atty::Stream) -> Option<usize> {
        unsafe {
            let mut winsize: libc::winsize = mem::zeroed();

            // Resolve correct fileno for the stream type
            let fileno = match stream {
                atty::Stream::Stdout => libc::STDOUT_FILENO,
                _ => libc::STDERR_FILENO,
            };

            if libc::ioctl(fileno, libc::TIOCGWINSZ, &mut winsize) < 0 {
                return None;
            }
            if winsize.ws_col > 0 {
                Some(winsize.ws_col as usize)
            } else {
                None
            }
        }
    }
}

#[cfg(target_os = "windows")]
mod terminal {
    use std::{cmp, mem, ptr};
    use winapi::um::fileapi::*;
    use winapi::um::handleapi::*;
    use winapi::um::processenv::*;
    use winapi::um::winbase::*;
    use winapi::um::wincon::*;
    use winapi::um::winnt::*;

    pub fn width(_stream: atty::Stream) -> Option<usize> {
        unsafe {
            let stdout = GetStdHandle(STD_ERROR_HANDLE);
            let mut csbi: CONSOLE_SCREEN_BUFFER_INFO = mem::zeroed();
            if GetConsoleScreenBufferInfo(stdout, &mut csbi) != 0 {
                return Some((csbi.srWindow.Right - csbi.srWindow.Left) as usize);
            }

            // On mintty/msys/cygwin based terminals, the above fails with
            // INVALID_HANDLE_VALUE. Use an alternate method which works
            // in that case as well.
            let h = CreateFileA(
                "CONOUT$\0".as_ptr() as *const CHAR,
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                ptr::null_mut(),
                OPEN_EXISTING,
                0,
                ptr::null_mut(),
            );
            if h == INVALID_HANDLE_VALUE {
                return None;
            }

            let mut csbi: CONSOLE_SCREEN_BUFFER_INFO = mem::zeroed();
            let rc = GetConsoleScreenBufferInfo(h, &mut csbi);
            CloseHandle(h);
            if rc != 0 {
                let width = (csbi.srWindow.Right - csbi.srWindow.Left) as usize;
                // Unfortunately cygwin/mintty does not set the size of the
                // backing console to match the actual window size. This
                // always reports a size of 80 or 120 (not sure what
                // determines that). Use a conservative max of 60 which should
                // work in most circumstances. ConEmu does some magic to
                // resize the console correctly, but there's no reasonable way
                // to detect which kind of terminal we are running in, or if
                // GetConsoleScreenBufferInfo returns accurate information.
                return Some(cmp::min(60, width));
            }
            None
        }
    }
}
