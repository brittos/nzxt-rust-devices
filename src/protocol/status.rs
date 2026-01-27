//! Device status parsing for Kraken Z series.
//!
//! Parses HID response buffers into structured status data.
//! Offsets verified via raw HID debug analysis.

use crate::error::{KrakenError, Result};
use crate::protocol::commands::{
    RESP_FIRMWARE, RESP_SPEED_ACK, RESP_STATUS, RESP_STATUS_ALT, RESP_SUB_OK,
};

// =============================================================================
// Response Parsing Offsets (for RESP_STATUS messages)
// =============================================================================

/// Offset for liquid temperature integer part.
const OFFSET_TEMP_INT: usize = 15;
/// Offset for liquid temperature decimal part.
const OFFSET_TEMP_DEC: usize = 16;
/// Offset for pump RPM low byte.
const OFFSET_PUMP_RPM_LO: usize = 17;
/// Offset for pump RPM high byte.
const OFFSET_PUMP_RPM_HI: usize = 18;
/// Offset for pump duty percentage.
const OFFSET_PUMP_DUTY: usize = 19;
/// Offset for fan duty percentage.
const OFFSET_FAN_DUTY: usize = 20;
/// Offset for fan RPM low byte.
const OFFSET_FAN_RPM_LO: usize = 23;
/// Offset for fan RPM high byte.
const OFFSET_FAN_RPM_HI: usize = 24;

/// Invalid temperature sentinel value (firmware fault indicator).
const INVALID_TEMP_SENTINEL: [u8; 2] = [0xFF, 0xFF];

// =============================================================================
// Status Structures
// =============================================================================

/// Device status readings.
#[derive(Debug, Clone, PartialEq)]
pub struct DeviceStatus {
    /// Liquid coolant temperature in Celsius.
    pub liquid_temp_c: f32,
    /// Pump speed in RPM.
    pub pump_rpm: u16,
    /// Pump duty cycle as percentage (0-100).
    pub pump_duty: u8,
    /// Fan speed in RPM.
    pub fan_rpm: u16,
    /// Fan duty cycle as percentage (0-100).
    pub fan_duty: u8,
}

impl DeviceStatus {
    /// Parse a status response from the device.
    ///
    /// # Arguments
    /// * `buf` - 64-byte HID response buffer (must have header RESP_STATUS)
    ///
    /// # Returns
    /// Parsed device status or error if response is malformed.
    ///
    /// # Errors
    /// Returns `InvalidResponse` if temperature bytes are 0xFF 0xFF (firmware fault).
    /// Parse a status response from the device.
    ///
    /// # Arguments
    /// * `buf` - 64-byte HID response buffer
    ///
    /// # Returns
    /// Parsed device status or error if response is malformed.
    ///
    /// # Errors
    /// Returns `InvalidResponse` if temperature bytes are 0xFF 0xFF (firmware fault).
    pub fn parse(buf: &[u8]) -> Result<Self> {
        if buf.len() < 25 {
            return Err(KrakenError::InvalidResponse {
                message: format!(
                    "Buffer too short: {} bytes, expected at least 25",
                    buf.len()
                ),
            });
        }

        // Handle standard Z3 Status (RESP_STATUS = [0x75, 0x01])
        if buf[0] == RESP_STATUS[0] && buf[1] == RESP_STATUS[1] {
            // Check for firmware fault indicator
            if buf[OFFSET_TEMP_INT..=OFFSET_TEMP_DEC] == INVALID_TEMP_SENTINEL {
                return Err(KrakenError::InvalidResponse {
                    message: "Invalid temperature reading (0xFFFF). Possible firmware fault. \
                            Try resetting the device or updating firmware."
                        .into(),
                });
            }

            let liquid_temp_c = buf[OFFSET_TEMP_INT] as f32 + (buf[OFFSET_TEMP_DEC] as f32 / 10.0);

            // Pump RPM is little-endian
            let pump_rpm = (buf[OFFSET_PUMP_RPM_HI] as u16) << 8 | (buf[OFFSET_PUMP_RPM_LO] as u16);
            let pump_duty = buf[OFFSET_PUMP_DUTY];

            // Fan RPM is little-endian
            let fan_rpm = (buf[OFFSET_FAN_RPM_HI] as u16) << 8 | (buf[OFFSET_FAN_RPM_LO] as u16);
            let fan_duty = buf[OFFSET_FAN_DUTY];

            return Ok(DeviceStatus {
                liquid_temp_c,
                pump_rpm,
                pump_duty,
                fan_rpm,
                fan_duty,
            });
        }

        // Handle alternative Z3 Status (RESP_STATUS_ALT or RESP_SPEED_ACK)
        // Based on debug analysis:
        // Temp: buf[2]
        // Pump RPM: buf[5] | buf[6] << 8 (Little Endian)
        // Pump Duty: buf[7] (?)
        if (buf[0] == RESP_STATUS_ALT || buf[0] == RESP_SPEED_ACK[0]) && buf[1] == RESP_SUB_OK {
            let liquid_temp_c = buf[2] as f32 + (buf[3] as f32 / 10.0);
            let pump_rpm = (buf[6] as u16) << 8 | (buf[5] as u16);
            let pump_duty = buf[7];

            // Fan RPM seems to be at offset 15/16 (Little Endian) or 14/15 based on "SFN62 p" structure?
            // Debug log analysis:
            // [08-15] ... 112 2  => 0x02 0x70 => 624 RPM
            // [08-15] ... 1 1    => 0x01 0x01 => 257 RPM
            // The values are at buf[14] and buf[15].
            let fan_rpm = (buf[15] as u16) << 8 | (buf[14] as u16);

            // Speculative Fan Duty at 13 (0x20=32%)?
            let fan_duty = buf[13];

            return Ok(DeviceStatus {
                liquid_temp_c,
                pump_rpm,
                pump_duty,
                fan_rpm,
                fan_duty,
            });
        }

        Err(KrakenError::InvalidResponse {
            message: format!("Unknown status header: [{:#04x}, {:#04x}]", buf[0], buf[1]),
        })
    }
}

