//! Node identification and hashing for xDS.
//!
//! This module provides [`NodeHash`], an efficient node identifier using
//! FNV-1a hashing. This is used to identify Envoy nodes in the cache
//! for per-node configuration snapshots.

use std::fmt;
use std::hash::{Hash, Hasher};

use fnv::FnvHasher;

/// Hash-based node identifier for efficient lookup.
///
/// `NodeHash` uses FNV-1a hashing to convert node IDs into fixed-size
/// hash values for efficient cache lookups. It also supports a wildcard
/// value for broadcast scenarios.
///
/// # Example
///
/// ```rust
/// use xds_core::NodeHash;
///
/// let node1 = NodeHash::from_id("envoy-node-1");
/// let node2 = NodeHash::from_id("envoy-node-2");
/// let wildcard = NodeHash::wildcard();
///
/// assert_ne!(node1, node2);
/// assert!(wildcard.is_wildcard());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeHash {
    hash: u64,
    is_wildcard: bool,
}

impl NodeHash {
    /// The wildcard hash value (all zeros).
    const WILDCARD_HASH: u64 = 0;

    /// Create a node hash from a node ID string.
    ///
    /// Uses FNV-1a hashing for fast, well-distributed hashes.
    #[must_use]
    pub fn from_id(node_id: &str) -> Self {
        let mut hasher = FnvHasher::default();
        node_id.hash(&mut hasher);
        let hash = hasher.finish();

        // Ensure we don't accidentally create a wildcard
        let hash = if hash == Self::WILDCARD_HASH {
            hash.wrapping_add(1)
        } else {
            hash
        };

        Self {
            hash,
            is_wildcard: false,
        }
    }

    /// Create a wildcard node hash that matches all nodes.
    ///
    /// This is used when you want to set a snapshot that applies
    /// to all nodes that don't have a specific snapshot.
    #[must_use]
    pub fn wildcard() -> Self {
        Self {
            hash: Self::WILDCARD_HASH,
            is_wildcard: true,
        }
    }

    /// Check if this is a wildcard hash.
    #[must_use]
    pub fn is_wildcard(&self) -> bool {
        self.is_wildcard
    }

    /// Get the raw hash value.
    #[must_use]
    pub fn as_u64(&self) -> u64 {
        self.hash
    }
}

impl fmt::Display for NodeHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_wildcard {
            write!(f, "<wildcard>")
        } else {
            write!(f, "{:016x}", self.hash)
        }
    }
}

impl Default for NodeHash {
    fn default() -> Self {
        Self::wildcard()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_hash_creation() {
        let node = NodeHash::from_id("test-node");
        assert!(!node.is_wildcard());
        assert_ne!(node.as_u64(), 0);
    }

    #[test]
    fn test_node_hash_deterministic() {
        let node1 = NodeHash::from_id("test-node");
        let node2 = NodeHash::from_id("test-node");
        assert_eq!(node1, node2);
        assert_eq!(node1.as_u64(), node2.as_u64());
    }

    #[test]
    fn test_different_nodes_different_hashes() {
        let node1 = NodeHash::from_id("node-1");
        let node2 = NodeHash::from_id("node-2");
        assert_ne!(node1, node2);
    }

    #[test]
    fn test_wildcard() {
        let wildcard = NodeHash::wildcard();
        assert!(wildcard.is_wildcard());
        assert_eq!(wildcard.as_u64(), 0);
    }

    #[test]
    fn test_display() {
        let node = NodeHash::from_id("test");
        let display = format!("{node}");
        assert_eq!(display.len(), 16); // 16 hex chars

        let wildcard = NodeHash::wildcard();
        assert_eq!(format!("{wildcard}"), "<wildcard>");
    }

    #[test]
    fn test_default_is_wildcard() {
        let default = NodeHash::default();
        assert!(default.is_wildcard());
    }
}
