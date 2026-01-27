//! Custom error types for NZXT Kraken devices.
//!
//! This module provides fine-grained error handling for device communication,
//! protocol parsing, and configuration validation.

use thiserror::Error;

/// Main error type for Kraken device operations.
#[derive(Error, Debug)]
pub enum KrakenError {
    /// Device not found during enumeration.
    #[error("Kraken Z63 not found. Check USB connection and permissions.")]
    DeviceNotFound,

    /// Multiple devices found when expecting one.
    #[error("Multiple Kraken devices found. Use --serial to specify which one.")]
    MultipleDevicesFound,

    /// HID communication error.
    #[error("HID communication error: {0}")]
    HidError(#[from] hidapi::HidError),

    /// Invalid or malformed response from device.
    #[error("Invalid response from device: {message}")]
    InvalidResponse { message: String },

    /// Duty cycle value out of valid range.
    #[error("Invalid duty cycle {value}% for {channel}. Valid range: {min}%-{max}%")]
    InvalidDuty {
        channel: String,
        value: u8,
        min: u8,
        max: u8,
    },

    /// Temperature value out of valid range for profile.
    #[error("Invalid temperature {0}°C. Valid range: 20-59°C")]
    InvalidTemperature(u8),

    /// Speed profile has invalid format.
    #[error("Invalid speed profile: {0}")]
    InvalidProfile(String),

    /// Device not initialized.
    #[error("Device not initialized. Call initialize() first.")]
    NotInitialized,

    /// Timeout waiting for device response.
    #[error("Timeout waiting for device response")]
    Timeout,

    /// Generic invalid input error.
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// Result type alias for Kraken operations.
pub type Result<T> = std::result::Result<T, KrakenError>;
