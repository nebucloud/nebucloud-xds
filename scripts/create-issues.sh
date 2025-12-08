#!/bin/bash
# Create GitHub Issues for nebucloud-xds
# 
# Prerequisites:
#   - GitHub CLI installed: brew install gh
#   - Authenticated: gh auth login
#
# Usage: ./scripts/create-issues.sh
#
# This script creates all issues for the nebucloud-xds milestones.
# Issues are organized by milestone and include labels and assignments.

set -euo pipefail

REPO="nebucloud/nebucloud-xds"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Creating GitHub Issues for nebucloud-xds${NC}"
echo "Repository: $REPO"
echo ""

# Check if gh is installed
if ! command -v gh &> /dev/null; then
    echo -e "${RED}Error: GitHub CLI (gh) is not installed${NC}"
    echo "Install with: brew install gh"
    exit 1
fi

# Check if authenticated
if ! gh auth status &> /dev/null; then
    echo -e "${RED}Error: Not authenticated with GitHub CLI${NC}"
    echo "Run: gh auth login"
    exit 1
fi

# Create labels first
echo -e "${YELLOW}Creating labels...${NC}"

gh label create "P0-critical" --color "b60205" --description "Must fix before release" --repo "$REPO" 2>/dev/null || true
gh label create "P1-high" --color "d93f0b" --description "Should fix before release" --repo "$REPO" 2>/dev/null || true
gh label create "P2-medium" --color "fbca04" --description "Nice to have" --repo "$REPO" 2>/dev/null || true
gh label create "P3-low" --color "0e8a16" --description "Future consideration" --repo "$REPO" 2>/dev/null || true
gh label create "security" --color "7057ff" --description "Security related" --repo "$REPO" 2>/dev/null || true
gh label create "breaking-change" --color "b60205" --description "Breaking API change" --repo "$REPO" 2>/dev/null || true
gh label create "milestone:foundation" --color "c5def5" --description "M1: Foundation" --repo "$REPO" 2>/dev/null || true
gh label create "milestone:core" --color "c5def5" --description "M2: Core Implementation" --repo "$REPO" 2>/dev/null || true
gh label create "milestone:protocol" --color "c5def5" --description "M3: Protocol Handlers" --repo "$REPO" 2>/dev/null || true
gh label create "milestone:production" --color "c5def5" --description "M4: Production Readiness" --repo "$REPO" 2>/dev/null || true
gh label create "milestone:docs" --color "c5def5" --description "M5: Examples & Documentation" --repo "$REPO" 2>/dev/null || true
gh label create "milestone:release" --color "c5def5" --description "M6: Release" --repo "$REPO" 2>/dev/null || true

echo -e "${GREEN}Labels created!${NC}"
echo ""

# Create milestones
echo -e "${YELLOW}Creating milestones...${NC}"

gh api repos/$REPO/milestones -f title="M1: Foundation" -f description="Establish workspace structure, update proto submodules, and set up CI/CD infrastructure" -f due_on="2025-12-22T00:00:00Z" 2>/dev/null || true
gh api repos/$REPO/milestones -f title="M2: Core Implementation" -f description="Implement core types, error handling, and cache system with production-grade patterns" -f due_on="2026-01-05T00:00:00Z" 2>/dev/null || true
gh api repos/$REPO/milestones -f title="M3: Protocol Handlers" -f description="Implement SotW and Delta xDS protocol handlers with proper stream management" -f due_on="2026-01-19T00:00:00Z" 2>/dev/null || true
gh api repos/$REPO/milestones -f title="M4: Production Readiness" -f description="Add observability, health checks, graceful shutdown, and production hardening" -f due_on="2026-02-02T00:00:00Z" 2>/dev/null || true
gh api repos/$REPO/milestones -f title="M5: Examples & Documentation" -f description="Create comprehensive examples and documentation for users" -f due_on="2026-02-16T00:00:00Z" 2>/dev/null || true
gh api repos/$REPO/milestones -f title="M6: Release" -f description="Prepare and publish v0.1.0 to crates.io" -f due_on="2026-03-01T00:00:00Z" 2>/dev/null || true

echo -e "${GREEN}Milestones created!${NC}"
echo ""

# Function to create an issue
create_issue() {
    local title="$1"
    local body="$2"
    local labels="$3"
    local milestone="$4"
    
    echo "  Creating: $title"
    gh issue create \
        --repo "$REPO" \
        --title "$title" \
        --body "$body" \
        --label "$labels" \
        --milestone "$milestone" \
        2>/dev/null || echo "    (may already exist)"
}

# ============================================================================
# M1: Foundation Issues
# ============================================================================
echo -e "${YELLOW}Creating M1: Foundation issues...${NC}"

create_issue \
    "Create 5-crate workspace structure" \
    "## Summary
Create the monorepo workspace structure with 5 crates:

