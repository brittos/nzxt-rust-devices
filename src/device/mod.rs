//! Device abstraction layer for NZXT Kraken coolers.
//!
//! Provides high-level device discovery and control interfaces.

pub mod bucket_manager;
pub mod bulk;
pub mod kraken;

pub use bucket_manager::BucketManager;

pub use bulk::{BulkDevice, is_bulk_available};
pub use kraken::KrakenZ63;
