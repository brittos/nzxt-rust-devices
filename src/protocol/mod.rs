//! HID protocol implementation for NZXT Kraken devices.
//!
//! This module contains the low-level HID command constants, builders,
//! and response parsing logic based on reverse-engineered protocol from liquidctl.

pub mod commands;
pub mod status;

pub use commands::*;
pub use status::*;
