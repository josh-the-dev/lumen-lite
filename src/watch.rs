//! Keeps every supported device in hardware lighting mode.
//!
//! The lights go dark when a device is left in software mode with nothing
//! sending it colors: iCUE crashed, started late, or lost the device during
//! boot or sleep. Reading the mode is one small request per device, so we
//! check every few seconds and hand control back to the onboard profile
//! whenever it's wrong. This also covers wake-from-sleep and replugging
//! without any Windows event plumbing.

use std::collections::HashMap;
use std::ffi::CString;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use hidapi::HidApi;

use crate::device::{self, Device, Found};
use crate::protocol::Mode;

pub const INTERVAL: Duration = Duration::from_secs(2);

/// If we have to correct a device this many times within `WINDOW`, another
/// program (usually iCUE) is actively driving it. Back off instead of
/// fighting over it.
const MAX_CORRECTIONS: usize = 3;
const WINDOW: Duration = Duration::from_secs(60);
const BACKOFF: Duration = Duration::from_secs(10 * 60);

#[derive(Default)]
struct Tracker {
    corrections: Vec<Instant>,
    paused_until: Option<Instant>,
    last_error: Option<String>,
}

/// Runs forever, reporting what it does through `log`. Per-device failures
/// are logged and retried on the next pass.
///
/// # Errors
///
/// Fails only if the HID subsystem can't be initialised.
pub fn run(mut log: impl FnMut(&str)) -> Result<()> {
    let mut api = HidApi::new().context("couldn't initialise HID")?;
    let mut trackers: HashMap<CString, Tracker> = HashMap::new();
    let mut last_scan_error = None;
    log("watching for Corsair devices");

    loop {
        // A failed scan leaves the device list empty; skip the pass rather
        // than forget every tracker and its backoff state.
        let scan_error = device::rescan(&mut api).err().map(|err| format!("{err:#}"));
        if let Some(err) = &scan_error {
            if scan_error != last_scan_error {
                log(&format!("device scan failed: {err}"));
            }
            last_scan_error = scan_error;
            thread::sleep(INTERVAL);
            continue;
        }
        last_scan_error = None;
        let found = device::discover(&api);

        trackers.retain(|path, _| found.iter().any(|f| &f.path == path));
        for f in &found {
            let tracker = trackers.entry(f.path.clone()).or_insert_with(|| {
                log(&format!("{}: connected", f.model.name));
                Tracker::default()
            });
            let result = check(&api, f, tracker, &mut log);
            // Only log an error when it changes, so a persistent problem
            // doesn't write a line every two seconds.
            let error = result.err().map(|err| format!("{err:#}"));
            if error.is_some() && error != tracker.last_error {
                log(&format!(
                    "{}: {}",
                    f.model.name,
                    error.as_deref().unwrap_or_default()
                ));
            }
            tracker.last_error = error;
        }

        thread::sleep(INTERVAL);
    }
}

fn check(
    api: &HidApi,
    found: &Found,
    tracker: &mut Tracker,
    log: &mut impl FnMut(&str),
) -> Result<()> {
    let now = Instant::now();
    if tracker.paused_until.is_some_and(|until| now < until) {
        return Ok(());
    }

    // Reopened every pass on purpose: a long-lived handle would buffer
    // unsolicited input reports between polls (more than `drain` clears),
    // and would go stale across sleep or replugging.
    let device = Device::open(api, found)?;
    if device.mode()? == Mode::Hardware {
        return Ok(());
    }

    tracker
        .corrections
        .retain(|at| now.duration_since(*at) < WINDOW);
    if tracker.corrections.len() >= MAX_CORRECTIONS {
        tracker.corrections.clear();
        tracker.paused_until = Some(now + BACKOFF);
        log(&format!(
            "{}: something keeps switching it to software mode (is iCUE running?), pausing for {} min",
            found.model.name,
            BACKOFF.as_secs() / 60
        ));
        return Ok(());
    }

    device.set_mode(Mode::Hardware)?;
    tracker.corrections.push(now);
    log(&format!(
        "{}: was in software mode, restored onboard lighting",
        found.model.name
    ));
    Ok(())
}
