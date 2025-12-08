//! Watch system for cache update notifications.
//!
//! The watch system provides:
//! - Unique watch identifiers ([`WatchId`])
//! - Watch subscriptions ([`Watch`]) for receiving updates
//! - Watch management ([`WatchManager`]) for handling multiple subscriptions

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::sync::mpsc;
use tracing::{debug, trace, warn};
use xds_core::{NodeHash, XdsError, XdsResult};

use crate::Snapshot;

/// Unique identifier for a watch subscription.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WatchId(u64);

impl WatchId {
    /// Create a new unique watch ID.
    fn next() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the numeric value of this watch ID.
    #[inline]
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for WatchId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "watch-{}", self.0)
    }
}

/// A watch subscription for receiving snapshot updates.
///
/// When a snapshot is updated for a node, all active watches for that node
/// receive the new snapshot through their channel.
#[derive(Debug)]
pub struct Watch {
    /// Unique identifier for this watch.
    id: WatchId,
    /// Node this watch is subscribed to.
    node_hash: NodeHash,
    /// Receiver for snapshot updates.
    receiver: mpsc::Receiver<Arc<Snapshot>>,
}

impl Watch {
    /// Get the unique identifier for this watch.
    #[inline]
    pub fn id(&self) -> WatchId {
        self.id
    }

    /// Get the node hash this watch is subscribed to.
    #[inline]
    pub fn node_hash(&self) -> NodeHash {
        self.node_hash
    }

    /// Receive the next snapshot update.
    ///
    /// Returns `None` if the watch has been cancelled.
    pub async fn recv(&mut self) -> Option<Arc<Snapshot>> {
        self.receiver.recv().await
    }

    /// Try to receive a snapshot update without waiting.
    ///
    /// Returns:
    /// - `Ok(snapshot)` if an update is available
    /// - `Err(TryRecvError::Empty)` if no update is available
    /// - `Err(TryRecvError::Disconnected)` if the watch has been cancelled
    pub fn try_recv(&mut self) -> Result<Arc<Snapshot>, mpsc::error::TryRecvError> {
        self.receiver.try_recv()
    }
}

/// Sender half of a watch, used internally to send updates.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Used for debugging and future features
pub(crate) struct WatchSender {
    id: WatchId,
    node_hash: NodeHash,
    sender: mpsc::Sender<Arc<Snapshot>>,
}

#[allow(dead_code)] // Methods used for debugging and future features
impl WatchSender {
    /// Try to send a snapshot update.
    ///
    /// Uses `try_send` to avoid blocking. If the channel is full,
    /// the update is dropped (the receiver will get the next one).
    pub fn try_send(&self, snapshot: Arc<Snapshot>) -> XdsResult<()> {
        match self.sender.try_send(snapshot) {
            Ok(()) => Ok(()),
            Err(mpsc::error::TrySendError::Full(_)) => {
                // Channel full, skip this update
                trace!(watch_id = %self.id, "watch channel full, skipping update");
                Ok(())
            }
            Err(mpsc::error::TrySendError::Closed(_)) => Err(XdsError::WatchClosed {
                watch_id: self.id.0,
            }),
        }
    }

    /// Get the watch ID.
    #[inline]
    pub fn id(&self) -> WatchId {
        self.id
    }
}

/// Manager for watch subscriptions.
///
/// Handles creating, tracking, and cancelling watches.
/// Uses a `Mutex` internally but operations are fast (no I/O).
#[derive(Debug)]
pub struct WatchManager {
    /// Map of node hash to active watch senders.
    watches: std::sync::Mutex<HashMap<NodeHash, Vec<WatchSender>>>,
    /// Channel buffer size for new watches.
    channel_buffer: usize,
}

impl Default for WatchManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WatchManager {
    /// Create a new watch manager with default settings.
    pub fn new() -> Self {
        Self::with_buffer_size(16)
    }

    /// Create a new watch manager with a custom channel buffer size.
    pub fn with_buffer_size(buffer_size: usize) -> Self {
        Self {
            watches: std::sync::Mutex::new(HashMap::new()),
            channel_buffer: buffer_size,
        }
    }

