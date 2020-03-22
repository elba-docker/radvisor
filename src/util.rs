use libc::{timespec, clock_gettime, CLOCK_REALTIME};

pub static N: u8 = '\n' as u8;
pub static R: u8 = '\r' as u8;
pub static S: u8 = ' ' as u8;

/// Returns true if the given char is a line feed, carriage return, or normal
/// space
#[inline]
pub fn is_space(c: u8) -> bool {
    c == N || c == R || c == S
}

/// Returns true if the given char is a line feed or carriage return
#[inline]
pub fn is_newline(c: u8) -> bool {
    c == N || c == R
}

/// Gets the nanosecond unix timestamp for a stat read
pub fn nano_ts() -> u128 {
    let mut tp: timespec = timespec {
        tv_sec: 0,
        tv_nsec: 0
    };
    // Invoke clock_gettime from time.h in libc
    unsafe { clock_gettime(CLOCK_REALTIME, &mut tp); }
    (tp.tv_nsec as u128) + ((tp.tv_sec as u128) * 1000000000)
}

/// Gets the second unix timestamp for the stat filename
pub fn second_ts() -> u64 {
    let mut tp: timespec = timespec {
        tv_sec: 0,
        tv_nsec: 0
    };
    // Invoke clock_gettime from time.h in libc
    unsafe { clock_gettime(CLOCK_REALTIME, &mut tp); }
    tp.tv_sec as u64
}
