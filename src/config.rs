//! Speed profile configurations for Kraken coolers.
//!
//! Provides pre-defined profiles and custom profile building.

use crate::error::Result;
use crate::protocol::{CURVE_POINTS, interpolate_profile};

// =============================================================================
// Speed Profiles
// =============================================================================

/// Pre-defined speed profile.
#[derive(Debug, Clone, PartialEq)]
pub enum SpeedProfile {
    /// Silent mode - low speeds, ramps up only at high temps.
    Silent,
    /// Performance mode - aggressive cooling curve.
    Performance,
    /// Fixed speed for all temperatures.
    Fixed(u8),
    /// Custom temperature/duty curve.
    Custom(Vec<(u8, u8)>),
}

impl SpeedProfile {
    /// Convert this profile to a 40-point duty curve (20°C to 59°C).
    pub fn to_duty_curve(&self) -> Result<[u8; CURVE_POINTS]> {
        match self {
            SpeedProfile::Silent => interpolate_profile(&PROFILE_SILENT),
            SpeedProfile::Performance => interpolate_profile(&PROFILE_PERFORMANCE),
            SpeedProfile::Fixed(duty) => Ok([*duty; CURVE_POINTS]),
            SpeedProfile::Custom(points) => interpolate_profile(points),
        }
    }

    /// Get profile name for display.
    pub fn name(&self) -> &'static str {
        match self {
            SpeedProfile::Silent => "Silent",
            SpeedProfile::Performance => "Performance",
            SpeedProfile::Fixed(_) => "Fixed",
            SpeedProfile::Custom(_) => "Custom",
        }
    }
}

impl std::fmt::Display for SpeedProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpeedProfile::Fixed(duty) => write!(f, "Fixed ({}%)", duty),
            _ => write!(f, "{}", self.name()),
        }
    }
}

// =============================================================================
// Pre-defined Profile Curves
// =============================================================================

/// Silent profile - minimal noise, ramps at 50°C+.
/// Based on NZXT CAM "Silent" preset.
pub const PROFILE_SILENT: [(u8, u8); 8] = [
    (20, 25),
    (30, 25),
    (40, 25),
    (45, 25),
    (50, 55),
    (55, 75),
    (58, 90),
    (59, 100),
];

/// Performance profile - aggressive cooling.
/// Based on NZXT CAM "Performance" preset.
pub const PROFILE_PERFORMANCE: [(u8, u8); 6] =
    [(20, 50), (30, 55), (40, 65), (50, 80), (55, 90), (59, 100)];

/// Pump Silent profile - maintains minimum flow.
pub const PROFILE_PUMP_SILENT: [(u8, u8); 5] = [(20, 70), (35, 70), (45, 80), (55, 95), (59, 100)];

/// Pump Performance profile - maximum cooling.
pub const PROFILE_PUMP_PERFORMANCE: [(u8, u8); 4] = [(20, 80), (40, 85), (50, 95), (59, 100)];

// =============================================================================
// LCD Profiles
// =============================================================================

/// LCD visual configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct LcdProfile {
    pub brightness: u8,
    /// Visual mode (0=Blank, 2=Liquid Temp, 4=Dual Infographic, 5=Gif?)
    pub mode: u8,
    /// Memory bucket index to display
    pub bucket: u8,
}

impl LcdProfile {
    pub const OFF: Self = Self {
        brightness: 0,
        mode: 0,
        bucket: 0,
    };

    pub const NIGHT: Self = Self {
        brightness: 10,
        mode: 2, // Liquid Temp
        bucket: 0,
    };

    pub const DAY: Self = Self {
        brightness: 75,
        mode: 4, // Dual Infographic
        bucket: 0,
    };

    pub const MAX: Self = Self {
        brightness: 100,
        mode: 2,
        bucket: 0,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_silent_profile() {
        let curve = SpeedProfile::Silent.to_duty_curve().unwrap();
        // At 20°C should be 25%
        assert_eq!(curve[0], 25);
        // At 59°C should be 100%
        assert_eq!(curve[39], 100);
    }

    #[test]
    fn test_fixed_profile() {
        let curve = SpeedProfile::Fixed(60).to_duty_curve().unwrap();
        assert!(curve.iter().all(|&d| d == 60));
    }

    #[test]
    fn test_custom_profile() {
        let custom = SpeedProfile::Custom(vec![(20, 30), (40, 50), (59, 100)]);
        let curve = custom.to_duty_curve().unwrap();
        assert_eq!(curve[0], 30);
        assert_eq!(curve[39], 100);
    }
}
