use std::time::{SystemTime};

/// Gets the nanosecond unix timestamp for a stat read
pub fn nano_ts() -> u128 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(t) => t.as_nanos(),
        Err(_) => 0,
    }
}

/// Gets the second unix timestamp for the stat filename
pub fn second_ts() -> u64 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(t) => t.as_secs(),
        Err(_) => 0,
    }
}
