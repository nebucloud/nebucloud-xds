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
│   ├── simple-server/
│   └── kubernetes-controller/
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
        .resources(TypeUrl::Cluster, vec![])
        .resources(TypeUrl::Listener, vec![])
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

## Roadmap

See [MILESTONES.md](./MILESTONES.md) for the development roadmap:

1. **M1: Foundation** - Workspace structure and tooling 🟡 (25%)
2. **M2: Core Implementation** - Types, cache, watches 🟡 (75%)
3. **M3: Protocols** - SotW and Delta handlers 🔴
4. **M4: Production Readiness** - Metrics, health checks, graceful shutdown 🔴
5. **M5: Examples & Documentation** - Examples and guides 🔴
6. **M6: Release** - Publishing and versioning 🔴

## Contributing

See the GitHub issues for tasks organized by milestone.

## License

Apache-2.0 OR MIT (dual-licensed)