    /// Create a new watch for a node.
    ///
    /// Returns a `Watch` that will receive snapshot updates for the specified node.
    pub fn create_watch(&self, node_hash: NodeHash) -> Watch {
        let id = WatchId::next();
        let (sender, receiver) = mpsc::channel(self.channel_buffer);

        let watch_sender = WatchSender {
            id,
            node_hash,
            sender,
        };

        // Lock is held briefly, no I/O
        {
            let mut watches = self.watches.lock().expect("watch lock poisoned");
            watches.entry(node_hash).or_default().push(watch_sender);
        }

        debug!(watch_id = %id, node = %node_hash, "created watch");

        Watch {
            id,
            node_hash,
            receiver,
        }
    }

    /// Cancel a watch subscription.
    ///
    /// The watch will no longer receive updates.
    pub fn cancel_watch(&self, watch_id: WatchId) {
        let mut watches = self.watches.lock().expect("watch lock poisoned");

        // Find and remove the watch
        for senders in watches.values_mut() {
            if let Some(pos) = senders.iter().position(|s| s.id == watch_id) {
                senders.swap_remove(pos);
                debug!(watch_id = %watch_id, "cancelled watch");
                return;
            }
        }

        warn!(watch_id = %watch_id, "attempted to cancel unknown watch");
    }

    /// Notify all watches for a node about a snapshot update.
    ///
    /// Removes any closed watches automatically.
    pub fn notify(&self, node_hash: NodeHash, snapshot: Arc<Snapshot>) {
        // Clone senders while holding lock briefly
        let senders: Vec<WatchSender> = {
            let watches = self.watches.lock().expect("watch lock poisoned");
            watches.get(&node_hash).cloned().unwrap_or_default()
        };

        if senders.is_empty() {
            return;
        }

        // Track which watches failed (closed)
        let mut closed_ids = Vec::new();

        for sender in &senders {
            if let Err(XdsError::WatchClosed { watch_id }) = sender.try_send(Arc::clone(&snapshot))
            {
                closed_ids.push(WatchId(watch_id));
            }
        }

        // Remove closed watches
        if !closed_ids.is_empty() {
            let mut watches = self.watches.lock().expect("watch lock poisoned");
            if let Some(senders) = watches.get_mut(&node_hash) {
                senders.retain(|s| !closed_ids.contains(&s.id));
            }
            debug!(count = closed_ids.len(), "removed closed watches");
        }

        trace!(
            node = %node_hash,
            watch_count = senders.len() - closed_ids.len(),
            "notified watches of snapshot update"
        );
    }

    /// Get the number of active watches for a node.
    pub fn watch_count(&self, node_hash: NodeHash) -> usize {
        let watches = self.watches.lock().expect("watch lock poisoned");
        watches.get(&node_hash).map(|v| v.len()).unwrap_or(0)
    }

    /// Get the total number of active watches across all nodes.
    pub fn total_watch_count(&self) -> usize {
        let watches = self.watches.lock().expect("watch lock poisoned");
        watches.values().map(|v| v.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watch_id_unique() {
        let id1 = WatchId::next();
        let id2 = WatchId::next();
        assert_ne!(id1, id2);
    }

    #[tokio::test]
    async fn watch_manager_create_and_notify() {
        let manager = WatchManager::new();
        let node = NodeHash::from_id("test-node");

        let mut watch = manager.create_watch(node);
        assert_eq!(manager.watch_count(node), 1);

        let snapshot = Arc::new(Snapshot::builder().version("v1").build());
        manager.notify(node, snapshot.clone());

        let received = watch.recv().await.unwrap();
        assert_eq!(received.version(), "v1");
    }

    #[test]
    fn watch_manager_cancel() {
        let manager = WatchManager::new();
        let node = NodeHash::from_id("test-node");

        let watch = manager.create_watch(node);
        assert_eq!(manager.watch_count(node), 1);

        manager.cancel_watch(watch.id());
        assert_eq!(manager.watch_count(node), 0);
    }
}
