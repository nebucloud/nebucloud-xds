//! Kubernetes Controller Example
//!
//! This example demonstrates a Kubernetes-native xDS control plane that:
//! - Watches Kubernetes resources (Services, Endpoints, etc.)
//! - Translates them to xDS resources
//! - Serves them to Envoy proxies
//!
//! This is a scaffold - actual Kubernetes integration would use kube-rs.
//!
//! Run with:
//! ```bash
//! cargo run --example kubernetes-controller
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use nebucloud_xds::prelude::*;
use tokio::signal;
use tokio::sync::mpsc;
use tracing::{debug, info, Level};
use tracing_subscriber::FmtSubscriber;

/// Kubernetes resource event.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Example code showing API surface
enum KubeEvent {
    /// Service added or updated.
    ServiceUpsert {
        name: String,
        namespace: String,
        ports: Vec<u32>,
    },
    /// Service deleted.
    ServiceDelete { name: String, namespace: String },
    /// Endpoints updated.
    EndpointsUpdate {
        name: String,
        namespace: String,
        addresses: Vec<String>,
    },
}

/// Controller state.
struct Controller {
    /// xDS cache.
    cache: Arc<ShardedCache>,
    /// Known services.
    services: HashMap<String, ServiceInfo>,
    /// Known endpoints.
    endpoints: HashMap<String, Vec<String>>,
    /// Current version.
    version: u64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Example code showing API surface
struct ServiceInfo {
    name: String,
    namespace: String,
    ports: Vec<u32>,
}

impl Controller {
    fn new(cache: Arc<ShardedCache>) -> Self {
        Self {
            cache,
            services: HashMap::new(),
            endpoints: HashMap::new(),
            version: 0,
        }
    }

    /// Handle a Kubernetes event.
    fn handle_event(&mut self, event: KubeEvent) {
        match event {
            KubeEvent::ServiceUpsert {
                name,
                namespace,
                ports,
            } => {
                let key = format!("{}/{}", namespace, name);
                info!(service = %key, "Service upserted");
                self.services.insert(
                    key,
                    ServiceInfo {
                        name,
                        namespace,
                        ports,
                    },
                );
                self.rebuild_snapshots();
            }
            KubeEvent::ServiceDelete { name, namespace } => {
                let key = format!("{}/{}", namespace, name);
                info!(service = %key, "Service deleted");
                self.services.remove(&key);
                self.rebuild_snapshots();
            }
            KubeEvent::EndpointsUpdate {
                name,
                namespace,
                addresses,
            } => {
                let key = format!("{}/{}", namespace, name);
                debug!(endpoints = %key, count = addresses.len(), "Endpoints updated");
                self.endpoints.insert(key, addresses);
                self.rebuild_snapshots();
            }
        }
    }

    /// Rebuild snapshots for all nodes.
    fn rebuild_snapshots(&mut self) {
        self.version += 1;
        let version = format!("k8s-{}", self.version);

        info!(version = %version, services = self.services.len(), "Rebuilding snapshots");

        // Build cluster resources from services
        // In a real implementation, this would create actual Cluster protos

        // Build endpoint resources
        // In a real implementation, this would create ClusterLoadAssignment protos

        // Create snapshot for mesh sidecars
        let sidecar_snapshot = Snapshot::builder()
            .version(&version)
            .resources(TypeUrl::new(TypeUrl::CLUSTER), vec![])
            .resources(TypeUrl::new(TypeUrl::ENDPOINT), vec![])
            .build();

        self.cache
            .set_snapshot(NodeHash::from_id("mesh-sidecar"), sidecar_snapshot);

        // Create snapshot for ingress gateways
        let gateway_snapshot = Snapshot::builder()
            .version(&version)
            .resources(TypeUrl::new(TypeUrl::LISTENER), vec![])
            .resources(TypeUrl::new(TypeUrl::ROUTE), vec![])
            .resources(TypeUrl::new(TypeUrl::CLUSTER), vec![])
            .build();

        self.cache
            .set_snapshot(NodeHash::from_id("ingress-gateway"), gateway_snapshot);

        info!(
            version = %version,
            snapshot_count = self.cache.snapshot_count(),
            "Snapshots rebuilt"
        );
    }
}

/// Simulate Kubernetes watch events.
async fn simulate_k8s_events(tx: mpsc::Sender<KubeEvent>) {
    let mut interval = tokio::time::interval(Duration::from_secs(5));

    // Initial services
    let services = vec![
        ("frontend", "default", vec![80]),
        ("backend-api", "default", vec![8080]),
        ("cache", "infrastructure", vec![6379]),
        ("database", "infrastructure", vec![5432]),
    ];

    // Add initial services
    for (name, namespace, ports) in &services {
        if tx
            .send(KubeEvent::ServiceUpsert {
                name: name.to_string(),
                namespace: namespace.to_string(),
                ports: ports.clone(),
            })
            .await
            .is_err()
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Add endpoints for each service
    for (name, namespace, _) in &services {
        if tx
            .send(KubeEvent::EndpointsUpdate {
                name: name.to_string(),
                namespace: namespace.to_string(),
                addresses: vec![
                    format!("10.0.0.{}", rand::random::<u8>()),
                    format!("10.0.0.{}", rand::random::<u8>()),
                ],
            })
            .await
            .is_err()
        {
            return;
        }
    }

    // Simulate ongoing changes
    loop {
        interval.tick().await;

        // Randomly update an endpoint
        let idx = rand::random::<usize>() % services.len();
        let (name, namespace, _) = &services[idx];

        if tx
            .send(KubeEvent::EndpointsUpdate {
                name: name.to_string(),
                namespace: namespace.to_string(),
                addresses: vec![
                    format!("10.0.0.{}", rand::random::<u8>()),
                    format!("10.0.0.{}", rand::random::<u8>()),
                    format!("10.0.0.{}", rand::random::<u8>()),
                ],
            })
            .await
            .is_err()
        {
            return;
        }
    }
}

mod rand {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static SEED: AtomicU64 = AtomicU64::new(0);

    pub fn random<T: From<u8>>() -> T {
        let mut seed = SEED.load(Ordering::Relaxed);
        if seed == 0 {
            seed = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;
        }
        // Simple LCG
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        SEED.store(seed, Ordering::Relaxed);
        T::from((seed >> 56) as u8)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Kubernetes controller example");
    info!("{}", nebucloud_xds::version::version_string());

    // Create the cache
    let cache = Arc::new(ShardedCache::new());

    // Create the controller
    let mut controller = Controller::new(Arc::clone(&cache));

    // Create event channel
    let (tx, mut rx) = mpsc::channel::<KubeEvent>(100);

    // Start simulated Kubernetes events
    tokio::spawn(simulate_k8s_events(tx));

    // Start the xDS server (placeholder)
    let _server = XdsServer::builder()
        .cache(Arc::clone(&cache))
        .enable_sotw()
        .enable_delta()
        .build()?;

    info!("Controller started, watching for Kubernetes events...");
    info!("Press Ctrl+C to shutdown");

    // Process events
    loop {
        tokio::select! {
            Some(event) = rx.recv() => {
                controller.handle_event(event);
            }
            _ = signal::ctrl_c() => {
                info!("Shutting down...");
                break;
            }
        }
    }

    Ok(())
}
