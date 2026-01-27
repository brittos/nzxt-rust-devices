//! Cooling control module.
//!
//! Provides temperature-based fan/pump curve interpolation and control logic.

mod controller;

pub use controller::{TempSource, interpolate_duty};
