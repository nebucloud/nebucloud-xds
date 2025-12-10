//! Test harness for Envoy integration tests.
//!
//! Provides utilities for:
//! - Starting/stopping xDS test server
//! - Creating sample xDS resources
//! - Checking Envoy admin API
//! - Waiting for Envoy to sync

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::oneshot;
use tracing::{info, warn};
use xds_cache::{Cache, ShardedCache, Snapshot};
use xds_core::{NodeHash, TypeUrl};
use xds_server::XdsServer;

/// Default xDS server address.
pub const XDS_SERVER_ADDR: &str = "0.0.0.0:18000";

/// Envoy admin API address (inside Docker).
pub const ENVOY_ADMIN_URL: &str = "http://localhost:9901";

/// Test harness for running xDS integration tests.
pub struct TestHarness {
    /// The xDS server instance.
    server: Option<XdsServer>,
    /// Shared cache.
    cache: Arc<ShardedCache>,
    /// Shutdown signal sender.
    shutdown_tx: Option<oneshot::Sender<()>>,
    /// Server join handle.
    server_handle: Option<tokio::task::JoinHandle<Result<(), tonic::transport::Error>>>,
}

impl TestHarness {
    /// Create a new test harness.
    pub fn new() -> Self {
        let cache = Arc::new(ShardedCache::new());

        Self {
            server: None,
            cache,
            shutdown_tx: None,
            server_handle: None,
        }
    }

    /// Get the cache for setting snapshots.
    pub fn cache(&self) -> &Arc<ShardedCache> {
        &self.cache
    }

    /// Start the xDS server.
    pub async fn start_server(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let server = XdsServer::builder()
            .cache(Arc::clone(&self.cache))
            .enable_sotw()
            .enable_delta()
            .enable_health_check()
            .build()?;

        let addr: SocketAddr = XDS_SERVER_ADDR.parse()?;
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        // Clone the server for moving into the task
        let server_for_task = XdsServer::builder()
            .cache(Arc::clone(&self.cache))
            .enable_sotw()
            .enable_delta()
            .enable_health_check()
            .build()?;

        let handle = tokio::spawn(async move {
            server_for_task.serve_with_shutdown(addr, shutdown_rx).await
        });

        self.server = Some(server);
        self.shutdown_tx = Some(shutdown_tx);
        self.server_handle = Some(handle);

        // Wait a bit for server to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        info!("xDS test server started on {}", XDS_SERVER_ADDR);
        Ok(())
    }

    /// Stop the xDS server.
    pub async fn stop_server(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        if let Some(handle) = self.server_handle.take() {
            let _ = tokio::time::timeout(Duration::from_secs(5), handle).await;
        }

        info!("xDS test server stopped");
    }

    /// Set a snapshot for the test node.
    pub fn set_snapshot(&self, snapshot: Snapshot) {
        let node = NodeHash::from_id("test-envoy-node");
        self.cache.set_snapshot(node, snapshot);
        info!("Set snapshot for test-envoy-node");
    }

    /// Create a snapshot with a basic cluster.
    pub fn create_cluster_snapshot(name: &str, version: &str) -> Snapshot {
        // Create a minimal cluster resource
        let cluster = TestCluster {
            name: name.to_string(),
            connect_timeout_ms: 5000,
        };

        Snapshot::builder()
            .version(version)
            .resource(TypeUrl::new(TypeUrl::CLUSTER), Arc::new(cluster))
            .build()
    }

    /// Create a snapshot with a basic listener.
    pub fn create_listener_snapshot(name: &str, port: u32, version: &str) -> Snapshot {
        let listener = TestListener {
            name: name.to_string(),
            port,
        };

        Snapshot::builder()
            .version(version)
            .resource(TypeUrl::new(TypeUrl::LISTENER), Arc::new(listener))
            .build()
    }

