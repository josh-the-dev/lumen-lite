//! Packet encoding for Corsair's "V2" peripheral protocol, used by the
//! K55 RGB PRO and Scimitar RGB Elite.
//!
//! Pure functions with no I/O, so every byte we send is unit-tested.
//! Outgoing packets are 65 bytes: a zero HID report ID, then a 64-byte
//! payload. Responses arrive as 64 bytes with the report ID stripped.

use std::fmt;

pub const VENDOR_ID: u16 = 0x1B1C;
/// Vendor-defined HID usage page that carries the control channel.
pub const USAGE_PAGE: u16 = 0xFF42;
pub const WRITE_SIZE: usize = 65;
pub const READ_SIZE: usize = 64;

const WRITE_WIRED: u8 = 0x08;
const CMD_SET: u8 = 0x01;
const CMD_GET: u8 = 0x02;

/// Device properties, read with GET and written with SET.
pub mod property {
    /// Who drives the lighting: the onboard profile or the host.
    pub const MODE: u16 = 0x0003;
}

pub type Packet = [u8; WRITE_SIZE];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The device plays the profile saved in its own memory.
    Hardware,
    /// The host drives the lighting. If nothing is sending colors (for
    /// example iCUE crashed or hasn't started), the lights stay dark.
    Software,
}

impl Mode {
    #[must_use]
    pub fn value(self) -> u32 {
        match self {
            Mode::Hardware => 0x01,
            Mode::Software => 0x02,
        }
    }

    #[must_use]
    pub fn from_value(value: u32) -> Option<Self> {
        match value {
            0x01 => Some(Mode::Hardware),
            0x02 => Some(Mode::Software),
            _ => None,
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Mode::Hardware => "hardware (onboard profile)",
            Mode::Software => "software (host-controlled)",
        })
    }
}

fn header(command: u8, address: u16) -> Packet {
    let mut packet = [0; WRITE_SIZE];
    packet[1] = WRITE_WIRED;
    packet[2] = command;
    packet[3..5].copy_from_slice(&address.to_le_bytes());
    packet
}

#[must_use]
pub fn get(address: u16) -> Packet {
    header(CMD_GET, address)
}

#[must_use]
pub fn set(address: u16, value: u32) -> Packet {
    let mut packet = header(CMD_SET, address);
    packet[5..9].copy_from_slice(&value.to_le_bytes());
    packet
}

#[must_use]
pub fn set_mode(mode: Mode) -> Packet {
    set(property::MODE, mode.value())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseError {
    TooShort(usize),
    Status(u8),
}

impl fmt::Display for ResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResponseError::TooShort(len) => write!(f, "response too short ({len} bytes)"),
            ResponseError::Status(0x01) => f.write_str("device rejected the value"),
            ResponseError::Status(0x03) => f.write_str("device reported failure"),
            ResponseError::Status(0x05) => f.write_str("device doesn't support this property"),
            ResponseError::Status(code) => write!(f, "device returned status {code:#04x}"),
        }
    }
}

impl std::error::Error for ResponseError {}

/// Checks a response's status byte and returns its 32-bit value.
///
/// # Errors
///
/// Fails if the response is truncated or carries a non-zero status.
pub fn parse_response(buf: &[u8]) -> Result<u32, ResponseError> {
    if buf.len() < 7 {
        return Err(ResponseError::TooShort(buf.len()));
    }
    if buf[2] != 0 {
        return Err(ResponseError::Status(buf[2]));
    }
    Ok(u32::from_le_bytes([buf[3], buf[4], buf[5], buf[6]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(packet: &Packet) -> &[u8] {
        &packet[..9]
    }

    #[test]
    fn get_mode_packet() {
        let packet = get(property::MODE);
        assert_eq!(payload(&packet), [0x00, 0x08, 0x02, 0x03, 0, 0, 0, 0, 0]);
        assert!(packet[9..].iter().all(|&b| b == 0));
    }

    #[test]
    fn set_hardware_mode_packet() {
        let packet = set_mode(Mode::Hardware);
        assert_eq!(payload(&packet), [0x00, 0x08, 0x01, 0x03, 0, 0x01, 0, 0, 0]);
        assert!(packet[9..].iter().all(|&b| b == 0));
    }

    #[test]
    fn set_software_mode_packet() {
        let packet = set_mode(Mode::Software);
        assert_eq!(payload(&packet), [0x00, 0x08, 0x01, 0x03, 0, 0x02, 0, 0, 0]);
    }

    #[test]
    fn parses_ok_response() {
        let mut buf = [0u8; READ_SIZE];
        buf[3] = 0x02;
        assert_eq!(parse_response(&buf), Ok(2));
        assert_eq!(Mode::from_value(2), Some(Mode::Software));
    }

    #[test]
    fn parses_status_error() {
        let mut buf = [0u8; READ_SIZE];
        buf[2] = 0x05;
        assert_eq!(parse_response(&buf), Err(ResponseError::Status(0x05)));
    }

    #[test]
    fn rejects_short_response() {
        assert_eq!(parse_response(&[0; 4]), Err(ResponseError::TooShort(4)));
    }

    #[test]
    fn mode_round_trips() {
        for mode in [Mode::Hardware, Mode::Software] {
            assert_eq!(Mode::from_value(mode.value()), Some(mode));
        }
        assert_eq!(Mode::from_value(0), None);
    }
}
