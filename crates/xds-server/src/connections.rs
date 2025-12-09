//! Connection tracking and limits.
//!
//! This module provides connection tracking and limiting functionality
//! to prevent resource exhaustion from too many concurrent connections.
//!
//! # Example
//!
//! ```rust,ignore
//! use xds_server::connections::{ConnectionTracker, ConnectionLimits};
//!
//! let limits = ConnectionLimits::new(1000, 100);
//! let tracker = ConnectionTracker::new(limits);
//!
//! if let Some(guard) = tracker.try_acquire() {
//!     // Connection accepted
//! } else {
//!     // Connection rejected - at limit
//! }
//! ```

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Instant;

use tracing::{debug, warn};

/// Limits for connection tracking.
#[derive(Debug, Clone)]
pub struct ConnectionLimits {
    /// Maximum total connections.
    pub max_connections: u64,
    /// Maximum connections per IP address.
    pub max_per_ip: u64,
    /// Maximum concurrent streams per connection.
    pub max_streams_per_connection: u32,
}

impl Default for ConnectionLimits {
    fn default() -> Self {
        Self {
            max_connections: 10_000,
            max_per_ip: 100,
            max_streams_per_connection: 100,
        }
    }
}

impl ConnectionLimits {
    /// Create new connection limits.
    pub fn new(max_connections: u64, max_per_ip: u64) -> Self {
        Self {
            max_connections,
            max_per_ip,
            ..Default::default()
        }
    }

    /// Set maximum streams per connection.
    pub fn with_max_streams(mut self, max: u32) -> Self {
        self.max_streams_per_connection = max;
        self
    }
}

/// Information about a tracked connection.
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// Unique connection ID.
    pub id: u64,
    /// Remote address (if available).
    pub remote_addr: Option<SocketAddr>,
    /// When the connection was established.
    pub connected_at: Instant,
    /// Number of active streams.
    pub active_streams: u32,
}

/// Tracker for managing active connections.
///
/// Provides connection counting, per-IP limits, and connection metadata.
#[derive(Debug)]
pub struct ConnectionTracker {
    inner: Arc<ConnectionTrackerInner>,
}

#[derive(Debug)]
struct ConnectionTrackerInner {
    limits: ConnectionLimits,
    /// Counter for generating connection IDs.
    next_id: AtomicU64,
    /// Total active connections.
    active: AtomicU64,
    /// Connections per IP address.
    per_ip: RwLock<HashMap<std::net::IpAddr, u64>>,
    /// Active connection info.
    connections: RwLock<HashMap<u64, ConnectionInfo>>,
}

impl Clone for ConnectionTracker {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl Default for ConnectionTracker {
    fn default() -> Self {
        Self::new(ConnectionLimits::default())
    }
}

impl ConnectionTracker {
    /// Create a new connection tracker with the given limits.
    pub fn new(limits: ConnectionLimits) -> Self {
        Self {
            inner: Arc::new(ConnectionTrackerInner {
                limits,
                next_id: AtomicU64::new(1),
                active: AtomicU64::new(0),
                per_ip: RwLock::new(HashMap::new()),
                connections: RwLock::new(HashMap::new()),
            }),
        }
    }

    /// Get the connection limits.
    pub fn limits(&self) -> &ConnectionLimits {
        &self.inner.limits
    }

    /// Get the current number of active connections.
    pub fn active_connections(&self) -> u64 {
        self.inner.active.load(Ordering::Relaxed)
    }

    /// Get connections for a specific IP.
    pub fn connections_for_ip(&self, ip: std::net::IpAddr) -> u64 {
        self.inner
            .per_ip
            .read()
            .unwrap()
            .get(&ip)
            .copied()
            .unwrap_or(0)
    }

    /// Try to acquire a connection slot.
    ///
    /// Returns `None` if connection limits are exceeded.
    pub fn try_acquire(&self, remote_addr: Option<SocketAddr>) -> Option<ConnectionGuard> {
        // Check total limit
        let current = self.inner.active.load(Ordering::Relaxed);
        if current >= self.inner.limits.max_connections {
            warn!(
                current = current,
                limit = self.inner.limits.max_connections,
                "connection rejected: at max connections"
            );
            return None;
        }

        // Check per-IP limit if address is known
        if let Some(addr) = remote_addr {
            let ip = addr.ip();
            let mut per_ip = self.inner.per_ip.write().unwrap();
            let ip_count = per_ip.get(&ip).copied().unwrap_or(0);

            if ip_count >= self.inner.limits.max_per_ip {
                warn!(
                    ip = %ip,
                    current = ip_count,
                    limit = self.inner.limits.max_per_ip,
                    "connection rejected: at max per-IP limit"
                );
                return None;
            }

            // Increment per-IP counter
            *per_ip.entry(ip).or_insert(0) += 1;
        }

        // Increment total counter
        self.inner.active.fetch_add(1, Ordering::Relaxed);

        // Generate connection ID and store info
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let info = ConnectionInfo {
            id,
            remote_addr,
            connected_at: Instant::now(),
            active_streams: 0,
        };

        self.inner.connections.write().unwrap().insert(id, info);

        debug!(
            id = id,
            remote_addr = ?remote_addr,
            active = self.active_connections(),
            "connection accepted"
        );

        Some(ConnectionGuard {
            tracker: self.clone(),
            id,
            remote_addr,
        })
    }

