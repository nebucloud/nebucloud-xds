---
"nebucloud-xds": minor
"xds-core": minor
"xds-cache": minor
"xds-server": minor
"xds-types": minor
---

Initial release of nebucloud-xds - Production-ready xDS control plane for Rust

### Features

- **5-crate workspace architecture**: Modular design with `xds-core`, `xds-cache`, `xds-server`, `xds-types`, and `nebucloud-xds` facade
- **Type-safe resource handling**: Generic `Resource` trait with `TypeUrl` and version tracking
- **Sharded cache**: DashMap-based concurrent cache with watch notifications
- **Error handling**: 16 comprehensive error variants with `thiserror`
- **Snapshot builder**: Ergonomic API for creating resource snapshots

### Infrastructure

- CI/CD pipeline with format, clippy, tests, docs, security audit, and MSRV checks
- Proto submodule sync automation (weekly)
- Dependabot for dependency updates
- cargo-deny for security and license auditing
- Changesets for versioning and changelog generation
