//! HID command definitions and builders for Kraken Z series.
//!
//! Protocol based on reverse-engineering from liquidctl project:
//! https://github.com/liquidctl/liquidctl/blob/main/liquidctl/driver/kraken3.py

use crate::error::{KrakenError, Result};

// =============================================================================
// Constants
// =============================================================================

/// HID report length for reads and writes.
pub const HID_REPORT_LENGTH: usize = 64;

/// NZXT Vendor ID.
pub const NZXT_VID: u16 = 0x1E71;

/// Kraken Z53/Z63/Z73 Product ID.
pub const KRAKEN_Z3_PID: u16 = 0x3008;

/// Critical temperature threshold (device enforced).
pub const CRITICAL_TEMPERATURE: u8 = 59;

/// Minimum temperature for speed curves.
pub const MIN_CURVE_TEMP: u8 = 20;

/// Number of duty points in a speed curve (20°C to 59°C inclusive).
pub const CURVE_POINTS: usize = 40;

// =============================================================================
// HID Commands
// =============================================================================

/// Request firmware version info.
pub const CMD_FIRMWARE_INFO: [u8; 2] = [0x10, 0x01];

/// Initialize device - step 1 (set update interval).
/// Format: [0x70, 0x02, 0x01, 0xB8, interval]
/// Default interval = 0x01 (500ms updates)
pub const CMD_INIT_INTERVAL: [u8; 5] = [0x70, 0x02, 0x01, 0xB8, 0x01];

/// Initialize device - step 2 (complete initialization).
pub const CMD_INIT_COMPLETE: [u8; 2] = [0x70, 0x01];

/// Request LCD info (Z series only).
pub const CMD_LCD_INFO: [u8; 2] = [0x30, 0x01];

/// Request LED/lighting info.
pub const CMD_LED_INFO: [u8; 2] = [0x20, 0x03];

/// Request device status (temperature, RPM, duty).
/// This command triggers the device to send a status message.
pub const CMD_REQUEST_STATUS: [u8; 2] = [0x74, 0x01];

/// Set host telemetry info (CPU/GPU temperature).
/// Format: [0x73, 0x01, cpu_temp, gpu_temp, ...]
pub const CMD_SET_HOST_INFO: [u8; 2] = [0x73, 0x01];

/// Set LCD brightness/orientation. Header: [0x30, 0x02, 0x01, brightness, 0x0, 0x0, 0x1, orientation]
/// Orientation: 0=0°, 1=90°, 2=180°, 3=270°
pub const CMD_SET_LCD_CONFIG_HEADER: [u8; 3] = [0x30, 0x02, 0x01];

/// Set LCD brightness. Deprecated in favor of CMD_SET_LCD_CONFIG_HEADER.
pub const CMD_SET_BRIGHTNESS_HEADER: [u8; 3] = [0x30, 0x02, 0x01];

/// Set LCD visual mode. Header: [0x38, 0x01, mode, index]
/// mode: 1=CPU, 2=GPU, 3=Liquid, 4=Dual (depending on firmware)
/// index: Layout/Sensor selection or Memory Bucket
pub const CMD_SET_VISUAL_MODE_HEADER: [u8; 2] = [0x38, 0x01];

/// Set speed header: [0x72, channel_id, ...]
pub const CMD_SET_SPEED_HEADER: u8 = 0x72;

/// Setup bucket command (0x32).
pub const CMD_BUCKET_OP: u8 = 0x32;

/// Bucket operation: Set/Create (0x01).
pub const OP_BUCKET_SET: u8 = 0x01;

/// Bucket operation: Delete (0x02).
pub const OP_BUCKET_DELETE: u8 = 0x02;

/// Bucket operation: Start Write (0x03).
pub const OP_BUCKET_WRITE_START: u8 = 0x03;

/// Bucket operation: Finish Write (0x04).
pub const OP_BUCKET_WRITE_FINISH: u8 = 0x04;

/// Query bucket command (0x30 0x04).
pub const CMD_BUCKET_QUERY: [u8; 2] = [0x30, 0x04];

