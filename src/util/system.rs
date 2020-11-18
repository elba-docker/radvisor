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
