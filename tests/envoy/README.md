# Envoy Integration Tests

This directory contains integration tests that verify xDS protocol compliance with a real Envoy proxy.

## Prerequisites

- Docker or Docker Desktop
- Port 18000 available (for xDS server)
- Port 9901 available (for Envoy admin)
- Port 10000 available (for Envoy listener)

## Running Tests

### 1. Start Envoy

```bash
docker compose -f tests/envoy/docker-compose.yaml up -d
```

### 2. Run the Tests

```bash
# Run all Envoy integration tests
cargo test --package envoy-tests -- --ignored --nocapture
```

### 3. Cleanup

```bash
docker compose -f tests/envoy/docker-compose.yaml down
```

## Test Scenarios

| Test | Description |
|------|-------------|
| `test_envoy_connects_to_xds` | Verifies Envoy connects to the xDS server |
| `test_envoy_receives_clusters` | Tests CDS (Cluster Discovery Service) |
| `test_envoy_receives_listeners` | Tests LDS (Listener Discovery Service) |
| `test_envoy_receives_updates` | Tests snapshot update propagation |
| `test_multiple_node_support` | Tests node-specific configurations |

## Architecture

```
┌─────────────────┐     gRPC/xDS      ┌─────────────────┐
│   Test Harness  │◄─────────────────►│  Envoy Proxy    │
│  (xDS Server)   │                   │  (Container)    │
│  Port 18000     │                   │  Port 9901/10000│
└─────────────────┘                   └─────────────────┘
        │                                     │
        ▼                                     ▼
   ShardedCache                         Admin API
   with Snapshots                       /clusters
                                        /listeners
                                        /config_dump
```

## Files

- `docker-compose.yaml` - Docker Compose configuration for Envoy
- `envoy.yaml` - Envoy bootstrap configuration (for local development)
- `src/harness.rs` - Test harness with xDS server setup
- `src/tests.rs` - Integration test cases

## Debugging

### View Envoy Admin

Open http://localhost:9901 in your browser to see:
- `/clusters` - Configured clusters
- `/listeners` - Configured listeners
- `/config_dump` - Full configuration
- `/stats` - Metrics

### View Envoy Logs

```bash
docker logs -f envoy-xds-test
```

### Test xDS Connection Manually

```bash
# Check if Envoy is connected to xDS
curl http://localhost:9901/config_dump | jq '.configs[] | select(.["@type"] | contains("DynamicCluster"))'
```

## CI Integration

These tests run automatically in GitHub Actions via `.github/workflows/envoy.yml`.
The workflow:
1. Starts Envoy as a service container
2. Builds the test package
3. Runs the integration tests with `--ignored` flag