    /// Get information about all active connections.
    pub fn list_connections(&self) -> Vec<ConnectionInfo> {
        self.inner
            .connections
            .read()
            .unwrap()
            .values()
            .cloned()
            .collect()
    }

    /// Get information about a specific connection.
    pub fn get_connection(&self, id: u64) -> Option<ConnectionInfo> {
        self.inner.connections.read().unwrap().get(&id).cloned()
    }

    fn release(&self, id: u64, remote_addr: Option<SocketAddr>) {
        // Decrement total counter
        self.inner.active.fetch_sub(1, Ordering::Relaxed);

        // Decrement per-IP counter
        if let Some(addr) = remote_addr {
            let ip = addr.ip();
            let mut per_ip = self.inner.per_ip.write().unwrap();
            if let Some(count) = per_ip.get_mut(&ip) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    per_ip.remove(&ip);
                }
            }
        }

        // Remove connection info
        self.inner.connections.write().unwrap().remove(&id);

        debug!(
            id = id,
            remote_addr = ?remote_addr,
            active = self.active_connections(),
            "connection released"
        );
    }
}

/// Guard for a tracked connection.
///
/// Automatically releases the connection slot when dropped.
#[derive(Debug)]
pub struct ConnectionGuard {
    tracker: ConnectionTracker,
    id: u64,
    remote_addr: Option<SocketAddr>,
}

impl ConnectionGuard {
    /// Get the connection ID.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get the remote address.
    pub fn remote_addr(&self) -> Option<SocketAddr> {
        self.remote_addr
    }

    /// Increment the stream count for this connection.
    pub fn add_stream(&self) -> bool {
        let mut connections = self.tracker.inner.connections.write().unwrap();
        if let Some(info) = connections.get_mut(&self.id) {
            if info.active_streams >= self.tracker.inner.limits.max_streams_per_connection {
                return false;
            }
            info.active_streams += 1;
            true
        } else {
            false
        }
    }

    /// Decrement the stream count for this connection.
    pub fn remove_stream(&self) {
        let mut connections = self.tracker.inner.connections.write().unwrap();
        if let Some(info) = connections.get_mut(&self.id) {
            info.active_streams = info.active_streams.saturating_sub(1);
        }
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.tracker.release(self.id, self.remote_addr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn connection_limits_default() {
        let limits = ConnectionLimits::default();
        assert_eq!(limits.max_connections, 10_000);
        assert_eq!(limits.max_per_ip, 100);
    }

    #[test]
    fn tracker_basic() {
        let tracker = ConnectionTracker::new(ConnectionLimits::new(10, 5));
        assert_eq!(tracker.active_connections(), 0);

        let guard = tracker.try_acquire(None).unwrap();
        assert_eq!(tracker.active_connections(), 1);
        assert!(guard.id() > 0);

        drop(guard);
        assert_eq!(tracker.active_connections(), 0);
    }

    #[test]
    fn tracker_max_connections() {
        let tracker = ConnectionTracker::new(ConnectionLimits::new(2, 10));

        let _g1 = tracker.try_acquire(None).unwrap();
        let _g2 = tracker.try_acquire(None).unwrap();

        // Should be rejected
        assert!(tracker.try_acquire(None).is_none());
    }

    #[test]
    fn tracker_per_ip_limit() {
        let tracker = ConnectionTracker::new(ConnectionLimits::new(100, 2));

        let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 8080);
        let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)), 8080);

        let _g1 = tracker.try_acquire(Some(addr1)).unwrap();
        let _g2 = tracker.try_acquire(Some(addr1)).unwrap();

        // Third connection from same IP should be rejected
        assert!(tracker.try_acquire(Some(addr1)).is_none());

        // But different IP should work
        let _g3 = tracker.try_acquire(Some(addr2)).unwrap();
    }

    #[test]
    fn tracker_per_ip_release() {
        let tracker = ConnectionTracker::new(ConnectionLimits::new(100, 2));

        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 8080);

        {
            let _g1 = tracker.try_acquire(Some(addr)).unwrap();
            let _g2 = tracker.try_acquire(Some(addr)).unwrap();
            assert!(tracker.try_acquire(Some(addr)).is_none());
        }

        // After guards dropped, should be able to acquire again
        let _g = tracker.try_acquire(Some(addr)).unwrap();
        assert_eq!(tracker.connections_for_ip(addr.ip()), 1);
    }

    #[test]
    fn tracker_stream_counting() {
        let tracker = ConnectionTracker::new(ConnectionLimits::new(10, 10).with_max_streams(2));

        let guard = tracker.try_acquire(None).unwrap();

        assert!(guard.add_stream());
        assert!(guard.add_stream());
        assert!(!guard.add_stream()); // Should fail - at limit

        guard.remove_stream();
        assert!(guard.add_stream()); // Should work now
    }

    #[test]
    fn tracker_list_connections() {
        let tracker = ConnectionTracker::new(ConnectionLimits::default());

        let _g1 = tracker.try_acquire(None).unwrap();
        let _g2 = tracker.try_acquire(None).unwrap();

        let connections = tracker.list_connections();
        assert_eq!(connections.len(), 2);
    }
}
