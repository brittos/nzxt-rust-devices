//! System sensor utilities for reading CPU and GPU temperatures.
//!
//! This module provides a wrapper around `sysinfo` for detecting and reading
//! system sensor values, with specific focus on CPU and GPU temperature sensors.

use sysinfo::Components;

// =============================================================================
// Sensor Info
// =============================================================================

/// Information about a detected sensor.
#[derive(Debug, Clone)]
pub struct SensorInfo {
    /// Sensor label/name.
    pub label: String,
    /// Current temperature in Celsius.
    pub temperature: f32,
    /// Critical temperature threshold (if available).
    pub critical: Option<f32>,
}

// =============================================================================
// System Sensors
// =============================================================================

/// Wrapper for system sensor access with caching.
pub struct SystemSensors {
    components: Components,
}

impl SystemSensors {
    /// Create a new SystemSensors instance with refreshed sensor list.
    pub fn new() -> Self {
        Self {
            components: Components::new_with_refreshed_list(),
        }
    }

    /// Refresh all sensor values.
    pub fn refresh(&mut self) {
        self.components.refresh(true);
    }

    /// Get the total number of detected sensors.
    pub fn count(&self) -> usize {
        self.components.len()
    }

    /// Find CPU temperature using common sensor label patterns.
    ///
    /// Searches for sensors with labels containing:
    /// - "cpu", "package", "core", "tdie", "computer"
    ///
    /// Returns the temperature of the first matching sensor.
    pub fn find_cpu_temp(&self) -> Option<f32> {
        self.components
            .iter()
            .find(|c| {
                let label = c.label().to_lowercase();
                label.contains("cpu")
                    || label.contains("package")
                    || label.contains("core")
                    || label.contains("tdie")
                    || label.contains("computer") // Fallback for some Windows systems
            })
            .and_then(|c| c.temperature())
    }

    /// Find GPU temperature using common sensor label patterns.
    ///
    /// Searches for sensors with labels containing:
    /// - "gpu", "nvidia", "amd", "edge"
    ///
    /// Returns the temperature of the first matching sensor.
    pub fn find_gpu_temp(&self) -> Option<f32> {
        self.components
            .iter()
            .find(|c| {
                let label = c.label().to_lowercase();
                label.contains("gpu")
                    || label.contains("nvidia")
                    || label.contains("amd")
                    || label.contains("edge")
            })
            .and_then(|c| c.temperature())
    }

    /// Get all detected sensors as a list of SensorInfo.
    pub fn list_all(&self) -> Vec<SensorInfo> {
        self.components
            .iter()
            .map(|c| SensorInfo {
                label: c.label().to_string(),
                temperature: c.temperature().unwrap_or(0.0),
                critical: c.critical(),
            })
            .collect()
    }

    /// Find the first sensor that matches one of the CPU patterns.
    /// Returns both the sensor info and whether it was found.
    pub fn find_cpu_sensor(&self) -> Option<SensorInfo> {
        self.components
            .iter()
            .find(|c| {
                let label = c.label().to_lowercase();
                label.contains("cpu")
                    || label.contains("package")
                    || label.contains("core")
                    || label.contains("tdie")
                    || label.contains("computer")
            })
            .map(|c| SensorInfo {
                label: c.label().to_string(),
                temperature: c.temperature().unwrap_or(0.0),
                critical: c.critical(),
            })
    }
}

impl Default for SystemSensors {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Convenience Functions
// =============================================================================

/// Get CPU temperature using default sensor detection.
///
/// This is a convenience function that creates a new SystemSensors instance
/// and finds the CPU temperature. For repeated calls, prefer using
/// SystemSensors directly.
pub fn get_cpu_temp() -> Option<f32> {
    let sensors = SystemSensors::new();
    sensors.find_cpu_temp()
}

/// Get GPU temperature using default sensor detection.
///
/// This is a convenience function that creates a new SystemSensors instance
/// and finds the GPU temperature. For repeated calls, prefer using
/// SystemSensors directly.
pub fn get_gpu_temp() -> Option<f32> {
    let sensors = SystemSensors::new();
    sensors.find_gpu_temp()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_sensors_creation() {
        let sensors = SystemSensors::new();
        // Just verify it doesn't panic - actual sensors depend on system
        let _ = sensors.count();
    }

    #[test]
    fn test_list_all_sensors() {
        let sensors = SystemSensors::new();
        let list = sensors.list_all();
        // list may be empty on systems without sensors (CI environments)
        // Just verify it returns a valid Vec without panicking
        let _ = list;
    }

    #[test]
    fn test_sensor_info_debug() {
        let info = SensorInfo {
            label: "Test".to_string(),
            temperature: 45.0,
            critical: Some(100.0),
        };
        // Verify Debug trait works
        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("Test"));
    }
}