- \`xds-core\` - Core types, errors, and traits
- \`xds-cache\` - Snapshot cache implementation
- \`xds-server\` - gRPC server implementation
- \`xds-types\` - Generated protobuf types
- \`nebucloud-xds\` - Facade crate re-exporting all public APIs

## Tasks
- [ ] Create workspace Cargo.toml with proper settings
- [ ] Create each crate with minimal structure
- [ ] Set up inter-crate dependencies
- [ ] Configure workspace-level dependencies
- [ ] Add deny.toml for security auditing
- [ ] Add rust-toolchain.toml (MSRV 1.75)

## Reference
See [IMPLEMENTATION_BLUEPRINT.md](./IMPLEMENTATION_BLUEPRINT.md) Step 1 for complete configuration.

## Acceptance Criteria
- [ ] \`cargo build\` succeeds
- [ ] \`cargo test\` runs (even with no tests)
- [ ] Workspace members correctly listed" \
    "P0-critical,milestone:foundation" \
    "M1: Foundation"

create_issue \
    "Update proto submodules (526 commits behind)" \
    "## Summary
The proto submodules are significantly behind upstream:

- \`data-plane-api\`: 526 commits behind (May 2024 → Dec 2025)
- \`xds\`: Unknown status
- \`googleapis\`: Unknown status

## Tasks
- [ ] Update data-plane-api submodule to latest
- [ ] Update xds submodule to latest
- [ ] Update googleapis submodule to latest
- [ ] Verify protobuf compilation succeeds
- [ ] Fix any breaking changes from upstream

## Risk
Proto updates may introduce breaking changes in the generated code. Thorough testing required.

## Acceptance Criteria
- [ ] All submodules at latest commit
- [ ] \`xds-types\` crate builds with new protos
- [ ] No compilation errors" \
    "P0-critical,milestone:foundation" \
    "M1: Foundation"

create_issue \
    "Set up cargo-deny and security auditing" \
    "## Summary
Configure cargo-deny for dependency auditing and license compliance.

## Tasks
- [ ] Create deny.toml with proper configuration
- [ ] Configure license allowlist (MIT, Apache-2.0, BSD-3-Clause, etc.)
- [ ] Set up advisory database checks
- [ ] Configure duplicate detection
- [ ] Add to CI pipeline

## Configuration
\`\`\`toml
[advisories]
db-path = \"~/.cargo/advisory-db\"
vulnerability = \"deny\"
unmaintained = \"warn\"
yanked = \"deny\"
\`\`\`

## Acceptance Criteria
- [ ] \`cargo deny check\` passes
- [ ] No known vulnerabilities
- [ ] Licenses are compliant" \
    "P0-critical,security,milestone:foundation" \
    "M1: Foundation"

create_issue \
    "Configure CI pipeline (GitHub Actions)" \
    "## Summary
Set up comprehensive CI pipeline with GitHub Actions.

## Tasks
- [ ] Create .github/workflows/ci.yml
- [ ] Add format check (cargo fmt)
- [ ] Add lint check (cargo clippy)
- [ ] Add test matrix (different features)
- [ ] Add code coverage with Codecov
- [ ] Add security audit (cargo-deny, cargo-audit)
- [ ] Add documentation build
- [ ] Add MSRV check (Rust 1.75)

## Reference
See [IMPLEMENTATION_BLUEPRINT.md](./IMPLEMENTATION_BLUEPRINT.md) Step 7 for complete workflow.

## Acceptance Criteria
- [ ] CI passes on main branch
- [ ] All checks green
- [ ] Coverage reporting working" \
    "P0-critical,milestone:foundation" \
    "M1: Foundation"

create_issue \
    "Set up proto-sync automation" \
    "## Summary
Create automated workflow to keep proto submodules up to date.

## Tasks
- [ ] Create .github/workflows/proto-sync.yml
- [ ] Configure weekly schedule (Sunday 00:00 UTC)
- [ ] Add manual trigger option
- [ ] Auto-create PR with changes
- [ ] Include change summary in PR body
- [ ] Verify tests pass before PR

## Acceptance Criteria
- [ ] Workflow runs on schedule
- [ ] Creates PRs automatically
- [ ] PRs include helpful description" \
    "P1-high,milestone:foundation" \
    "M1: Foundation"

create_issue \
    "Configure Dependabot" \
    "## Summary
Set up Dependabot for automated dependency updates.

## Tasks
- [ ] Create .github/dependabot.yml
- [ ] Configure weekly Cargo updates
- [ ] Configure GitHub Actions updates
- [ ] Set appropriate labels
- [ ] Configure commit message format

## Acceptance Criteria
- [ ] Dependabot PRs created for updates
- [ ] Proper labels applied" \
    "P1-high,milestone:foundation" \
    "M1: Foundation"

create_issue \
    "Add rust-toolchain.toml (MSRV 1.75)" \
    "## Summary
Pin the Rust toolchain version for consistent builds.

## Tasks
- [ ] Create rust-toolchain.toml
- [ ] Set channel to stable
- [ ] Document MSRV in README
- [ ] Add MSRV check to CI

## Configuration
\`\`\`toml
[toolchain]
channel = \"1.75\"
components = [\"rustfmt\", \"clippy\"]
\`\`\`

## Acceptance Criteria
- [ ] Toolchain file present
- [ ] CI verifies MSRV compatibility" \
    "P1-high,milestone:foundation" \
    "M1: Foundation"

create_issue \
    "Set up Changesets for versioning" \
    "## Summary
Configure Changesets for semantic versioning and changelog generation.

## Tasks
- [ ] Install @changesets/cli
- [ ] Create .changeset/config.json
- [ ] Configure changelog format (git-cliff)
- [ ] Create release workflow
- [ ] Document versioning process

## Acceptance Criteria
- [ ] Can create changesets
- [ ] Version bumps work correctly
- [ ] Changelogs are generated" \
    "P1-high,milestone:foundation" \
    "M1: Foundation"

echo -e "${GREEN}M1 issues created!${NC}"
echo ""

# ============================================================================
# M2: Core Implementation Issues
# ============================================================================
echo -e "${YELLOW}Creating M2: Core Implementation issues...${NC}"

create_issue \
    "Implement XdsError with 16 error variants" \
    "## Summary
Create comprehensive error type replacing all panics with proper error handling.

## Error Variants
1. \`InvalidTypeUrl\` - Malformed or unknown type URL
2. \`ResourceNotFound\` - Requested resource doesn't exist
3. \`VersionMismatch\` - Version conflict during update
4. \`InvalidResource\` - Resource validation failed
5. \`CacheError\` - Cache operation failed
6. \`WatchError\` - Watch creation/notification failed
7. \`SnapshotIncomplete\` - Missing required resource types
8. \`EncodingError\` - Protobuf encoding failed
9. \`DecodingError\` - Protobuf decoding failed
10. \`TransportError\` - gRPC transport error
11. \`StreamClosed\` - Client stream closed unexpectedly
12. \`NackReceived\` - Client rejected configuration
13. \`Timeout\` - Operation timed out
14. \`Shutdown\` - Server is shutting down
15. \`RateLimited\` - Too many requests
16. \`Internal\` - Unexpected internal error

## Tasks
- [ ] Create XdsError enum with thiserror
- [ ] Implement Display for all variants
- [ ] Implement From<XdsError> for tonic::Status
- [ ] Add error source chaining where applicable
- [ ] Write unit tests for all conversions

## Reference
See [IMPLEMENTATION_BLUEPRINT.md](./IMPLEMENTATION_BLUEPRINT.md) Step 2.

## Acceptance Criteria
- [ ] All error variants defined
- [ ] Clean conversion to gRPC status codes
- [ ] No panics in error paths" \
    "P0-critical,milestone:core" \
    "M2: Core Implementation"

create_issue \
    "Add ResourceVersion and NodeHash types" \
    "## Summary
Create newtype wrappers for version tracking and node identification.

## Types

### ResourceVersion
- Wraps version string
- Supports empty version (initial state)
- Implements ordering for comparison

### NodeHash
- Efficient node identification
- FNV-1a hashing for performance
- Wildcard support for broadcast

## Tasks
- [ ] Implement ResourceVersion with comparison
- [ ] Implement NodeHash with FNV hashing
- [ ] Add From/Into conversions
- [ ] Write property tests

## Reference
See [IMPLEMENTATION_BLUEPRINT.md](./IMPLEMENTATION_BLUEPRINT.md) Step 2.

## Acceptance Criteria
- [ ] Types are ergonomic to use
- [ ] Hashing is deterministic
- [ ] Good performance characteristics" \
    "P0-critical,milestone:core" \
    "M2: Core Implementation"

create_issue \
    "Create generic Resource trait" \
    "## Summary
Replace hardcoded Resource enum with extensible trait-based system.

## Current Problem
The existing code has a hardcoded \`Resource\` enum that can't be extended by users.

## Solution
Create a \`Resource\` trait that users can implement for custom resource types.

## Tasks
- [ ] Define Resource trait with type_url(), name(), encode()
- [ ] Create BoxResource type alias
- [ ] Implement for standard Envoy resources
- [ ] Create ResourceRegistry for type management
- [ ] Write documentation with examples

## Reference
See [IMPLEMENTATION_BLUEPRINT.md](./IMPLEMENTATION_BLUEPRINT.md) Step 3.

## Acceptance Criteria
- [ ] Trait is object-safe
- [ ] Can implement for custom types
- [ ] Registry correctly tracks types" \
    "P0-critical,milestone:core" \
    "M2: Core Implementation"

create_issue \
    "Implement ResourceRegistry" \
    "## Summary
Create a registry for managing resource types and their metadata.

## Features
- Register resource types with type URLs
- Look up resources by type URL
- Validate resource type compatibility
- Support for custom resource types

## Tasks
- [ ] Implement ResourceRegistry struct
- [ ] Add registration methods
- [ ] Add lookup methods
- [ ] Pre-register standard Envoy types
- [ ] Write tests

## Acceptance Criteria
- [ ] Can register custom types
- [ ] Lookup is O(1)
- [ ] Thread-safe" \
    "P0-critical,milestone:core" \
    "M2: Core Implementation"

create_issue \
    "Build ShardedCache with DashMap" \
    "## Summary
Implement production-grade cache using DashMap instead of RwLock.

## Current Problem
The existing cache uses RwLock which is held across await points, causing potential deadlocks.

## Solution
Use DashMap for lock-free concurrent access, ensuring refs are dropped before any await.

## Critical Pattern
\`\`\`rust
// Get snapshot and immediately clone
let snapshot = {
    let entry = self.snapshots.get(&node_hash);
    entry.map(|e| e.value().clone())
};
// Now safe to await - no DashMap ref held
\`\`\`

## Tasks
- [ ] Implement ShardedCache with DashMap
- [ ] Add snapshot storage and retrieval
- [ ] Implement watch management
- [ ] Add cache statistics
- [ ] Write stress tests
- [ ] Verify no lock-across-await

## Reference
See [IMPLEMENTATION_BLUEPRINT.md](./IMPLEMENTATION_BLUEPRINT.md) Step 4.

## Acceptance Criteria
- [ ] No deadlocks under load
- [ ] DashMap refs never held across await
- [ ] Stress tests pass with 1000+ concurrent ops" \
    "P0-critical,milestone:core" \
    "M2: Core Implementation"

create_issue \
    "Implement Watch and WatchManager" \
    "## Summary
Create the watch notification system for streaming updates to clients.

## Components

### Watch
- Represents a single client subscription
- Contains channel for sending updates
- Tracks subscribed resources and versions

### WatchManager
- Manages all active watches
- Routes updates to relevant watches
- Cleans up closed watches

## Tasks
- [ ] Implement Watch struct
- [ ] Implement WatchSender with backpressure
- [ ] Implement WatchManager
- [ ] Add cancellation support
- [ ] Write concurrency tests

## Reference
See [IMPLEMENTATION_BLUEPRINT.md](./IMPLEMENTATION_BLUEPRINT.md) Step 4.

## Acceptance Criteria
- [ ] Watches receive updates promptly
- [ ] Closed watches are cleaned up
- [ ] Backpressure handled correctly" \
    "P0-critical,milestone:core" \
    "M2: Core Implementation"

create_issue \
    "Add Snapshot builder pattern" \
    "## Summary
Create ergonomic builder for constructing resource snapshots.

## API Design
\`\`\`rust
let snapshot = Snapshot::builder()
    .version(\"v1\")
    .resource(TYPE_URL_CDS, \"1\", \"cluster-1\", cluster)
    .resource(TYPE_URL_EDS, \"1\", \"cluster-1\", endpoints)
    .build();
\`\`\`

## Tasks
- [ ] Implement SnapshotBuilder
- [ ] Support multiple resource types
- [ ] Add validation on build
- [ ] Support consistent versioning
- [ ] Write builder tests

## Reference
See [IMPLEMENTATION_BLUEPRINT.md](./IMPLEMENTATION_BLUEPRINT.md) Step 4.

## Acceptance Criteria
- [ ] Builder is ergonomic
- [ ] Validation catches errors early
- [ ] Good error messages" \
    "P1-high,milestone:core" \
    "M2: Core Implementation"

create_issue \
    "Write unit tests for cache" \
    "## Summary
Comprehensive test coverage for the cache system.

## Test Cases
- [ ] Basic CRUD operations
- [ ] Concurrent access patterns
- [ ] Watch notification delivery
- [ ] Watch cleanup on client disconnect
- [ ] Snapshot versioning
- [ ] Large snapshot handling
- [ ] Memory usage under load

## Tasks
- [ ] Write unit tests for ShardedCache
- [ ] Write unit tests for WatchManager
- [ ] Write property tests
- [ ] Add benchmarks
- [ ] Achieve 80%+ coverage

## Acceptance Criteria
- [ ] 80%+ code coverage
- [ ] All edge cases covered
- [ ] Benchmarks establish baseline" \
    "P0-critical,milestone:core" \
    "M2: Core Implementation"

echo -e "${GREEN}M2 issues created!${NC}"
echo ""

# ============================================================================
# M3: Protocol Handlers Issues
# ============================================================================
echo -e "${YELLOW}Creating M3: Protocol Handlers issues...${NC}"

create_issue \
    "Implement SotwStreamHandler" \
    "## Summary
Implement State of the World (SotW) xDS protocol handler.

## Protocol Overview
SotW sends complete resource sets on each update. Client tracks versions via version_info.

## Features
- Handle DiscoveryRequest messages
- Generate DiscoveryResponse with full resource set
- Track client version per type
- Handle ACK/NACK properly
- Support resource subscriptions

## Tasks
- [ ] Implement SotwStreamHandler struct
- [ ] Add request processing logic
- [ ] Add response generation
- [ ] Handle version tracking
- [ ] Handle ACK/NACK
- [ ] Write protocol tests

## Reference
See [IMPLEMENTATION_BLUEPRINT.md](./IMPLEMENTATION_BLUEPRINT.md) Step 6.

## Acceptance Criteria
- [ ] Works with Envoy (SotW mode)
- [ ] Version tracking correct
- [ ] NACK handling proper" \
    "P0-critical,milestone:protocol" \
    "M3: Protocol Handlers"

create_issue \
    "Implement DeltaStreamHandler" \
    "## Summary
Implement Delta (incremental) xDS protocol handler.

## Protocol Overview
Delta sends only changed resources, more efficient for large configs.

## Features
- Handle DeltaDiscoveryRequest messages
- Generate DeltaDiscoveryResponse with changes only
- Track per-resource versions
- Handle resource subscriptions/unsubscriptions
- Support resource removal

## Tasks
- [ ] Implement DeltaStreamHandler struct
- [ ] Add delta request processing
- [ ] Add incremental response generation
- [ ] Track resource-level versions
- [ ] Handle subscribe/unsubscribe
- [ ] Write protocol tests

## Reference
See [IMPLEMENTATION_BLUEPRINT.md](./IMPLEMENTATION_BLUEPRINT.md) Step 6.

## Acceptance Criteria
- [ ] Works with Envoy (Delta mode)
- [ ] Sends only changes
- [ ] Resource removal works" \
    "P0-critical,milestone:protocol" \
    "M3: Protocol Handlers"

create_issue \
    "Build AggregatedDiscoveryService (ADS)" \
    "## Summary
Implement the Aggregated Discovery Service for multiplexed xDS.

## Features
- Single gRPC stream for all resource types
- Type ordering guarantees (CDS before EDS, etc.)
- Both SotW and Delta variants
- Proper resource type detection

## Tasks
- [ ] Implement AggregatedDiscoveryService
- [ ] Add StreamAggregatedResources (SotW)
- [ ] Add DeltaAggregatedResources (Delta)
- [ ] Handle type multiplexing
- [ ] Write integration tests

## Reference
See [IMPLEMENTATION_BLUEPRINT.md](./IMPLEMENTATION_BLUEPRINT.md) Step 6.

## Acceptance Criteria
- [ ] ADS works with Envoy
- [ ] Type ordering correct
- [ ] Both protocols work" \
    "P0-critical,milestone:protocol" \
    "M3: Protocol Handlers"

create_issue \
    "Add CDS, EDS, LDS, RDS, SDS services" \
    "## Summary
Implement individual discovery services for each resource type.

## Services
- **CDS** - Cluster Discovery Service
- **EDS** - Endpoint Discovery Service
- **LDS** - Listener Discovery Service
- **RDS** - Route Discovery Service
- **SDS** - Secret Discovery Service

## Tasks
- [ ] Implement ClusterDiscoveryService
- [ ] Implement EndpointDiscoveryService
- [ ] Implement ListenerDiscoveryService
- [ ] Implement RouteDiscoveryService
- [ ] Implement SecretDiscoveryService
- [ ] Share stream handling logic
- [ ] Write tests for each service

## Acceptance Criteria
- [ ] All services work independently
- [ ] Can use without ADS
- [ ] Proper resource type validation" \
    "P1-high,milestone:protocol" \
    "M3: Protocol Handlers"

create_issue \
    "Implement proper NACK handling" \
    "## Summary
Handle client rejections (NACKs) gracefully.

## NACK Indicators
- error_detail field populated in request
- version_info unchanged from last ACK
- response_nonce matching previous response

## Tasks
- [ ] Detect NACK vs ACK
- [ ] Log NACK details
- [ ] Emit metrics for NACKs
- [ ] Support NACK callbacks
- [ ] Don't resend failed config
- [ ] Write NACK tests

## Acceptance Criteria
- [ ] NACKs detected correctly
- [ ] No infinite retry loops
- [ ] Metrics exposed" \
    "P0-critical,milestone:protocol" \
    "M3: Protocol Handlers"

create_issue \
    "Add resource subscription tracking" \
    "## Summary
Track which resources each client has subscribed to.

## Features
- Track subscribed resource names per client
- Support wildcard subscriptions
- Handle subscription changes mid-stream
- Efficient lookup for updates

## Tasks
- [ ] Implement subscription tracking structure
- [ ] Handle initial subscription
- [ ] Handle subscription updates
- [ ] Optimize for common patterns
- [ ] Write tests

## Acceptance Criteria
- [ ] Accurate tracking
- [ ] Handles wildcards
- [ ] Efficient updates" \
    "P1-high,milestone:protocol" \
    "M3: Protocol Handlers"

create_issue \
    "Write protocol compliance tests" \
    "## Summary
Test xDS protocol compliance against Envoy.

## Test Scenarios
- [ ] Initial connection and resource fetch
- [ ] Resource updates trigger responses
- [ ] ACK advances version
- [ ] NACK doesn't advance version
- [ ] Subscription changes
- [ ] Connection recovery
- [ ] Type ordering in ADS

## Tasks
- [ ] Set up Envoy testcontainer
- [ ] Write SotW compliance tests
- [ ] Write Delta compliance tests
- [ ] Write ADS compliance tests
- [ ] Document test scenarios

## Acceptance Criteria
- [ ] All tests pass with real Envoy
- [ ] Protocol edge cases covered
- [ ] Tests run in CI" \
    "P0-critical,milestone:protocol" \
    "M3: Protocol Handlers"

echo -e "${GREEN}M3 issues created!${NC}"
echo ""

# ============================================================================
# M4: Production Readiness Issues
# ============================================================================
echo -e "${YELLOW}Creating M4: Production Readiness issues...${NC}"

create_issue \
    "Implement XdsServerBuilder" \
    "## Summary
Create builder pattern for configuring the xDS server.

## Configuration Options
- Address/port
- Cache implementation
- Health check endpoint
- Metrics endpoint
- gRPC reflection
- TLS configuration
- Graceful shutdown timeout
- Connection limits

## API Design
\`\`\`rust
let server = XdsServer::builder()
    .with_cache(cache)
    .with_address(\"[::]:18000\".parse()?)
    .with_health_check()
    .with_metrics()
    .with_graceful_shutdown(Duration::from_secs(30))
    .build()?;
\`\`\`

## Tasks
- [ ] Implement XdsServerBuilder
- [ ] Add all configuration options
- [ ] Validate configuration on build
- [ ] Write tests
- [ ] Document all options

## Reference
See [IMPLEMENTATION_BLUEPRINT.md](./IMPLEMENTATION_BLUEPRINT.md) Step 5.

## Acceptance Criteria
- [ ] Builder is ergonomic
- [ ] All options work correctly
- [ ] Good defaults" \
    "P0-critical,milestone:production" \
    "M4: Production Readiness"

create_issue \
    "Add Prometheus metrics (XdsMetrics)" \
    "## Summary
Expose Prometheus metrics for observability.

## Metrics
### Counters
- \`xds_requests_total{type_url, node_id}\`
- \`xds_responses_total{type_url, node_id}\`
- \`xds_nacks_total{type_url, node_id}\`

### Gauges
- \`xds_connected_clients\`
- \`xds_active_watches\`
- \`xds_snapshot_count\`

### Histograms
- \`xds_request_duration_seconds\`
- \`xds_cache_operation_duration_seconds\`

## Tasks
- [ ] Implement XdsMetrics struct
- [ ] Add counter methods
- [ ] Add gauge methods
- [ ] Add histogram methods
- [ ] Expose /metrics endpoint
- [ ] Write tests
- [ ] Add Grafana dashboard example

## Acceptance Criteria
- [ ] Metrics scraped by Prometheus
- [ ] Useful for debugging
- [ ] Low overhead" \
    "P0-critical,milestone:production" \
    "M4: Production Readiness"

create_issue \
    "Implement health checks (tonic-health)" \
    "## Summary
Add gRPC health checking support.

## Features
- Implement grpc.health.v1.Health service
- Report server readiness
- Support per-service health
- Integrate with Kubernetes probes

## Tasks
- [ ] Add tonic-health dependency
- [ ] Implement health service
- [ ] Add readiness logic
- [ ] Document Kubernetes config
- [ ] Write tests

## Acceptance Criteria
- [ ] grpc-health-probe works
- [ ] Kubernetes probes work
- [ ] Health reflects actual state" \
    "P0-critical,milestone:production" \
    "M4: Production Readiness"

create_issue \
    "Add gRPC reflection" \
    "## Summary
Enable gRPC reflection for debugging and tooling.

## Benefits
- grpcurl can discover services
- Postman/Insomnia integration
- Easier debugging

## Tasks
- [ ] Add tonic-reflection dependency
- [ ] Enable reflection service
- [ ] Test with grpcurl
- [ ] Document usage

## Acceptance Criteria
- [ ] grpcurl -describe works
- [ ] All services visible" \
    "P1-high,milestone:production" \
    "M4: Production Readiness"

create_issue \
    "Implement graceful shutdown" \
    "## Summary
Drain connections gracefully on shutdown.

## Behavior
1. Stop accepting new connections
2. Send GoAway to existing connections
3. Wait for in-flight requests to complete
4. Force close after timeout
5. Exit cleanly

## Tasks
- [ ] Implement shutdown signal handling
- [ ] Add connection draining
- [ ] Configure drain timeout
- [ ] Log shutdown progress
- [ ] Write tests

## Reference
See [IMPLEMENTATION_BLUEPRINT.md](./IMPLEMENTATION_BLUEPRINT.md) Step 5.

## Acceptance Criteria
- [ ] Clean shutdown on SIGTERM
- [ ] No dropped requests
- [ ] Timeout enforced" \
    "P0-critical,milestone:production" \
    "M4: Production Readiness"

create_issue \
    "Add connection tracking and limits" \
    "## Summary
Track and limit client connections.

## Features
- Count active connections
- Maximum connection limit
- Connection timeout
- Idle connection cleanup
- Per-node connection limiting

## Tasks
- [ ] Implement connection tracking
- [ ] Add max connection limit
- [ ] Add idle timeout
- [ ] Add metrics for connections
- [ ] Write tests

## Acceptance Criteria
- [ ] Limits enforced
- [ ] Metrics accurate
- [ ] No connection leaks" \
    "P1-high,milestone:production" \
    "M4: Production Readiness"

create_issue \
    "Performance benchmarks" \
    "## Summary
Establish performance baseline with benchmarks.

## Benchmarks
- Cache read/write throughput
- Snapshot update latency
- Watch notification latency
- Memory usage per client
- CPU usage under load

## Tasks
- [ ] Set up criterion benchmarks
- [ ] Benchmark cache operations
- [ ] Benchmark watch system
- [ ] Document results
- [ ] Add to CI

## Acceptance Criteria
- [ ] Benchmarks run in CI
- [ ] Results documented
- [ ] No regressions accepted" \
    "P1-high,milestone:production" \
    "M4: Production Readiness"

create_issue \
    "Load testing with 1000+ nodes" \
    "## Summary
Verify performance at scale.

## Scenarios
- 1000 concurrent clients
- 100 snapshots, 1000 resources each
- High update frequency (1/sec)
- Client churn (connect/disconnect)

## Metrics to Track
- p50/p95/p99 latency
- Memory usage
- CPU usage
- Error rate

## Tasks
- [ ] Create load test framework
- [ ] Write test scenarios
- [ ] Run tests and document results
- [ ] Identify bottlenecks
- [ ] Optimize if needed

## Acceptance Criteria
- [ ] Handles 1000+ clients
- [ ] < 10ms p99 cache latency
- [ ] Stable memory usage" \
    "P1-high,milestone:production" \
    "M4: Production Readiness"

echo -e "${GREEN}M4 issues created!${NC}"
echo ""

# ============================================================================
# M5: Examples & Documentation Issues
# ============================================================================
echo -e "${YELLOW}Creating M5: Examples & Documentation issues...${NC}"

create_issue \
    "Create simple-server example" \
    "## Summary
Create a minimal runnable xDS control plane example.

## Features
- Complete main.rs with all setup
- Sample Cluster and Endpoint resources
- Periodic resource updates
- Server with health checks and metrics
- Sample Envoy configuration

## Tasks
- [ ] Create examples/simple-server/
- [ ] Implement main.rs
- [ ] Add sample resources
- [ ] Add Envoy config
- [ ] Document how to run
- [ ] Test with real Envoy

## Reference
See [IMPLEMENTATION_BLUEPRINT.md](./IMPLEMENTATION_BLUEPRINT.md) Step 8.

## Acceptance Criteria
- [ ] Compiles and runs
- [ ] Works with Envoy
- [ ] Well documented" \
    "P0-critical,milestone:docs" \
    "M5: Examples & Documentation"

create_issue \
    "Create kubernetes-controller example" \
    "## Summary
Create a Kubernetes-integrated xDS controller example.

## Features
- Watch Kubernetes Service/Endpoints
- Convert to xDS Clusters/ClusterLoadAssignments
- Serve to Envoy sidecars
- Similar to Istio Pilot

## Tasks
- [ ] Create examples/kubernetes-controller/
- [ ] Implement K8s watching with kube-rs
- [ ] Convert K8s resources to xDS
- [ ] Handle resource updates
- [ ] Document deployment

## Reference
See [IMPLEMENTATION_BLUEPRINT.md](./IMPLEMENTATION_BLUEPRINT.md) Step 8.

## Acceptance Criteria
- [ ] Works in Kubernetes
- [ ] Handles dynamic updates
- [ ] Well documented" \
    "P0-critical,milestone:docs" \
    "M5: Examples & Documentation"

create_issue \
    "Write integration tests with Envoy" \
    "## Summary
Test the library with real Envoy containers.

## Test Scenarios
- Initial resource loading
- Resource updates
- Multiple resource types
- Client disconnect/reconnect
- Multiple clients

## Tasks
- [ ] Set up testcontainers for Envoy
- [ ] Write integration test suite
- [ ] Test SotW protocol
- [ ] Test Delta protocol
- [ ] Add to CI pipeline

## Reference
See [IMPLEMENTATION_BLUEPRINT.md](./IMPLEMENTATION_BLUEPRINT.md) Step 8.

## Acceptance Criteria
- [ ] Tests run in CI
- [ ] Cover main scenarios
- [ ] Use real Envoy" \
    "P0-critical,milestone:docs" \
    "M5: Examples & Documentation"

create_issue \
    "Add API documentation (rustdoc)" \
    "## Summary
Comprehensive API documentation for all public types.

## Requirements
- All public types documented
- All public functions documented
- Examples in documentation
- Module-level documentation
- Links between related items

## Tasks
- [ ] Document all public types
- [ ] Add examples to complex APIs
- [ ] Write module documentation
- [ ] Enable doc tests
- [ ] Deploy to docs.rs

## Acceptance Criteria
- [ ] No missing docs warnings
- [ ] Examples compile and run
- [ ] Published to docs.rs" \
    "P0-critical,milestone:docs" \
    "M5: Examples & Documentation"

create_issue \
    "Write getting started guide" \
    "## Summary
Create beginner-friendly documentation.

## Sections
1. Installation
2. Quick start (5 minute example)
3. Key concepts
4. Basic usage patterns
5. Configuration options
6. Troubleshooting

## Tasks
- [ ] Write guide content
- [ ] Add code examples
- [ ] Test all examples
- [ ] Add to README or docs site

## Acceptance Criteria
- [ ] Beginner can get started
- [ ] Examples work
- [ ] Common issues addressed" \
    "P1-high,milestone:docs" \
    "M5: Examples & Documentation"

create_issue \
    "Add architecture documentation" \
    "## Summary
Document the internal architecture.

## Sections
1. Overview and design goals
2. Crate structure
3. Cache design
4. Watch system
5. Protocol handlers
6. Extension points

## Tasks
- [ ] Write architecture docs
- [ ] Add diagrams
- [ ] Document design decisions
- [ ] Explain extension points

## Acceptance Criteria
- [ ] Contributors can understand codebase
- [ ] Design decisions explained
- [ ] Diagrams included" \
    "P1-high,milestone:docs" \
    "M5: Examples & Documentation"

create_issue \
    "Create migration guide from go-control-plane" \
    "## Summary
Help users migrate from go-control-plane.

## Sections
1. Conceptual mapping
2. API differences
3. Type equivalents
4. Example migrations
5. Known differences

## Tasks
- [ ] Document API mapping
- [ ] Show migration examples
- [ ] Note behavioral differences
- [ ] Address common patterns

## Acceptance Criteria
- [ ] Go users can migrate
- [ ] Key differences documented
- [ ] Examples provided" \
    "P2-medium,milestone:docs" \
    "M5: Examples & Documentation"

echo -e "${GREEN}M5 issues created!${NC}"
echo ""

# ============================================================================
# M6: Release Issues
# ============================================================================
echo -e "${YELLOW}Creating M6: Release issues...${NC}"

create_issue \
    "Final security audit" \
    "## Summary
Complete security review before release.

## Checklist
- [ ] cargo-audit clean
- [ ] cargo-deny clean
- [ ] No unsafe code (or justified)
- [ ] Input validation reviewed
- [ ] Error messages don't leak sensitive info
- [ ] Dependencies reviewed

## Tasks
- [ ] Run all security tools
- [ ] Review audit results
- [ ] Fix any issues
- [ ] Document security considerations

## Acceptance Criteria
- [ ] No known vulnerabilities
- [ ] All issues addressed
- [ ] Security documented" \
    "P0-critical,security,milestone:release" \
    "M6: Release"

create_issue \
    "Version all crates at 0.1.0" \
    "## Summary
Set initial version for all crates.

## Crates
- xds-core: 0.1.0
- xds-cache: 0.1.0
- xds-server: 0.1.0
- xds-types: 0.1.0
- nebucloud-xds: 0.1.0

## Tasks
- [ ] Update all Cargo.toml versions
- [ ] Update inter-crate dependencies
- [ ] Verify build succeeds
- [ ] Tag release

## Acceptance Criteria
- [ ] All versions 0.1.0
- [ ] Dependencies correct
- [ ] Build succeeds" \
    "P0-critical,milestone:release" \
    "M6: Release"

create_issue \
    "Write CHANGELOG.md" \
    "## Summary
Create changelog for initial release.

## Sections
- Added: New features
- Changed: Breaking changes
- Fixed: Bug fixes
- Security: Security fixes

## Tasks
- [ ] Generate changelog with git-cliff
- [ ] Review and edit
- [ ] Highlight key features
- [ ] Note migration info

## Acceptance Criteria
- [ ] Changelog complete
- [ ] Key features highlighted
- [ ] Breaking changes noted" \
    "P0-critical,milestone:release" \
    "M6: Release"

create_issue \
    "Publish to crates.io" \
    "## Summary
Publish all crates to crates.io.

## Publish Order (dependency order)
1. xds-core
2. xds-types
3. xds-cache
4. xds-server
5. nebucloud-xds

## Tasks
- [ ] Verify crates.io credentials
- [ ] Dry-run publish
- [ ] Publish in order
- [ ] Verify on crates.io

## Acceptance Criteria
- [ ] All crates published
- [ ] Documentation visible
- [ ] Can be installed" \
    "P0-critical,milestone:release" \
    "M6: Release"

create_issue \
    "Create GitHub release with binaries" \
    "## Summary
Create GitHub release with artifacts.

## Artifacts
- Source tarball
- Changelog excerpt
- Documentation link
- Example binaries (optional)

## Tasks
- [ ] Create release tag
- [ ] Write release notes
- [ ] Attach artifacts
- [ ] Link to crates.io

## Acceptance Criteria
- [ ] Release created
- [ ] Notes complete
- [ ] Artifacts attached" \
    "P1-high,milestone:release" \
    "M6: Release"

create_issue \
    "Announce on Rust community channels" \
    "## Summary
Share the release with the Rust community.

## Channels
- r/rust
- Rust Users Forum
- This Week in Rust
- Twitter/X

## Tasks
- [ ] Write announcement
- [ ] Post to Reddit
- [ ] Submit to TWiR
- [ ] Share on social media

## Acceptance Criteria
- [ ] Announcement posted
- [ ] Feedback addressed" \
    "P2-medium,milestone:release" \
    "M6: Release"

echo -e "${GREEN}M6 issues created!${NC}"
echo ""

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}All issues created successfully!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "Next steps:"
echo "1. Review issues at https://github.com/$REPO/issues"
echo "2. Assign issues to team members"
echo "3. Start with M1: Foundation issues"
echo ""