/// Firmware version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirmwareVersion {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
}

impl FirmwareVersion {
    /// Parse firmware version from a RESP_FIRMWARE response.
    ///
    /// # Arguments
    /// * `buf` - HID response buffer (must start with RESP_FIRMWARE)
    pub fn parse(buf: &[u8]) -> Result<Self> {
        if buf.len() < 0x14 {
            return Err(KrakenError::InvalidResponse {
                message: "Firmware response too short".into(),
            });
        }

        if buf[0] != RESP_FIRMWARE[0] || buf[1] != RESP_FIRMWARE[1] {
            return Err(KrakenError::InvalidResponse {
                message: format!(
                    "Invalid firmware response header: [{:#04x}, {:#04x}], expected [{:#04x}, {:#04x}]",
                    buf[0], buf[1], RESP_FIRMWARE[0], RESP_FIRMWARE[1]
                ),
            });
        }

        Ok(FirmwareVersion {
            major: buf[0x11],
            minor: buf[0x12],
            patch: buf[0x13],
        })
    }
}

impl std::fmt::Display for FirmwareVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl std::fmt::Display for DeviceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "+-----------------------------------+")?;
        writeln!(f, "|      NZXT Kraken Z63 Status       |")?;
        writeln!(f, "+-----------------------------------+")?;
        writeln!(
            f,
            "|  Liquid Temp:    {:>5.1} C          |",
            self.liquid_temp_c
        )?;
        writeln!(f, "+-----------------------------------+")?;
        writeln!(f, "|  Pump Speed:    {:>5} RPM         |", self.pump_rpm)?;
        writeln!(f, "|  Pump Duty:       {:>3}%            |", self.pump_duty)?;
        writeln!(f, "+-----------------------------------+")?;
        writeln!(f, "|  Fan Speed:     {:>5} RPM         |", self.fan_rpm)?;
        writeln!(f, "|  Fan Duty:        {:>3}%            |", self.fan_duty)?;
        writeln!(f, "+-----------------------------------+")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_status() {
        let mut buf = [0u8; 64];
        // Set header to RESP_STATUS
        buf[0] = RESP_STATUS[0];
        buf[1] = RESP_STATUS[1];

        // Temperature: 32.5°C
        buf[15] = 32;
        buf[16] = 5;
        // Pump RPM: 2500 (little-endian)
        buf[17] = 0xC4; // low byte
        buf[18] = 0x09; // high byte -> 0x09C4 = 2500
        // Pump duty: 75%
        buf[19] = 75;
        // Fan duty: 50%
        buf[20] = 50;
        // Fan RPM: 1200 (little-endian) = 0x04B0
        buf[23] = 0xB0; // low byte
        buf[24] = 0x04; // high byte

        let status = DeviceStatus::parse(&buf).unwrap();
        assert_eq!(status.liquid_temp_c, 32.5);
        assert_eq!(status.pump_rpm, 2500);
        assert_eq!(status.pump_duty, 75);
        assert_eq!(status.fan_rpm, 1200);
        assert_eq!(status.fan_duty, 50);
    }

    #[test]
    fn test_invalid_temp() {
        let mut buf = [0u8; 64];
        // Set header to RESP_STATUS
        buf[0] = RESP_STATUS[0];
        buf[1] = RESP_STATUS[1];
        // Set invalid temp
        buf[15] = 0xFF;
        buf[16] = 0xFF;

        let result = DeviceStatus::parse(&buf);
        assert!(result.is_err());
    }

    #[test]
    fn test_firmware_parse() {
        let mut buf = [0u8; 64];
        buf[0] = RESP_FIRMWARE[0];
        buf[1] = RESP_FIRMWARE[1];
        buf[0x11] = 2;
        buf[0x12] = 1;
        buf[0x13] = 5;

        let fw = FirmwareVersion::parse(&buf).unwrap();
        assert_eq!(fw.to_string(), "2.1.5");
    }
}
