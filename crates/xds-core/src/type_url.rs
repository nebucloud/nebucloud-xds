//! Type URL handling for xDS resources.
//!
//! This module provides type URL constants and utilities for
//! working with xDS resource type URLs.

use std::fmt;

/// Type URL wrapper for xDS resource types.
///
/// Type URLs identify the protobuf message type of xDS resources.
/// This type provides validation and comparison operations.
///
/// # Example
///
/// ```rust
/// use xds_core::TypeUrl;
///
/// let cluster_type = TypeUrl::new("type.googleapis.com/envoy.config.cluster.v3.Cluster");
/// assert_eq!(cluster_type.short_name(), "Cluster");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeUrl(String);

impl TypeUrl {
    /// Type URL for Cluster (CDS).
    pub const CLUSTER: &'static str = "type.googleapis.com/envoy.config.cluster.v3.Cluster";

    /// Type URL for ClusterLoadAssignment (EDS).
    pub const ENDPOINT: &'static str =
        "type.googleapis.com/envoy.config.endpoint.v3.ClusterLoadAssignment";

    /// Type URL for Listener (LDS).
    pub const LISTENER: &'static str = "type.googleapis.com/envoy.config.listener.v3.Listener";

    /// Type URL for RouteConfiguration (RDS).
    pub const ROUTE: &'static str =
        "type.googleapis.com/envoy.config.route.v3.RouteConfiguration";

    /// Type URL for Secret (SDS).
    pub const SECRET: &'static str =
        "type.googleapis.com/envoy.extensions.transport_sockets.tls.v3.Secret";

    /// Type URL for Runtime (RTDS).
    pub const RUNTIME: &'static str = "type.googleapis.com/envoy.service.runtime.v3.Runtime";

    /// Type URL for ScopedRouteConfiguration.
    pub const SCOPED_ROUTE: &'static str =
        "type.googleapis.com/envoy.config.route.v3.ScopedRouteConfiguration";

    /// Type URL for VirtualHost (VHDS).
    pub const VIRTUAL_HOST: &'static str =
        "type.googleapis.com/envoy.config.route.v3.VirtualHost";

    /// Type URL for ExtensionConfig (ECDS).
    pub const EXTENSION_CONFIG: &'static str =
        "type.googleapis.com/envoy.config.core.v3.TypedExtensionConfig";

    /// Create a new type URL from a string.
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self(url.into())
    }

    /// Get the type URL as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Extract the short name from the type URL.
    ///
    /// For example, `type.googleapis.com/envoy.config.cluster.v3.Cluster`
    /// returns `Cluster`.
    #[must_use]
    pub fn short_name(&self) -> &str {
        self.0.rsplit('/').next().and_then(|s| s.rsplit('.').next()).unwrap_or(&self.0)
    }

    /// Check if this is a valid xDS type URL.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.0.starts_with("type.googleapis.com/")
    }

    /// Consume and return the inner string.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for TypeUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for TypeUrl {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for TypeUrl {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<TypeUrl> for String {
    fn from(t: TypeUrl) -> Self {
        t.0
    }
}

impl AsRef<str> for TypeUrl {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_url_creation() {
        let t = TypeUrl::new(TypeUrl::CLUSTER);
        assert_eq!(t.as_str(), TypeUrl::CLUSTER);
    }

    #[test]
    fn test_short_name() {
        let t = TypeUrl::new(TypeUrl::CLUSTER);
        assert_eq!(t.short_name(), "Cluster");

        let t = TypeUrl::new(TypeUrl::ENDPOINT);
        assert_eq!(t.short_name(), "ClusterLoadAssignment");
    }

    #[test]
    fn test_is_valid() {
        let valid = TypeUrl::new(TypeUrl::CLUSTER);
        assert!(valid.is_valid());

        let invalid = TypeUrl::new("invalid-url");
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_from_string() {
        let t: TypeUrl = TypeUrl::CLUSTER.into();
        assert_eq!(t.as_str(), TypeUrl::CLUSTER);
    }
}
