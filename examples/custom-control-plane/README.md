# Custom Control Plane Example

This example demonstrates how to build a **domain-specific xDS control plane** using `nebucloud-xds` as a library. It shows the pattern that external consumers (like a trucking dispatch system) would follow.

## Overview

The example implements a **Logistics Dispatch Control Plane** that maps trucking domain concepts to xDS resources:

| Domain Concept | xDS Resource | Purpose |
|----------------|--------------|---------|
| Load Category | Cluster | Group similar loads (e.g., "refrigerated") |
| Vehicle Fleet | ClusterLoadAssignment | Available trucks with health/capacity |
| Assignment | RouteConfiguration | Route load requests to assigned vehicle |
| Region/Depot | Endpoints by locality | Geographic load balancing |

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        Your Application                                  │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────────────┐  │
│  │   Domain     │───▶│  Conversion  │───▶│  nebucloud-xds           │  │
│  │   Types      │    │  Layer       │    │  (Cache + Server)        │  │
│  │              │    │              │    │                          │  │
│  │  - Load      │    │  - ToCluster │    │  - ShardedCache          │  │
│  │  - Vehicle   │    │  - ToEndpoint│    │  - XdsServer             │  │
│  │  - Assignment│    │  - ToRoute   │    │  - Snapshot              │  │
│  └──────────────┘    └──────────────┘    └──────────────────────────┘  │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
                                   │
                                   ▼ xDS Protocol (gRPC)
                          ┌────────────────┐
                          │  Envoy Proxies │
                          │  (Vehicles,    │
                          │   Depots, etc) │
                          └────────────────┘
```

## Project Structure

```
src/
├── main.rs         # Entry point - server setup and background tasks
├── domain.rs       # Business domain types (Load, Vehicle, Assignment)
├── conversion.rs   # Domain → xDS resource conversion
└── snapshot.rs     # Snapshot builder from domain state
```

## Key Concepts

### 1. Domain Types (`domain.rs`)

Define your business objects without any xDS dependencies:

```rust
pub struct Load {
    pub id: String,
    pub name: String,
    pub origin: Location,
    pub destination: Location,
    pub weight_lbs: u32,
    pub requirements: Vec<String>, // e.g., ["refrigerated"]
    pub status: LoadStatus,
}

pub struct Vehicle {
    pub id: String,
    pub driver_name: String,
    pub location: Location,
    pub capacity_lbs: u32,
    pub capabilities: Vec<String>,
    pub health_score: f64,
}
```

### 2. Resource Wrappers (`conversion.rs`)

Create wrapper types that implement the `Resource` trait:

```rust
#[derive(Debug, Clone)]
pub struct ClusterResource {
    pub name: String,
    pub category: String,
}

impl Resource for ClusterResource {
    fn type_url(&self) -> &str { TypeUrl::CLUSTER }
    fn name(&self) -> &str { &self.name }
    
    fn encode(&self) -> Result<prost_types::Any, _> {
        // Encode to protobuf
        Ok(prost_types::Any { ... })
    }
    
    fn as_any(&self) -> &dyn Any { self }
}
```

### 3. Snapshot Building (`snapshot.rs`)

Build xDS snapshots from your domain state:

```rust
pub fn build_dispatcher_snapshot(&self) -> Snapshot {
    let clusters = self.build_load_clusters(&available_loads);
    let endpoints = self.build_vehicle_endpoints(&available_vehicles);
    let routes = self.build_assignment_routes();

    Snapshot::builder()
        .version(&version)
        .resources(TypeUrl::new(TypeUrl::CLUSTER), clusters)
        .resources(TypeUrl::new(TypeUrl::ENDPOINT), endpoints)
        .resources(TypeUrl::new(TypeUrl::ROUTE), routes)
        .build()
}
```

### 4. Cache Integration (`main.rs`)

Push snapshots to the cache, which automatically notifies connected Envoys:

```rust
let cache = Arc::new(ShardedCache::new());

// Build and set snapshot
let snapshot = state.build_dispatcher_snapshot();
cache.set_snapshot(NodeHash::from_id("dispatcher-node"), snapshot);

// Create server
let server = XdsServer::builder()
    .cache(Arc::clone(&cache))
    .enable_sotw()
    .enable_delta()
    .build()?;
```

## Running

```bash
# From the nebucloud-xds root
cargo run -p custom-control-plane-example
```

Output:
```
╔═══════════════════════════════════════════════════════════╗
║     Custom Control Plane Example - Logistics Dispatch     ║
╚═══════════════════════════════════════════════════════════╝

nebucloud-xds 0.1.0 (MSRV 1.75)
✓ Created xDS snapshot cache
✓ Initialized dispatch state manager
Loading sample data...
  ✓ Added 3 sample loads
  ✓ Added 4 sample vehicles
  ✓ Added 1 sample assignment
Built dispatcher snapshot version=v1 clusters=2 endpoints=3 routes=1
✓ Built initial xDS snapshots
✓ xDS server configured
  └─ SotW enabled: true
  └─ Delta enabled: true

🚀 Control plane ready!
   Would listen on: [::]:18000

   Node types:
   ├─ dispatcher-node  → Full dispatch view (loads, vehicles, routes)
   ├─ vehicle-*        → Per-vehicle assignments
   └─ depot-*          → Regional vehicle fleet
```

## Testing

```bash
cargo test -p custom-control-plane-example
```

## Using This Pattern

To build your own domain-specific control plane:

1. **Define domain types** - Your business objects without xDS dependencies
2. **Create resource wrappers** - Implement `Resource` trait for each xDS type you need
3. **Build conversion functions** - Transform domain → xDS resources
4. **Create a state manager** - Track domain state and build snapshots
5. **Integrate with cache** - Push snapshots on domain changes
6. **Run the server** - Use `XdsServer` to serve to Envoy clients

## Next Steps

For a production system, you would:

- Add actual protobuf encoding in `encode()` methods
- Implement a REST/gRPC API for domain mutations
- Add database persistence for domain state
- Implement webhook/event handlers for real-time updates
- Add authentication and RBAC for xDS clients
- Deploy Envoy sidecars to vehicles/depots
