//! HID I/O for connected devices.

use std::ffi::CString;

use anyhow::{Context, Result, anyhow};
use hidapi::{HidApi, HidDevice};

use crate::devices::{self, Model};
use crate::protocol::{self, Mode, Packet, READ_SIZE, USAGE_PAGE, VENDOR_ID, property};

const RESPONSE_TIMEOUT_MS: i32 = 50;
const DRAIN_TIMEOUT_MS: i32 = 3;
const MAX_DRAIN_READS: usize = 16;
const ATTEMPTS: usize = 3;

/// A supported device seen during enumeration. Not opened yet.
#[derive(Debug, Clone)]
pub struct Found {
    pub model: &'static Model,
    pub path: CString,
    serial: Option<String>,
    usage: u16,
}

/// Re-enumerates HID devices, fetching details only for Corsair ones.
///
/// `HidApi::refresh_devices` reads string descriptors from every HID
/// device on the system, which measured ~100 ms per pass; filtering by
/// vendor cuts that to well under 1 ms and leaves other devices alone.
///
/// # Errors
///
/// Fails if Windows can't list HID interfaces.
pub fn rescan(api: &mut HidApi) -> Result<()> {
    api.reset_devices()?;
    api.add_devices(VENDOR_ID, 0)
        .context("couldn't enumerate HID devices")
}

/// Lists supported devices, one entry per physical device.
#[must_use]
pub fn discover(api: &HidApi) -> Vec<Found> {
    let mut found: Vec<Found> = api
        .device_list()
        .filter(|info| info.vendor_id() == VENDOR_ID && info.usage_page() == USAGE_PAGE)
        .filter_map(|info| {
            Some(Found {
                model: devices::by_pid(info.product_id())?,
                path: info.path().to_owned(),
                serial: info.serial_number().map(str::to_owned),
                usage: info.usage(),
            })
        })
        .collect();

    // A device can expose several collections on the vendor page; the
    // command channel is the lowest usage.
    found.sort_by(|a, b| (a.model.pid, &a.serial, a.usage).cmp(&(b.model.pid, &b.serial, b.usage)));
    found.dedup_by(|a, b| a.model.pid == b.model.pid && a.serial == b.serial);
    found
}

pub struct Device {
    hid: HidDevice,
}

impl Device {
    /// # Errors
    ///
    /// Fails if the device was unplugged or Windows refuses access.
    pub fn open(api: &HidApi, found: &Found) -> Result<Self> {
        let hid = api
            .open_path(&found.path)
            .with_context(|| format!("couldn't open {}", found.model.name))?;
        Ok(Self { hid })
    }

    /// Reads which lighting mode the device is in.
    ///
    /// # Errors
    ///
    /// Fails on I/O errors, no response, an error status, or an unknown mode.
    pub fn mode(&self) -> Result<Mode> {
        let value = self.request(&protocol::get(property::MODE))?;
        Mode::from_value(value).ok_or_else(|| anyhow!("unknown lighting mode {value:#x}"))
    }

    /// # Errors
    ///
    /// Fails on I/O errors, no response, or if the device rejects the mode.
    pub fn set_mode(&self, mode: Mode) -> Result<()> {
        self.request(&protocol::set_mode(mode)).map(drop)
    }

    fn request(&self, packet: &Packet) -> Result<u32> {
        for _ in 0..ATTEMPTS {
            self.drain();
            self.hid.write(packet).context("write failed")?;
            let mut buf = [0; READ_SIZE];
            let len = self
                .hid
                .read_timeout(&mut buf, RESPONSE_TIMEOUT_MS)
                .context("read failed")?;
            if len > 0 {
                return Ok(protocol::parse_response(&buf[..len])?);
            }
        }
        Err(anyhow!("no response after {ATTEMPTS} attempts"))
    }

    /// Discards stale reports so the next read is the reply to our request.
    fn drain(&self) {
        let mut buf = [0; READ_SIZE];
        for _ in 0..MAX_DRAIN_READS {
            if !matches!(self.hid.read_timeout(&mut buf, DRAIN_TIMEOUT_MS), Ok(len) if len > 0) {
                break;
            }
        }
    }
}
