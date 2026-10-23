//! lumen-lite: keeps Corsair keyboard and mouse lighting on, without iCUE.

pub mod device;
pub mod devices;
pub mod protocol;
pub mod watch;

use std::time::SystemTime;

/// Prefixes a log message with the current UTC time.
#[must_use]
pub fn timestamped(msg: &str) -> String {
    format!(
        "{} {msg}",
        humantime::format_rfc3339_seconds(SystemTime::now())
    )
}
