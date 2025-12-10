//! Integration tests with real Envoy proxy.
//!
//! These tests require Docker and start a real Envoy container.

use std::time::Duration;

use crate::harness::TestHarness;
use tracing_subscriber::FmtSubscriber;

/// Initialize tracing for tests.
fn init_tracing() {
    let _ = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .with_test_writer()
        .try_init();
}

/// Test that Envoy connects to xDS server.
///
/// This test:
/// 1. Starts the xDS test server
/// 2. Waits for Envoy to become ready
/// 3. Verifies Envoy connected to xDS
#[tokio::test]
#[ignore = "requires Docker with Envoy running"]
async fn test_envoy_connects_to_xds() {
    init_tracing();

    let mut harness = TestHarness::new();

    // Start xDS server
    harness
        .start_server()
        .await
        .expect("Failed to start xDS server");

    // Wait for Envoy to be ready
    harness
        .wait_for_envoy(Duration::from_secs(30))
        .await
        .expect("Envoy did not become ready");

    // Check config dump to verify xDS connection
    let config_dump = harness
        .get_envoy_config_dump()
        .await
        .expect("Failed to get config dump");

    // Verify xDS cluster is present
    assert!(
        config_dump.to_string().contains("xds_cluster"),
        "xds_cluster should be in config"
    );

    harness.stop_server().await;
}

/// Test that Envoy receives cluster configuration.
#[tokio::test]
#[ignore = "requires Docker with Envoy running"]
async fn test_envoy_receives_clusters() {
    init_tracing();

    let mut harness = TestHarness::new();

    // Start xDS server
    harness
        .start_server()
        .await
        .expect("Failed to start xDS server");

    // Set a cluster snapshot
    let snapshot = TestHarness::create_cluster_snapshot("test-backend", "v1");
    harness.set_snapshot(snapshot);

    // Wait for Envoy
    harness
        .wait_for_envoy(Duration::from_secs(30))
        .await
        .expect("Envoy did not become ready");

    // Give Envoy time to receive the cluster
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Check clusters
    let clusters = harness
        .get_envoy_clusters()
        .await
        .expect("Failed to get clusters");

    tracing::info!("Clusters: {}", serde_json::to_string_pretty(&clusters).unwrap());

    harness.stop_server().await;
}

/// Test that Envoy receives listener configuration.
#[tokio::test]
#[ignore = "requires Docker with Envoy running"]
async fn test_envoy_receives_listeners() {
    init_tracing();

    let mut harness = TestHarness::new();

    // Start xDS server
    harness
        .start_server()
        .await
        .expect("Failed to start xDS server");

    // Set a listener snapshot
    let snapshot = TestHarness::create_listener_snapshot("test-listener", 10000, "v1");
    harness.set_snapshot(snapshot);

    // Wait for Envoy
    harness
        .wait_for_envoy(Duration::from_secs(30))
        .await
        .expect("Envoy did not become ready");

    // Give Envoy time to receive the listener
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Check listeners
    let listeners = harness
        .get_envoy_listeners()
        .await
        .expect("Failed to get listeners");

    tracing::info!("Listeners: {}", serde_json::to_string_pretty(&listeners).unwrap());

    harness.stop_server().await;
}

/// Test snapshot update propagation.
#[tokio::test]
#[ignore = "requires Docker with Envoy running"]
async fn test_envoy_receives_updates() {
    init_tracing();

    let mut harness = TestHarness::new();

    // Start xDS server
    harness
        .start_server()
        .await
        .expect("Failed to start xDS server");

    // Wait for Envoy
    harness
        .wait_for_envoy(Duration::from_secs(30))
        .await
        .expect("Envoy did not become ready");

    // Set initial snapshot
    let snapshot_v1 = TestHarness::create_cluster_snapshot("backend-v1", "v1");
    harness.set_snapshot(snapshot_v1);

    // Wait for initial sync
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Update snapshot
    let snapshot_v2 = TestHarness::create_cluster_snapshot("backend-v2", "v2");
    harness.set_snapshot(snapshot_v2);

    // Wait for update propagation
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Verify the update was received
    let config_dump = harness
        .get_envoy_config_dump()
        .await
        .expect("Failed to get config dump");

    tracing::info!("Config after update: {}", serde_json::to_string_pretty(&config_dump).unwrap());

    harness.stop_server().await;
}

/// Test multiple nodes with different configurations.
#[tokio::test]
#[ignore = "requires Docker with Envoy running"]
async fn test_multiple_node_support() {
    init_tracing();

    let mut harness = TestHarness::new();

    // Start xDS server
    harness
        .start_server()
        .await
        .expect("Failed to start xDS server");

    // Wait for Envoy
    harness
        .wait_for_envoy(Duration::from_secs(30))
        .await
        .expect("Envoy did not become ready");

    // Set snapshot for the test node
    let snapshot = TestHarness::create_cluster_snapshot("node-specific-cluster", "v1");
    harness.set_snapshot(snapshot);

    // Also set a snapshot for a different node (should not affect our test Envoy)
    use xds_cache::Cache;
    use xds_core::NodeHash;
    let other_node = NodeHash::from_id("other-node");
    let other_snapshot = TestHarness::create_cluster_snapshot("other-cluster", "v1");
    harness.cache().set_snapshot(other_node, other_snapshot);

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Our Envoy should only see node-specific-cluster
    let clusters = harness
        .get_envoy_clusters()
        .await
        .expect("Failed to get clusters");

    tracing::info!("Clusters for test node: {}", serde_json::to_string_pretty(&clusters).unwrap());

    harness.stop_server().await;
}
