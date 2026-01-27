//! FIFO Bucket Manager for LCD memory management.
//!
//! Manages 15 buckets (0-14) using FIFO (First In, First Out) strategy.
//! Bucket 15 is reserved for system use.

use std::collections::VecDeque;

use crate::device::KrakenZ63;

/// Maximum number of buckets available for user images.
const MAX_BUCKETS: usize = 16;

/// FIFO-based bucket manager for LCD memory.
///
/// Tracks which buckets are in use and maintains arrival order.
/// When all buckets are full, automatically frees the oldest one.
pub struct BucketManager {
    /// Tracks which buckets (0-14) are currently occupied.
    occupied: [bool; MAX_BUCKETS],
    /// FIFO queue maintaining order of bucket usage.
    queue: VecDeque<u8>,
}

impl BucketManager {
    /// Creates a new bucket manager with all buckets free.
    pub fn new() -> Self {
        Self {
            occupied: [false; MAX_BUCKETS],
            queue: VecDeque::with_capacity(MAX_BUCKETS),
        }
    }

    /// Creates a bucket manager synchronized with current device state.
    ///
    /// Queries the device to see which buckets are already occupied.
    pub fn from_device(kraken: &KrakenZ63) -> Result<Self, crate::KrakenError> {
        let mut manager = Self::new();

        if let Ok(buckets) = kraken.query_all_buckets() {
            for (idx, exists, _, _) in buckets {
                if idx < MAX_BUCKETS as u8 && exists {
                    manager.occupied[idx as usize] = true;
                    manager.queue.push_back(idx);
                }
            }
        }

        Ok(manager)
    }

    /// Acquires a bucket for use.
    ///
    /// Returns the bucket index to use. If all buckets are occupied,
    /// automatically frees the oldest one (FIFO).
    pub fn acquire(&mut self, kraken: &KrakenZ63) -> u8 {
        // Defined high water mark to trigger cleanup
        // When we reach this many buckets, we start cleaning up old ones
        // This prevents us from ever reaching the full 15/16 limit
        const HIGH_WATER_MARK: usize = 12;

        let occupied_count = self.queue.len();

        // 1. Try to find a free bucket IF we are below the high water mark
        if occupied_count < HIGH_WATER_MARK {
            for i in 0..MAX_BUCKETS {
                if !self.occupied[i] {
                    self.occupied[i] = true;
                    self.queue.push_back(i as u8);
                    return i as u8;
                }
            }
        }

        // 2. If we reached high water mark (or somehow full), free the 3 oldest
        // This keeps the system cycling between ~9 and 12 buckets

        let mut freed_bucket = None;

        // Try to free up to 8 buckets
        for i in 0..8 {
            if let Some(oldest) = self.queue.pop_front() {
                // Delete from device
                let _ = kraken.delete_bucket(oldest);

                // If it's the first one (oldest), we'll reuse it now
                if i == 0 {
                    freed_bucket = Some(oldest);
                    self.queue.push_back(oldest); // Move to back (active)
                } else {
                    // For the others, mark as free (remove from occupied array)
                    // They will be picked up by find_free_bucket in next calls
                    self.occupied[oldest as usize] = false;
                }
            }
        }

        if let Some(bucket) = freed_bucket {
            return bucket;
        }

        // Fallback (shouldn't happen unless queue was empty)
        0
    }

    /// Releases a specific bucket.
    pub fn release(&mut self, idx: u8) {
        if (idx as usize) < MAX_BUCKETS {
            self.occupied[idx as usize] = false;
            self.queue.retain(|&x| x != idx);
        }
    }

    /// Returns the number of occupied buckets.
    pub fn occupied_count(&self) -> usize {
        self.occupied.iter().filter(|&&x| x).count()
    }

    /// Clears all bucket tracking (call after delete_all_buckets).
    pub fn clear(&mut self) {
        self.occupied = [false; MAX_BUCKETS];
        self.queue.clear();
    }
}

impl Default for BucketManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_manager() {
        let manager = BucketManager::new();
        assert_eq!(manager.occupied_count(), 0);
    }
}