    /// Wait for Envoy to be ready.
    pub async fn wait_for_envoy(&self, timeout: Duration) -> Result<(), String> {
        let start = std::time::Instant::now();
        let client = reqwest::Client::new();

        while start.elapsed() < timeout {
            match client.get(format!("{}/ready", ENVOY_ADMIN_URL)).send().await {
                Ok(resp) if resp.status().is_success() => {
                    info!("Envoy is ready");
                    return Ok(());
                }
                Ok(resp) => {
                    warn!("Envoy not ready: status {}", resp.status());
                }
                Err(e) => {
                    warn!("Cannot connect to Envoy: {}", e);
                }
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        Err("Envoy did not become ready in time".to_string())
    }

    /// Check Envoy cluster status via admin API.
    pub async fn get_envoy_clusters(&self) -> Result<serde_json::Value, reqwest::Error> {
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{}/clusters?format=json", ENVOY_ADMIN_URL))
            .send()
            .await?
            .json()
            .await?;
        Ok(resp)
    }

    /// Check Envoy listeners via admin API.
    pub async fn get_envoy_listeners(&self) -> Result<serde_json::Value, reqwest::Error> {
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{}/listeners?format=json", ENVOY_ADMIN_URL))
            .send()
            .await?
            .json()
            .await?;
        Ok(resp)
    }

    /// Check Envoy config dump via admin API.
    pub async fn get_envoy_config_dump(&self) -> Result<serde_json::Value, reqwest::Error> {
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{}/config_dump", ENVOY_ADMIN_URL))
            .send()
            .await?
            .json()
            .await?;
        Ok(resp)
    }

    /// Wait for a specific number of clusters to be configured.
    pub async fn wait_for_clusters(
        &self,
        expected_count: usize,
        timeout: Duration,
    ) -> Result<(), String> {
        let start = std::time::Instant::now();

        while start.elapsed() < timeout {
            if let Ok(clusters) = self.get_envoy_clusters().await {
                if let Some(cluster_statuses) = clusters.get("cluster_statuses") {
                    if let Some(arr) = cluster_statuses.as_array() {
                        // Filter out static clusters (xds_cluster)
                        let dynamic_count = arr
                            .iter()
                            .filter(|c| {
                                c.get("name")
                                    .and_then(|n| n.as_str())
                                    .map(|n| n != "xds_cluster")
                                    .unwrap_or(false)
                            })
                            .count();

                        if dynamic_count >= expected_count {
                            info!("Found {} dynamic clusters", dynamic_count);
                            return Ok(());
                        }
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        Err(format!(
            "Did not find {} clusters in time",
            expected_count
        ))
    }
}

impl Default for TestHarness {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TestHarness {
    fn drop(&mut self) {
        // Send shutdown signal if server is still running
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// A minimal test cluster resource.
#[derive(Debug, Clone)]
pub struct TestCluster {
    pub name: String,
    pub connect_timeout_ms: u64,
}

impl xds_core::Resource for TestCluster {
    fn type_url(&self) -> &str {
        TypeUrl::CLUSTER
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn encode(&self) -> Result<prost_types::Any, Box<dyn std::error::Error + Send + Sync>> {
        // For real integration, we'd encode to envoy.config.cluster.v3.Cluster
        // For now, create a minimal Any
        Ok(prost_types::Any {
            type_url: TypeUrl::CLUSTER.to_string(),
            value: Vec::new(), // Would be protobuf-encoded cluster
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// A minimal test listener resource.
#[derive(Debug, Clone)]
pub struct TestListener {
    pub name: String,
    pub port: u32,
}

impl xds_core::Resource for TestListener {
    fn type_url(&self) -> &str {
        TypeUrl::LISTENER
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn encode(&self) -> Result<prost_types::Any, Box<dyn std::error::Error + Send + Sync>> {
        Ok(prost_types::Any {
            type_url: TypeUrl::LISTENER.to_string(),
            value: Vec::new(),
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