/// Start bulk transfer command (0x36 0x01).
pub const CMD_BULK_START: u8 = 0x36;
pub const OP_BULK_START: u8 = 0x01;

/// End bulk transfer command (0x36 0x02).
pub const OP_BULK_END: u8 = 0x02;

/// Protocol header for sending pixel data bulk info (0x36 0x01).
/// Deprecated: use CMD_BULK_START / OP_BULK_START instead.
pub const CMD_SETUP_BULK_HEADER: [u8; 2] = [0x36, 0x01];

// =============================================================================
// Response Headers (from device)
// =============================================================================

/// Bucket setup response header (0x33).
pub const RESP_BUCKET_SETUP: u8 = 0x33;

/// Bulk transfer response header (0x37).
pub const RESP_BULK: u8 = 0x37;

/// Visual mode response header (0x39).
pub const RESP_VISUAL_MODE: u8 = 0x39;

/// Speed profile ACK header (0xFF 0x01).
pub const RESP_SPEED_ACK: [u8; 2] = [0xFF, 0x01];

/// Device status response header (0x75 0x01).
/// This is the header for periodic status messages containing temp, RPM, duty.
pub const RESP_STATUS: [u8; 2] = [0x75, 0x01];

/// Alternative status response header (0x71).
/// Some firmware versions use this instead of 0x75.
pub const RESP_STATUS_ALT: u8 = 0x71;

/// Firmware info response header (0x11 0x01).
pub const RESP_FIRMWARE: [u8; 2] = [0x11, 0x01];

/// LED info response header (0x21).
pub const RESP_LED_INFO: u8 = 0x21;

/// Common response sub-byte indicating success/valid response (0x01).
/// Most response headers are followed by this byte: [header, 0x01]
pub const RESP_SUB_OK: u8 = 0x01;

// =============================================================================
// Speed Channels
// =============================================================================

/// Speed control channel identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// Pump channel - minimum 20%, maximum 100%.
    Pump,
    /// Fan channel - minimum 0%, maximum 100%.
    Fan,
}

impl Channel {
    /// Get the HID channel identifier bytes.
    pub const fn id(&self) -> u8 {
        match self {
            Channel::Pump => 0x01,
            Channel::Fan => 0x02,
        }
    }

    /// Get the minimum duty cycle for this channel.
    pub const fn min_duty(&self) -> u8 {
        match self {
            Channel::Pump => 20,
            Channel::Fan => 0,
        }
    }

    /// Get the maximum duty cycle for this channel.
    pub const fn max_duty(&self) -> u8 {
        100
    }

    /// Validate a duty cycle value for this channel.
    pub fn validate_duty(&self, duty: u8) -> Result<u8> {
        let min = self.min_duty();
        let max = self.max_duty();

        if duty < min || duty > max {
            return Err(KrakenError::InvalidDuty {
                channel: format!("{:?}", self),
                value: duty,
                min,
                max,
            });
        }

        Ok(duty)
    }
}

impl std::fmt::Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Channel::Pump => write!(f, "Pump"),
            Channel::Fan => write!(f, "Fan"),
        }
    }
}

// =============================================================================
// Command Builders
// =============================================================================

/// Build a speed profile command.
///
/// The device expects 40 duty values corresponding to temperatures 20°C to 59°C.
/// Each duty value is a percentage (0-100).
///
/// # Arguments
/// * `channel` - The channel to set (Pump or Fan)
/// * `duties` - Array of 40 duty percentages (temperatures 20-59°C)
///
/// # Returns
/// A 64-byte HID report ready to send to the device.
pub fn build_speed_profile_cmd(
    channel: Channel,
    duties: &[u8; CURVE_POINTS],
) -> [u8; HID_REPORT_LENGTH] {
    let mut buf = [0u8; HID_REPORT_LENGTH];

    // Command header: CMD_SET_SPEED_HEADER + channel ID
    buf[0] = CMD_SET_SPEED_HEADER;
    buf[1] = channel.id();

    // Duty values for temperatures 20-59°C
    // Start at offset 2 (after 0x72 and channel ID)
    buf[2..2 + CURVE_POINTS].copy_from_slice(duties);

    buf
}

