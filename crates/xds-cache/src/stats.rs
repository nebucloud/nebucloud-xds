//! Cache statistics and metrics.

use std::sync::atomic::{AtomicU64, Ordering};

/// Statistics for cache operations.
///
/// All counters are atomic and can be safely accessed from multiple threads.
#[derive(Debug, Default)]
pub struct CacheStats {
    /// Number of snapshot sets.
    snapshots_set: AtomicU64,
    /// Number of snapshot gets (hits).
    snapshot_hits: AtomicU64,
    /// Number of snapshot gets (misses).
    snapshot_misses: AtomicU64,
    /// Number of snapshot clears.
    snapshots_cleared: AtomicU64,
    /// Number of watch notifications sent.
    notifications_sent: AtomicU64,
}

impl CacheStats {
    /// Create new cache statistics.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a snapshot set operation.
    #[inline]
    pub fn record_set(&self) {
        self.snapshots_set.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a snapshot hit.
    #[inline]
    pub fn record_hit(&self) {
        self.snapshot_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a snapshot miss.
    #[inline]
    pub fn record_miss(&self) {
        self.snapshot_misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a snapshot clear.
    #[inline]
    pub fn record_clear(&self) {
        self.snapshots_cleared.fetch_add(1, Ordering::Relaxed);
    }

    /// Record notifications sent.
    #[inline]
    pub fn record_notifications(&self, count: u64) {
        self.notifications_sent.fetch_add(count, Ordering::Relaxed);
    }

    /// Get total snapshots set.
    #[inline]
    pub fn snapshots_set(&self) -> u64 {
        self.snapshots_set.load(Ordering::Relaxed)
    }

    /// Get total snapshot hits.
    #[inline]
    pub fn snapshot_hits(&self) -> u64 {
        self.snapshot_hits.load(Ordering::Relaxed)
    }

    /// Get total snapshot misses.
    #[inline]
    pub fn snapshot_misses(&self) -> u64 {
        self.snapshot_misses.load(Ordering::Relaxed)
    }

    /// Get total snapshots cleared.
    #[inline]
    pub fn snapshots_cleared(&self) -> u64 {
        self.snapshots_cleared.load(Ordering::Relaxed)
    }

    /// Get total notifications sent.
    #[inline]
    pub fn notifications_sent(&self) -> u64 {
        self.notifications_sent.load(Ordering::Relaxed)
    }

    /// Calculate hit rate (0.0 to 1.0).
    pub fn hit_rate(&self) -> f64 {
        let hits = self.snapshot_hits() as f64;
        let total = hits + self.snapshot_misses() as f64;
        if total == 0.0 {
            0.0
        } else {
            hits / total
        }
    }

    /// Reset all statistics.
    pub fn reset(&self) {
        self.snapshots_set.store(0, Ordering::Relaxed);
        self.snapshot_hits.store(0, Ordering::Relaxed);
        self.snapshot_misses.store(0, Ordering::Relaxed);
        self.snapshots_cleared.store(0, Ordering::Relaxed);
        self.notifications_sent.store(0, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_stats_basic() {
        let stats = CacheStats::new();

        stats.record_set();
        stats.record_hit();
        stats.record_hit();
        stats.record_miss();

        assert_eq!(stats.snapshots_set(), 1);
        assert_eq!(stats.snapshot_hits(), 2);
        assert_eq!(stats.snapshot_misses(), 1);
        assert!((stats.hit_rate() - 0.666).abs() < 0.01);
    }

    #[test]
    fn cache_stats_reset() {
        let stats = CacheStats::new();
        stats.record_set();
        stats.reset();
        assert_eq!(stats.snapshots_set(), 0);
    }
}
