//! Stream context and identification.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use xds_core::NodeHash;

/// Unique identifier for a stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StreamId(u64);

impl StreamId {
    /// Generate a new unique stream ID.
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the numeric value.
    #[inline]
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Default for StreamId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for StreamId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "stream-{}", self.0)
    }
}

/// Context for an active xDS stream.
///
/// Tracks metadata about a client connection including:
/// - Stream identifier
/// - Node information
/// - Timing information
/// - Request/response counts
#[derive(Debug)]
pub struct StreamContext {
    /// Unique stream identifier.
    id: StreamId,
    /// Node hash for this stream's client.
    node_hash: Option<NodeHash>,
    /// Node ID as a string.
    node_id: Option<String>,
    /// When the stream was created.
    created_at: Instant,
    /// Number of requests received.
    requests: AtomicU64,
    /// Number of responses sent.
    responses: AtomicU64,
    /// Last request timestamp.
    last_request: std::sync::Mutex<Instant>,
}

impl StreamContext {
    /// Create a new stream context.
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            id: StreamId::new(),
            node_hash: None,
            node_id: None,
            created_at: now,
            requests: AtomicU64::new(0),
            responses: AtomicU64::new(0),
            last_request: std::sync::Mutex::new(now),
        }
    }

    /// Get the stream ID.
    #[inline]
    pub fn id(&self) -> StreamId {
        self.id
    }

    /// Get the node hash if set.
    #[inline]
    pub fn node_hash(&self) -> Option<NodeHash> {
        self.node_hash
    }

    /// Get the node ID if set.
    #[inline]
    pub fn node_id(&self) -> Option<&str> {
        self.node_id.as_deref()
    }

    /// Set the node information.
    pub fn set_node(&mut self, node_id: String, node_hash: NodeHash) {
        self.node_id = Some(node_id);
        self.node_hash = Some(node_hash);
    }

    /// Get when this stream was created.
    #[inline]
    pub fn created_at(&self) -> Instant {
        self.created_at
    }

    /// Get stream duration.
    #[inline]
    pub fn duration(&self) -> std::time::Duration {
        self.created_at.elapsed()
    }

    /// Record a request.
    pub fn record_request(&self) {
        self.requests.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut last) = self.last_request.lock() {
            *last = Instant::now();
        }
    }

    /// Record a response.
    pub fn record_response(&self) {
        self.responses.fetch_add(1, Ordering::Relaxed);
    }

    /// Get total requests.
    #[inline]
    pub fn request_count(&self) -> u64 {
        self.requests.load(Ordering::Relaxed)
    }

    /// Get total responses.
    #[inline]
    pub fn response_count(&self) -> u64 {
        self.responses.load(Ordering::Relaxed)
    }

    /// Get time since last request.
    pub fn idle_time(&self) -> std::time::Duration {
        self.last_request
            .lock()
            .map(|t| t.elapsed())
            .unwrap_or_default()
    }
}

impl Default for StreamContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_id_unique() {
        let id1 = StreamId::new();
        let id2 = StreamId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn stream_context_basic() {
        let ctx = StreamContext::new();
        assert_eq!(ctx.request_count(), 0);
        assert_eq!(ctx.response_count(), 0);
        assert!(ctx.node_hash().is_none());
    }

    #[test]
    fn stream_context_counting() {
        let ctx = StreamContext::new();
        ctx.record_request();
        ctx.record_request();
        ctx.record_response();

        assert_eq!(ctx.request_count(), 2);
        assert_eq!(ctx.response_count(), 1);
    }

    #[test]
    fn stream_context_node() {
        let mut ctx = StreamContext::new();
        let hash = NodeHash::from_id("test-node");
        ctx.set_node("test-node".to_string(), hash);

        assert_eq!(ctx.node_id(), Some("test-node"));
        assert_eq!(ctx.node_hash(), Some(hash));
    }
}