/// Build a fixed speed command.
///
/// Creates a flat speed profile where all temperature points use the same duty.
///
/// # Arguments
/// * `channel` - The channel to set (Pump or Fan)
/// * `duty` - Fixed duty percentage
///
/// # Returns
/// A 64-byte HID report ready to send to the device.
pub fn build_fixed_speed_cmd(channel: Channel, duty: u8) -> Result<[u8; HID_REPORT_LENGTH]> {
    // Validate duty for channel
    let duty = channel.validate_duty(duty)?;

    // Create flat curve with same duty at all temperatures
    let duties = [duty; CURVE_POINTS];

    Ok(build_speed_profile_cmd(channel, &duties))
}

/// Interpolate a sparse profile into a full 40-point curve.
///
/// # Arguments
/// * `profile` - Sparse profile as (temperature, duty) pairs
///
/// # Returns
/// Full 40-point duty curve for temperatures 20-59°C.
pub fn interpolate_profile(profile: &[(u8, u8)]) -> Result<[u8; CURVE_POINTS]> {
    if profile.is_empty() {
        return Err(KrakenError::InvalidProfile(
            "Profile cannot be empty".into(),
        ));
    }

    // Validate and sort profile by temperature
    let mut sorted: Vec<(u8, u8)> = profile.to_vec();
    sorted.sort_by_key(|(temp, _)| *temp);

    // Validate temperature range
    for (temp, _) in &sorted {
        if *temp < MIN_CURVE_TEMP || *temp > CRITICAL_TEMPERATURE {
            return Err(KrakenError::InvalidTemperature(*temp));
        }
    }

    let mut duties = [0u8; CURVE_POINTS];

    for (i, temp) in (MIN_CURVE_TEMP..=CRITICAL_TEMPERATURE).enumerate() {
        // Find surrounding points for interpolation
        let duty = if let Some(&(_, duty)) = sorted.iter().find(|(t, _)| *t == temp) {
            // Exact match
            duty
        } else {
            // Interpolate between surrounding points
            let lower = sorted.iter().rfind(|(t, _)| *t < temp);
            let upper = sorted.iter().find(|(t, _)| *t > temp);

            match (lower, upper) {
                (Some(&(t1, d1)), Some(&(t2, d2))) => {
                    // Linear interpolation
                    let ratio = (temp - t1) as f32 / (t2 - t1) as f32;
                    (d1 as f32 + ratio * (d2 as f32 - d1 as f32)).round() as u8
                }
                (Some(&(_, duty)), None) => duty, // Use last known value
                (None, Some(&(_, duty))) => duty, // Use first known value
                (None, None) => unreachable!(),   // Profile is not empty
            }
        };

        duties[i] = duty;
    }

    Ok(duties)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_ids() {
        assert_eq!(Channel::Pump.id(), 0x01);
        assert_eq!(Channel::Fan.id(), 0x02);
    }

    #[test]
    fn test_duty_validation() {
        // Pump: 20-100%
        assert!(Channel::Pump.validate_duty(20).is_ok());
        assert!(Channel::Pump.validate_duty(100).is_ok());
        assert!(Channel::Pump.validate_duty(19).is_err());

        // Fan: 0-100%
        assert!(Channel::Fan.validate_duty(0).is_ok());
        assert!(Channel::Fan.validate_duty(100).is_ok());
        assert!(Channel::Fan.validate_duty(101).is_err());
    }

    #[test]
    fn test_fixed_speed_cmd() {
        let cmd = build_fixed_speed_cmd(Channel::Pump, 50).unwrap();
        assert_eq!(cmd[0], CMD_SET_SPEED_HEADER);
        assert_eq!(cmd[1], 0x01);
        // Duty values start at index 2
        assert!(cmd[2..42].iter().all(|&d| d == 50));
    }

    #[test]
    fn test_interpolate_profile() {
        let profile = [(20, 25), (40, 50), (59, 100)];
        let curve = interpolate_profile(&profile).unwrap();

        assert_eq!(curve[0], 25); // 20°C
        assert_eq!(curve[20], 50); // 40°C
        assert_eq!(curve[39], 100); // 59°C
    }
}
