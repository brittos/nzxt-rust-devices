//! Parsing utilities for CLI arguments and configuration values.
//!
//! This module provides reusable parsing functions for common input formats
//! used throughout the application.

use crate::config::SpeedProfile;
use crate::error::{KrakenError, Result};
use crate::protocol::Channel;

// =============================================================================
// Color Parsing
// =============================================================================

/// Parse a hex color string into RGB components.
///
/// Accepts formats: `#RRGGBB` or `RRGGBB`
///
/// # Arguments
/// * `hex` - Hex color string
///
/// # Returns
/// Tuple of (red, green, blue) values (0-255 each)
///
/// # Example
/// ```
/// use nzxt_rust_devices::utils::parsing::parse_hex_color;
///
/// let (r, g, b) = parse_hex_color("#FF5500").unwrap();
/// assert_eq!(r, 255);
/// assert_eq!(g, 85);
/// assert_eq!(b, 0);
/// ```
pub fn parse_hex_color(hex: &str) -> Result<(u8, u8, u8)> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return Err(KrakenError::InvalidInput(format!(
            "Invalid color hex: {}",
            hex
        )));
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    Ok((r, g, b))
}

// =============================================================================
// LCD Mode Parsing
// =============================================================================

/// Parse LCD display mode string to mode ID.
///
/// # Arguments
/// * `mode` - Mode string from CAM export or user input
///
/// # Returns
/// Mode ID for the device (1-5)
///
/// # Supported Modes
/// - "cpu temperature" → 1
/// - "liquid temperature" → 2
/// - "gpu temperature" → 3
/// - "dual infographic" → 4
/// - "gif" → 5
pub fn parse_lcd_mode_string(mode: &str) -> u8 {
    match mode.to_lowercase().as_str() {
        "liquid temperature" => 2,
        "cpu temperature" => 1,
        "gpu temperature" => 3,
        "dual infographic" => 4,
        "gif" => 5,
        _ => {
            eprintln!(
                "⚠️ Warning: Unknown LCD mode '{}', defaulting to Liquid Temp (2)",
                mode
            );
            2
        }
    }
}

// =============================================================================
// Speed Profile Parsing
// =============================================================================

/// Parse a speed profile name into a SpeedProfile enum.
///
/// # Arguments
/// * `name` - Profile name: "silent", "performance", or "fixed:XX"
///
/// # Returns
/// The corresponding SpeedProfile variant
///
/// # Example
/// ```
/// use nzxt_rust_devices::utils::parsing::parse_speed_profile;
/// use nzxt_rust_devices::config::SpeedProfile;
///
/// let profile = parse_speed_profile("silent").unwrap();
/// assert!(matches!(profile, SpeedProfile::Silent));
///
/// let fixed = parse_speed_profile("fixed:75").unwrap();
/// assert!(matches!(fixed, SpeedProfile::Fixed(75)));
/// ```
pub fn parse_speed_profile(name: &str) -> Result<SpeedProfile> {
    let lower = name.to_lowercase();

    if lower == "silent" {
        return Ok(SpeedProfile::Silent);
    }

    if lower == "performance" {
        return Ok(SpeedProfile::Performance);
    }

    if let Some(rest) = lower.strip_prefix("fixed:") {
        let duty: u8 = rest.parse().map_err(|_| {
            KrakenError::InvalidInput("Invalid duty value. Use 'fixed:XX' where XX is 0-100".into())
        })?;
        return Ok(SpeedProfile::Fixed(duty));
    }

    Err(KrakenError::InvalidInput(format!(
        "Unknown profile '{}'. Use: silent, performance, or fixed:XX",
        name
    )))
}

// =============================================================================
// Channel Parsing
// =============================================================================

/// Parse a channel name string into a Channel enum.
///
/// # Arguments
/// * `name` - Channel name: "fan" or "pump"
///
/// # Returns
/// The corresponding Channel variant
pub fn parse_channel(name: &str) -> Result<Channel> {
    match name.to_lowercase().as_str() {
        "fan" => Ok(Channel::Fan),
        "pump" => Ok(Channel::Pump),
        _ => Err(KrakenError::InvalidInput(format!(
            "Unknown channel '{}'. Use: fan or pump",
            name
        ))),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color_with_hash() {
        let (r, g, b) = parse_hex_color("#FF0000").unwrap();
        assert_eq!((r, g, b), (255, 0, 0));
    }

    #[test]
    fn test_parse_hex_color_without_hash() {
        let (r, g, b) = parse_hex_color("00FF00").unwrap();
        assert_eq!((r, g, b), (0, 255, 0));
    }

    #[test]
    fn test_parse_hex_color_invalid() {
        assert!(parse_hex_color("FFF").is_err());
        assert!(parse_hex_color("").is_err());
    }

    #[test]
    fn test_parse_lcd_mode() {
        assert_eq!(parse_lcd_mode_string("cpu temperature"), 1);
        assert_eq!(parse_lcd_mode_string("Liquid Temperature"), 2);
        assert_eq!(parse_lcd_mode_string("GPU TEMPERATURE"), 3);
        assert_eq!(parse_lcd_mode_string("unknown"), 2); // default
    }

    #[test]
    fn test_parse_speed_profile() {
        assert!(matches!(
            parse_speed_profile("silent").unwrap(),
            SpeedProfile::Silent
        ));
        assert!(matches!(
            parse_speed_profile("PERFORMANCE").unwrap(),
            SpeedProfile::Performance
        ));
        assert!(matches!(
            parse_speed_profile("fixed:50").unwrap(),
            SpeedProfile::Fixed(50)
        ));
    }

    #[test]
    fn test_parse_channel() {
        assert!(matches!(parse_channel("fan").unwrap(), Channel::Fan));
        assert!(matches!(parse_channel("PUMP").unwrap(), Channel::Pump));
        assert!(parse_channel("invalid").is_err());
    }
}
