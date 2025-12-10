# nebucloud-xds

Production-ready xDS control plane library for Rust.

## Overview

This library provides a complete implementation of the xDS protocol for building Envoy control planes. It supports:

- **State-of-the-World (SotW)** xDS protocol
- **Delta xDS** protocol (incremental updates)
- **Aggregated Discovery Service (ADS)**
- Individual xDS services (CDS, LDS, RDS, EDS, SDS)

## Architecture

The library is organized as a multi-crate workspace:

```
nebucloud-xds/
├── crates/
│   ├── xds-core/       # Core types, traits, error handling
│   ├── xds-cache/      # Snapshot cache with DashMap
│   ├── xds-server/     # gRPC server implementation
│   ├── xds-types/      # Generated protobuf types
│   └── nebucloud-xds/  # Facade crate (re-exports all)
├── examples/
│   ├── simple-server/          # Basic xDS server
│   ├── kubernetes-controller/  # K8s-native control plane
│   └── custom-control-plane/   # Domain-specific control plane
└── tests/
    └── integration/
```

## Design Principles

1. **No panics in library code** - All errors are returned as `Result<T, XdsError>`
2. **No locks held across await points** - Uses DashMap for lock-free concurrent access
3. **Type-safe resources** - Generic `Resource` trait with runtime registry
4. **Observable** - Built-in metrics and tracing support

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
nebucloud-xds = "0.1"
```

### Basic Usage

```rust
use nebucloud_xds::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a cache
    let cache = Arc::new(ShardedCache::new());

    // Build a snapshot for a node
    let snapshot = Snapshot::builder()
        .version("v1")
        .resources(TypeUrl::new(TypeUrl::CLUSTER), vec![])
        .resources(TypeUrl::new(TypeUrl::LISTENER), vec![])
        .build();

    // Set snapshot for a node
    let node = NodeHash::from_id("my-envoy-proxy");
    cache.set_snapshot(node, snapshot);

    // Create the xDS server
    let server = XdsServer::builder()
        .cache(cache)
        .enable_sotw()
        .enable_delta()
        .build()?;

    println!("xDS server ready!");
    Ok(())
}
```

### Watch for Updates

```rust
use nebucloud_xds::prelude::*;

async fn watch_example(cache: &ShardedCache) {
    let node = NodeHash::from_id("my-node");
    
    // Create a watch
    let mut watch = cache.create_watch(node);
    
    // Receive updates
    while let Some(snapshot) = watch.recv().await {
        println!("Got snapshot version: {}", snapshot.version());
    }
}
```

## Building

### Requirements

- Rust 1.75+ (MSRV)
- Protobuf compiler (for xds-types with full proto support)

### Build Commands

```bash
# Check the workspace
cargo check --workspace

# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Build with release optimizations
cargo build --workspace --release

# Run examples
cargo run --example simple-server
cargo run --example kubernetes-controller
```

### Development Tools

```bash
# Format code
cargo fmt --all

# Run clippy
cargo clippy --workspace --all-targets

# Check dependencies for security issues
cargo deny check

