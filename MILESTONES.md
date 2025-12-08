# nebucloud-xds Milestones

> Production-Ready xDS Control Plane for Kubernetes-native Service Mesh

This document tracks the milestones and progress for transforming the legacy rust-control-plane into a production-ready nebucloud-xds library.

---

## Overview

| Milestone | Target | Status | Progress |
|-----------|--------|--------|----------|
| [M1: Foundation](#m1-foundation) | Week 1-2 | 🟡 In Progress | 62.5% |
| [M2: Core Implementation](#m2-core-implementation) | Week 3-4 | 🟡 In Progress | 75% |
| [M3: Protocol Handlers](#m3-protocol-handlers) | Week 5-6 | 🔴 Not Started | 0% |
| [M4: Production Readiness](#m4-production-readiness) | Week 7-8 | 🔴 Not Started | 0% |
| [M5: Examples & Documentation](#m5-examples--documentation) | Week 9-10 | 🔴 Not Started | 0% |
| [M6: Release](#m6-release) | Week 11-12 | 🔴 Not Started | 0% |

**Legend:** 🔴 Not Started | 🟡 In Progress | 🟢 Complete

---

## M1: Foundation

**Goal:** Establish the workspace structure, update proto submodules, and set up CI/CD infrastructure.

**Duration:** Week 1-2

### Issues

| # | Issue | Priority | Estimate | Assignee | Status |
|---|-------|----------|----------|----------|--------|
| 1 | Create 5-crate workspace structure | P0 | 2h | - | 🟢 |
| 2 | Update proto submodules (526 commits behind) | P0 | 4h | - | 🔴 |
| 3 | Set up cargo-deny and security auditing | P0 | 2h | - | 🟢 |
| 4 | Configure CI pipeline (GitHub Actions) | P0 | 4h | - | 🟢 |
| 5 | Set up proto-sync automation | P1 | 2h | - | 🔴 |
| 6 | Configure Dependabot | P1 | 1h | - | 🟢 |
| 7 | Add rust-toolchain.toml (MSRV 1.75) | P1 | 30m | - | 🟢 |
| 8 | Set up Changesets for versioning | P1 | 2h | - | 🔴 |

**Exit Criteria:**
- [x] All 5 crates created with proper Cargo.toml
- [ ] Proto submodules updated to latest
- [x] CI passing on `main` branch
- [x] cargo-deny passing

---

## M2: Core Implementation

**Goal:** Implement the core types, error handling, and cache system with production-grade patterns.

**Duration:** Week 3-4

### Issues

| # | Issue | Priority | Estimate | Assignee | Status |
|---|-------|----------|----------|----------|--------|
| 9 | Implement XdsError with 16 error variants | P0 | 4h | - | 🟢 |
| 10 | Add ResourceVersion and NodeHash types | P0 | 2h | - | 🟢 |
| 11 | Create generic Resource trait | P0 | 4h | - | 🟢 |
| 12 | Implement ResourceRegistry | P0 | 2h | - | 🔴 |
| 13 | Build ShardedCache with DashMap | P0 | 8h | - | 🟢 |
| 14 | Implement Watch and WatchManager | P0 | 4h | - | 🟢 |
| 15 | Add Snapshot builder pattern | P1 | 3h | - | 🟢 |
| 16 | Write unit tests for cache | P0 | 4h | - | 🔴 |

**Exit Criteria:**
- [ ] No panics in error paths (remove all `.unwrap()` in production code)
- [ ] DashMap-based cache passing stress tests
- [ ] Watch notification system working
- [ ] 80%+ test coverage for xds-core and xds-cache

**Critical Fix:** Lock-across-await bug must be resolved (drop DashMap refs before any `.await`)

---

## M3: Protocol Handlers

**Goal:** Implement SotW and Delta xDS protocol handlers with proper stream management.

**Duration:** Week 5-6

### Issues

| # | Issue | Priority | Estimate | Assignee | Status |
|---|-------|----------|----------|----------|--------|
| 17 | Implement SotwStreamHandler | P0 | 8h | - | 🔴 |
| 18 | Implement DeltaStreamHandler | P0 | 8h | - | 🔴 |
| 19 | Build AggregatedDiscoveryService (ADS) | P0 | 4h | - | 🔴 |
| 20 | Add CDS, EDS, LDS, RDS, SDS services | P1 | 6h | - | 🔴 |
| 21 | Implement proper NACK handling | P0 | 4h | - | 🔴 |
| 22 | Add resource subscription tracking | P1 | 4h | - | 🔴 |
| 23 | Write protocol compliance tests | P0 | 6h | - | 🔴 |

**Exit Criteria:**
- [ ] SotW protocol working with Envoy
- [ ] Delta protocol working with Envoy
- [ ] NACK/ACK handling correct
- [ ] All xDS resource types supported

---

## M4: Production Readiness

**Goal:** Add observability, health checks, graceful shutdown, and production hardening.

**Duration:** Week 7-8

### Issues

| # | Issue | Priority | Estimate | Assignee | Status |
|---|-------|----------|----------|----------|--------|
| 24 | Implement XdsServerBuilder | P0 | 4h | - | 🔴 |
| 25 | Add Prometheus metrics (XdsMetrics) | P0 | 4h | - | 🔴 |
| 26 | Implement health checks (tonic-health) | P0 | 2h | - | 🔴 |
| 27 | Add gRPC reflection | P1 | 1h | - | 🔴 |
| 28 | Implement graceful shutdown | P0 | 3h | - | 🔴 |
| 29 | Add connection tracking and limits | P1 | 4h | - | 🔴 |
| 30 | Performance benchmarks | P1 | 4h | - | 🔴 |
| 31 | Load testing with 1000+ nodes | P1 | 4h | - | 🔴 |

**Exit Criteria:**
- [ ] Metrics exported to Prometheus
- [ ] Health endpoints working
- [ ] Graceful shutdown draining connections
- [ ] < 10ms p99 latency for cache operations

---

## M5: Examples & Documentation

**Goal:** Create comprehensive examples and documentation for users.

**Duration:** Week 9-10

### Issues

| # | Issue | Priority | Estimate | Assignee | Status |
|---|-------|----------|----------|----------|--------|
| 32 | Create simple-server example | P0 | 4h | - | 🔴 |
| 33 | Create kubernetes-controller example | P0 | 8h | - | 🔴 |
| 34 | Write integration tests with Envoy | P0 | 6h | - | 🔴 |
| 35 | Add API documentation (rustdoc) | P0 | 4h | - | 🔴 |
| 36 | Write getting started guide | P1 | 3h | - | 🔴 |
| 37 | Add architecture documentation | P1 | 2h | - | 🔴 |
| 38 | Create migration guide from go-control-plane | P2 | 4h | - | 🔴 |

**Exit Criteria:**
- [ ] Examples compile and run
- [ ] Integration tests passing with real Envoy
- [ ] Documentation published to docs.rs
- [ ] README with quickstart

---

## M6: Release

**Goal:** Prepare and publish v0.1.0 to crates.io.

**Duration:** Week 11-12

### Issues

| # | Issue | Priority | Estimate | Assignee | Status |
|---|-------|----------|----------|----------|--------|
| 39 | Final security audit | P0 | 4h | - | 🔴 |
| 40 | Version all crates at 0.1.0 | P0 | 1h | - | 🔴 |
| 41 | Write CHANGELOG.md | P0 | 2h | - | 🔴 |
| 42 | Publish to crates.io | P0 | 2h | - | 🔴 |
| 43 | Create GitHub release with binaries | P1 | 2h | - | 🔴 |
| 44 | Announce on Rust community channels | P2 | 1h | - | 🔴 |

**Exit Criteria:**
- [ ] All crates published to crates.io
- [ ] GitHub release created
- [ ] No P0 issues open
- [ ] Security audit clean

---

## Issue Templates

### Bug Report Template
```markdown
## Description
Brief description of the bug.

## Steps to Reproduce
1. Step one
2. Step two
3. ...

## Expected Behavior
What should happen.

## Actual Behavior
What actually happens.

## Environment
- Rust version:
- OS:
- nebucloud-xds version:
```

### Feature Request Template
```markdown
## Summary
Brief description of the feature.

## Motivation
Why is this feature needed?

## Proposed Solution
Describe the solution.

## Alternatives Considered
Other approaches that were considered.

## Additional Context
Any other relevant information.
```

---

## Labels

| Label | Color | Description |
|-------|-------|-------------|
| `P0-critical` | `#b60205` | Must fix before release |
| `P1-high` | `#d93f0b` | Should fix before release |
| `P2-medium` | `#fbca04` | Nice to have |
| `P3-low` | `#0e8a16` | Future consideration |
| `bug` | `#d73a4a` | Something isn't working |
| `enhancement` | `#a2eeef` | New feature or request |
| `documentation` | `#0075ca` | Improvements to docs |
| `security` | `#7057ff` | Security related |
| `breaking-change` | `#b60205` | Breaking API change |
| `good-first-issue` | `#7057ff` | Good for newcomers |
| `help-wanted` | `#008672` | Extra attention needed |

---

## Tracking

### Weekly Progress

| Week | Focus | Completed | Notes |
|------|-------|-----------|-------|
| 1 | Foundation setup | - | - |
| 2 | Proto sync, CI | - | - |
| 3 | Core types | - | - |
| 4 | Cache implementation | - | - |
| 5 | SotW protocol | - | - |
| 6 | Delta protocol | - | - |
| 7 | Server builder | - | - |
| 8 | Metrics, health | - | - |
| 9 | Examples | - | - |
| 10 | Integration tests | - | - |
| 11 | Documentation | - | - |
| 12 | Release | - | - |

---

## References

- [Implementation Blueprint](./IMPLEMENTATION_BLUEPRINT.md) - Detailed code for each component
- [Analysis & Redesign](./ANALYSIS_AND_REDESIGN.md) - Production readiness analysis
- [go-control-plane](https://github.com/envoyproxy/go-control-plane) - Reference implementation
- [xDS Protocol Spec](https://www.envoyproxy.io/docs/envoy/latest/api-docs/xds_protocol)
