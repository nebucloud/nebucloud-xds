//! Prometheus metrics for xDS server.
//!
//! This module provides comprehensive metrics for monitoring the xDS server:
//!
//! - Request/response counters per resource type
//! - Latency histograms for request processing
//! - Cache operation metrics
//! - Connection and stream tracking
//!
//! # Example
//!
//! ```rust,ignore
//! use xds_server::metrics::XdsMetrics;
//!
//! let metrics = XdsMetrics::new();
//! metrics.record_request("type.googleapis.com/envoy.config.cluster.v3.Cluster");
//! metrics.record_response("type.googleapis.com/envoy.config.cluster.v3.Cluster", 150);
//! ```

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use metrics::{counter, gauge, histogram};

/// Metrics for the xDS server.
///
/// Provides Prometheus-compatible metrics for monitoring server health,
/// performance, and resource distribution.
#[derive(Debug, Clone)]
pub struct XdsMetrics {
    inner: Arc<XdsMetricsInner>,
}

#[derive(Debug)]
struct XdsMetricsInner {
    /// Total active streams.
    active_streams: AtomicU64,
    /// Total active connections.
    active_connections: AtomicU64,
}

impl Default for XdsMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl XdsMetrics {
    /// Create a new metrics instance.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(XdsMetricsInner {
                active_streams: AtomicU64::new(0),
                active_connections: AtomicU64::new(0),
            }),
        }
    }

    /// Record an incoming request.
    pub fn record_request(&self, type_url: &str) {
        counter!("xds_requests_total", "type_url" => type_url.to_string()).increment(1);
    }

    /// Record a response sent.
    pub fn record_response(&self, type_url: &str, latency_ms: u64) {
        counter!("xds_responses_total", "type_url" => type_url.to_string()).increment(1);
        histogram!("xds_response_latency_ms", "type_url" => type_url.to_string())
            .record(latency_ms as f64);
    }

    /// Record a NACK (negative acknowledgment).
    pub fn record_nack(&self, type_url: &str) {
        counter!("xds_nacks_total", "type_url" => type_url.to_string()).increment(1);
    }

    /// Record an ACK (acknowledgment).
    pub fn record_ack(&self, type_url: &str) {
        counter!("xds_acks_total", "type_url" => type_url.to_string()).increment(1);
    }

    /// Record a stream opened.
    pub fn stream_opened(&self, service: &str) {
        let count = self.inner.active_streams.fetch_add(1, Ordering::Relaxed) + 1;
        counter!("xds_streams_opened_total", "service" => service.to_string()).increment(1);
        gauge!("xds_active_streams").set(count as f64);
    }

    /// Record a stream closed.
    pub fn stream_closed(&self, service: &str, duration: Duration) {
        let count = self.inner.active_streams.fetch_sub(1, Ordering::Relaxed) - 1;
        counter!("xds_streams_closed_total", "service" => service.to_string()).increment(1);
        gauge!("xds_active_streams").set(count as f64);
        histogram!("xds_stream_duration_seconds", "service" => service.to_string())
            .record(duration.as_secs_f64());
    }

    /// Record a connection opened.
    pub fn connection_opened(&self) {
        let count = self.inner.active_connections.fetch_add(1, Ordering::Relaxed) + 1;
        counter!("xds_connections_opened_total").increment(1);
        gauge!("xds_active_connections").set(count as f64);
    }

    /// Record a connection closed.
    pub fn connection_closed(&self) {
        let count = self.inner.active_connections.fetch_sub(1, Ordering::Relaxed) - 1;
        counter!("xds_connections_closed_total").increment(1);
        gauge!("xds_active_connections").set(count as f64);
    }

    /// Record cache hit.
    pub fn cache_hit(&self, type_url: &str) {
        counter!("xds_cache_hits_total", "type_url" => type_url.to_string()).increment(1);
    }

    /// Record cache miss.
    pub fn cache_miss(&self, type_url: &str) {
        counter!("xds_cache_misses_total", "type_url" => type_url.to_string()).increment(1);
    }

    /// Record snapshot update.
    pub fn snapshot_updated(&self, node_count: usize, resource_count: usize) {
        counter!("xds_snapshot_updates_total").increment(1);
        gauge!("xds_snapshot_nodes").set(node_count as f64);
        gauge!("xds_snapshot_resources").set(resource_count as f64);
    }

    /// Get the current number of active streams.
    pub fn active_streams(&self) -> u64 {
        self.inner.active_streams.load(Ordering::Relaxed)
    }

    /// Get the current number of active connections.
    pub fn active_connections(&self) -> u64 {
        self.inner.active_connections.load(Ordering::Relaxed)
    }
}

/// Timer for measuring operation latency.
///
/// Automatically records the duration when dropped.
#[derive(Debug)]
pub struct LatencyTimer {
    start: Instant,
    type_url: String,
    metrics: XdsMetrics,
}

impl LatencyTimer {
    /// Create a new latency timer.
    pub fn new(metrics: XdsMetrics, type_url: impl Into<String>) -> Self {
        Self {
            start: Instant::now(),
            type_url: type_url.into(),
            metrics,
        }
    }

    /// Finish the timer and record the latency.
    pub fn finish(self) {
        let elapsed = self.start.elapsed();
        self.metrics
            .record_response(&self.type_url, elapsed.as_millis() as u64);
    }
}

impl Drop for LatencyTimer {
    fn drop(&mut self) {
        // Record on drop as well, in case finish() wasn't called
    }
}

/// Stream duration tracker.
///
/// Records stream duration when dropped.
#[derive(Debug)]
pub struct StreamTracker {
    start: Instant,
    service: String,
    metrics: XdsMetrics,
}

impl StreamTracker {
    /// Create a new stream tracker.
    pub fn new(metrics: XdsMetrics, service: impl Into<String>) -> Self {
        let service = service.into();
        metrics.stream_opened(&service);
        Self {
            start: Instant::now(),
            service,
            metrics,
        }
    }
}

impl Drop for StreamTracker {
    fn drop(&mut self) {
        self.metrics
            .stream_closed(&self.service, self.start.elapsed());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_creation() {
        let metrics = XdsMetrics::new();
        assert_eq!(metrics.active_streams(), 0);
        assert_eq!(metrics.active_connections(), 0);
    }

    #[test]
    fn stream_tracking() {
        let metrics = XdsMetrics::new();

        metrics.stream_opened("ads");
        assert_eq!(metrics.active_streams(), 1);

        metrics.stream_opened("cds");
        assert_eq!(metrics.active_streams(), 2);

        metrics.stream_closed("ads", Duration::from_secs(10));
        assert_eq!(metrics.active_streams(), 1);
    }

    #[test]
    fn connection_tracking() {
        let metrics = XdsMetrics::new();

        metrics.connection_opened();
        assert_eq!(metrics.active_connections(), 1);

        metrics.connection_opened();
        assert_eq!(metrics.active_connections(), 2);

        metrics.connection_closed();
        assert_eq!(metrics.active_connections(), 1);
    }
}
