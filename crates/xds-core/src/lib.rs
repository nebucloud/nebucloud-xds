//! # xds-core
//!
//! Core types, traits, and error handling for xDS control plane implementations.
//!
//! This crate provides the foundational types used across all other xDS crates:
//!
//! - [`XdsError`] - Comprehensive error type with proper gRPC status code mapping
//! - [`ResourceVersion`] - Version tracking for xDS resources
//! - [`NodeHash`] - Efficient node identification using FNV-1a hashing
//! - [`Resource`] - Trait for implementing custom xDS resource types
//! - [`TypeUrl`] - Type URL handling and constants
//!
//! ## Example
//!
//! ```rust
//! use xds_core::{XdsError, NodeHash, ResourceVersion};
//!
//! // Create a node hash from a node ID
//! let node = NodeHash::from_id("my-envoy-node");
//!
//! // Create a resource version
//! let version = ResourceVersion::new("v1");
//!
//! // Check if version is empty (initial state)
//! assert!(!version.is_empty());
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(unsafe_code)]
#![warn(missing_docs)]

mod error;
mod node;
mod resource;
mod type_url;
mod version;

pub use error::XdsError;
pub use node::NodeHash;
pub use resource::{
    BoxResource, Resource, ResourceRegistry, ResourceTypeInfo, SharedResourceRegistry,
};
pub use type_url::TypeUrl;
pub use version::ResourceVersion;

/// Result type alias using [`XdsError`].
pub type Result<T> = std::result::Result<T, XdsError>;

/// Alias for Result to maintain backward compatibility.
pub type XdsResult<T> = Result<T>;