# Generate documentation
cargo doc --workspace --no-deps --open
```

## Crates

### xds-core

Core types and traits:
- `XdsError` - Comprehensive error type with 16+ variants
- `Resource` trait - Type-erased resource interface
- `NodeHash` - FNV-1a based node identification
- `TypeUrl` - xDS type URL constants
- `ResourceVersion` - SHA-256 based versioning

### xds-cache

High-performance snapshot caching:
- `ShardedCache` - DashMap-based concurrent cache
- `Snapshot` - Immutable resource collection
- `WatchManager` - Subscription system for updates
- `CacheStats` - Metrics for cache operations

### xds-server

gRPC server implementation:
- `XdsServer` - Main server type
- `XdsServerBuilder` - Fluent configuration API
- `SotwHandler` - State-of-the-World protocol
- `DeltaHandler` - Delta xDS protocol
- `StreamContext` - Per-stream metadata

### xds-types

Envoy protobuf types (stub types included, full proto generation optional):
- Discovery request/response types
- Resource types (Cluster, Listener, Route, Endpoint)
- Node and configuration types

### nebucloud-xds

Facade crate that re-exports all public APIs:
- `prelude` module for convenient imports
- Version information

## Examples

### simple-server

A basic xDS control plane demonstrating:
- Cache setup with sample resources
- Periodic snapshot updates
- Statistics reporting

### kubernetes-controller

A Kubernetes-native control plane scaffold:
- Simulated Kubernetes watch events
- Service/Endpoint to xDS translation
- Multi-node type support

### custom-control-plane

A **domain-specific control plane** showing how external consumers can use this library:
- Business domain types (Load, Vehicle, Assignment)
- Domain → xDS resource conversion patterns
- Snapshot building from domain state
- Multi-node-type support (dispatcher, vehicles, depots)

This example demonstrates the pattern for building custom control planes for use cases like:
- Logistics/trucking dispatch
- IoT device management
- Gaming server orchestration
- Any domain that benefits from dynamic configuration distribution

## Building Custom Control Planes

This library is designed to be used as a foundation for building domain-specific control planes. The key pattern is:

1. **Define domain types** - Your business objects without xDS dependencies
2. **Create resource wrappers** - Implement the `Resource` trait for xDS types
3. **Build conversion layer** - Transform domain objects to xDS resources
4. **Manage snapshots** - Build and push snapshots on domain changes

```rust
use nebucloud_xds::prelude::*;
use std::sync::Arc;

// 1. Define your domain type
struct Vehicle {
    id: String,
    location: (f64, f64),
    capacity: u32,
}

// 2. Create a resource wrapper
#[derive(Debug)]
struct VehicleEndpoint {
    cluster_name: String,
    vehicles: Vec<Vehicle>,
}

impl Resource for VehicleEndpoint {
    fn type_url(&self) -> &str { TypeUrl::ENDPOINT }
    fn name(&self) -> &str { &self.cluster_name }
    fn encode(&self) -> Result<prost_types::Any, _> { /* encode */ }
    fn as_any(&self) -> &dyn std::any::Any { self }
}

// 3. Build snapshots from your domain
fn build_snapshot(vehicles: &[Vehicle]) -> Snapshot {
    let endpoint = Arc::new(VehicleEndpoint {
        cluster_name: "fleet".into(),
        vehicles: vehicles.to_vec(),
    });
    
    Snapshot::builder()
        .version("v1")
        .resources(TypeUrl::new(TypeUrl::ENDPOINT), vec![endpoint])
        .build()
}

// 4. Push to cache (Envoy clients get notified automatically)
fn update_fleet(cache: &ShardedCache, vehicles: &[Vehicle]) {
    let snapshot = build_snapshot(vehicles);
    cache.set_snapshot(NodeHash::from_id("fleet-node"), snapshot);
}
```

See `examples/custom-control-plane/` for a complete implementation.

## Roadmap

See [MILESTONES.md](./MILESTONES.md) for the full development roadmap:

| Milestone | Description | Status |
|-----------|-------------|--------|
| **M1: Foundation** | Workspace structure and tooling | 🟢 Complete |
| **M2: Core Implementation** | Types, cache, watches | 🟢 Complete |
| **M3: Protocol Handlers** | SotW and Delta handlers | 🟢 Complete |
| **M4: Production Readiness** | Metrics, health checks, graceful shutdown | 🟢 Complete |
| **M5: Examples & Documentation** | Examples and guides | 🟡 50% |
| **M6: Release** | Publishing and versioning | 🟡 33% |

## Contributing

See the GitHub issues for tasks organized by milestone.

### Versioning

This project uses [Changesets](https://github.com/changesets/changesets) for version management and [git-cliff](https://git-cliff.org) for changelog generation.

**Adding a changeset:**

```bash
# Create a changeset file manually in .changeset/
# Or describe your changes in the PR
```

**Creating a release:**

1. Changesets accumulate in `.changeset/`
2. Run the Release workflow from GitHub Actions
3. Select the version bump type (patch/minor/major)
4. Versions are bumped, CHANGELOG updated, and published to crates.io

## License

Apache-2.0 OR MIT (dual-licensed)
