//! Cooling controller with temperature-based fan/pump curves.
//!
//! This module provides the logic for interpolating duty cycles from
//! temperature curves, supporting both liquid and CPU temperature sources.

/// Temperature source for calculating duty cycle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempSource {
    /// Internal liquid temp from Kraken sensor
    Liquid,
    /// External CPU temp from system sensors
    Cpu,
}

impl From<&str> for TempSource {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "cpu" => TempSource::Cpu,
            _ => TempSource::Liquid,
        }
    }
}

impl std::fmt::Display for TempSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TempSource::Liquid => write!(f, "Liquid"),
            TempSource::Cpu => write!(f, "CPU"),
        }
    }
}

/// Interpolate duty cycle from a temperature curve.
///
/// Returns duty percentage (0-100) for the given temperature.
/// Uses linear interpolation between curve points.
///
/// # Arguments
/// * `curve` - Slice of (temperature, duty) points
/// * `temp` - Current temperature in Celsius
///
/// # Returns
/// Duty cycle percentage (0-100)
pub fn interpolate_duty(curve: &[(u8, u8)], temp: u8) -> u8 {
    if curve.is_empty() {
        return 50; // Default 50% if no curve defined
    }

    // Sort by temperature (should already be sorted, but ensure consistency)
    let mut sorted: Vec<_> = curve.to_vec();
    sorted.sort_by_key(|(t, _)| *t);

    // Below minimum temp → use minimum duty
    if temp <= sorted[0].0 {
        return sorted[0].1;
    }

    // Above maximum temp → use maximum duty
    if temp >= sorted.last().unwrap().0 {
        return sorted.last().unwrap().1;
    }

    // Find the two surrounding points and interpolate
    for window in sorted.windows(2) {
        let (t1, d1) = window[0];
        let (t2, d2) = window[1];

        if temp >= t1 && temp <= t2 {
            // Linear interpolation
            let ratio = (temp - t1) as f32 / (t2 - t1) as f32;
            let duty = d1 as f32 + ratio * (d2 as f32 - d1 as f32);
            return duty.round() as u8;
        }
    }

    50 // Fallback (should not reach here)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolate_exact_point() {
        let curve = vec![(20, 25), (40, 50), (60, 100)];
        assert_eq!(interpolate_duty(&curve, 20), 25);
        assert_eq!(interpolate_duty(&curve, 40), 50);
        assert_eq!(interpolate_duty(&curve, 60), 100);
    }

    #[test]
    fn test_interpolate_middle() {
        let curve = vec![(20, 25), (40, 50), (60, 100)];
        // Midpoint between 20-40 is 30, duty should be ~37.5, rounded to 38
        assert_eq!(interpolate_duty(&curve, 30), 38);
    }

    #[test]
    fn test_interpolate_below_min() {
        let curve = vec![(20, 25), (40, 50)];
        assert_eq!(interpolate_duty(&curve, 10), 25); // Use min duty
        assert_eq!(interpolate_duty(&curve, 0), 25);
    }

    #[test]
    fn test_interpolate_above_max() {
        let curve = vec![(20, 25), (40, 50)];
        assert_eq!(interpolate_duty(&curve, 80), 50); // Use max duty
        assert_eq!(interpolate_duty(&curve, 100), 50);
    }

    #[test]
    fn test_empty_curve() {
        let curve: Vec<(u8, u8)> = vec![];
        assert_eq!(interpolate_duty(&curve, 50), 50); // Default fallback
    }

    #[test]
    fn test_temp_source_from_str() {
        assert_eq!(TempSource::from("Liquid"), TempSource::Liquid);
        assert_eq!(TempSource::from("liquid"), TempSource::Liquid);
        assert_eq!(TempSource::from("CPU"), TempSource::Cpu);
        assert_eq!(TempSource::from("cpu"), TempSource::Cpu);
        assert_eq!(TempSource::from("unknown"), TempSource::Liquid); // Default
    }
}
