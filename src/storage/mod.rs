//! Profile storage and persistence module.
//!
//! Handles saving and loading profiles to/from disk.
//! Includes defaults management and profile persistence.

pub mod defaults;
pub mod profiles;
pub mod types;

// Re-export commonly used items
pub use defaults::{ensure_defaults_exist, get_defaults_path, get_profile, update_fixed};
pub use profiles::*;
pub use types::*;
