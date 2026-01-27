//! NZXT Rust Devices Library
//!
//! A Rust driver for NZXT Kraken Z-series liquid coolers.
//!
//! # Features
//!
//! - Read device status (temperature, RPM, duty)
//! - Control fan and pump speeds
//! - Apply pre-defined or custom speed profiles
//!
//! # Example
//!
//! ```no_run
//! use nzxt_rust_devices::device::KrakenZ63;
//! use nzxt_rust_devices::protocol::Channel;
//! use nzxt_rust_devices::config::SpeedProfile;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Open and initialize the device
//!     let mut kraken = KrakenZ63::open()?;
//!     let firmware = kraken.initialize()?;
//!     println!("Connected! Firmware: {}", firmware);
//!
//!     // Read current status
//!     let status = kraken.get_status()?;
//!     println!("{}", status);
//!
//!     // Set fixed speeds
//!     kraken.set_pump_speed(80)?;
//!     kraken.set_fan_speed(50)?;
//!
//!     // Or use a profile
//!     let profile = SpeedProfile::Silent.to_duty_curve()?;
//!     // kraken.set_speed_profile(Channel::Fan, &profile)?;
//!
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod cooling;
pub mod device;
pub mod error;
pub mod protocol;
pub mod storage;
pub mod utils;

// Re-exports for convenience
pub use device::KrakenZ63;
pub use error::{KrakenError, Result};
pub use protocol::Channel;

// Re-exports for Radial Gauge Editor (GUI)
pub use storage::{StoredGradientStop, StoredRadialGaugeConfig};
pub use utils::radial_gauge::{GradientStop, RadialGaugeConfig};
pub use utils::stats_image::generate_radial_stats_image;
