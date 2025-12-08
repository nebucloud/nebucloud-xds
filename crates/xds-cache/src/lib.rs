//! # xds-cache
//!
//! High-performance snapshot cache for xDS resources.
//!
//! This crate provides the caching layer for xDS control plane implementations:
//!
//! - [`ShardedCache`] - DashMap-based concurrent cache for snapshots
//! - [`Snapshot`] - Immutable collection of resources for a node
//! - [`Watch`] - Subscription system for cache updates
//!
//! ## Key Design Decisions
//!
//! - Uses `DashMap` for lock-free concurrent access
//! - All `DashMap` references are dropped before any `.await` to prevent deadlocks
//! - Snapshots are immutable and atomically replaced
//! - Watch notifications are async and non-blocking
//!
//! ## Example
//!
//! ```rust,ignore
//! use xds_cache::{ShardedCache, Snapshot};
//! use xds_core::{NodeHash, TypeUrl};
//!
//! // Create a cache
//! let cache = ShardedCache::new();
//!
//! // Build a snapshot
//! let snapshot = Snapshot::builder()
//!     .version("v1")
//!     .build();
//!
//! // Set snapshot for a node
//! cache.set_snapshot(NodeHash::from_id("node-1"), snapshot).await?;
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(unsafe_code)]
#![warn(missing_docs)]

mod cache;
mod snapshot;
mod stats;
mod watch;

pub use cache::{Cache, ShardedCache};
pub use snapshot::{Snapshot, SnapshotBuilder, SnapshotResources};
pub use stats::CacheStats;
pub use watch::{Watch, WatchId, WatchManager};
