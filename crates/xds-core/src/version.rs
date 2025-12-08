//! Resource version tracking for xDS.
//!
//! This module provides [`ResourceVersion`], a type for tracking versions
//! of xDS resources. Versions are used to detect changes and avoid
//! sending unchanged resources.

use std::fmt;

/// Version identifier for xDS resources.
///
/// `ResourceVersion` wraps a version string and provides ordering and
/// comparison operations. An empty version represents the initial state
/// (no version received yet).
///
/// # Example
///
/// ```rust
/// use xds_core::ResourceVersion;
///
/// let v1 = ResourceVersion::new("v1");
/// let v2 = ResourceVersion::new("v2");
/// let empty = ResourceVersion::empty();
///
/// assert!(!v1.is_empty());
/// assert!(empty.is_empty());
/// assert_ne!(v1, v2);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ResourceVersion(String);

impl ResourceVersion {
    /// Create a new resource version from a string.
    #[must_use]
    pub fn new(version: impl Into<String>) -> Self {
        Self(version.into())
    }

    /// Create an empty resource version (initial state).
    #[must_use]
    pub fn empty() -> Self {
        Self(String::new())
    }

    /// Check if the version is empty (initial state).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Get the version as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume and return the inner string.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for ResourceVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ResourceVersion {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ResourceVersion {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<ResourceVersion> for String {
    fn from(v: ResourceVersion) -> Self {
        v.0
    }
}

impl AsRef<str> for ResourceVersion {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_creation() {
        let v = ResourceVersion::new("v1");
        assert_eq!(v.as_str(), "v1");
        assert!(!v.is_empty());
    }

    #[test]
    fn test_empty_version() {
        let v = ResourceVersion::empty();
        assert!(v.is_empty());
        assert_eq!(v.as_str(), "");
    }

    #[test]
    fn test_version_equality() {
        let v1 = ResourceVersion::new("v1");
        let v1_copy = ResourceVersion::new("v1");
        let v2 = ResourceVersion::new("v2");

        assert_eq!(v1, v1_copy);
        assert_ne!(v1, v2);
    }

    #[test]
    fn test_version_from_string() {
        let v: ResourceVersion = "v1".into();
        assert_eq!(v.as_str(), "v1");

        let v: ResourceVersion = String::from("v2").into();
        assert_eq!(v.as_str(), "v2");
    }

    #[test]
    fn test_version_display() {
        let v = ResourceVersion::new("v1.2.3");
        assert_eq!(format!("{v}"), "v1.2.3");
    }
}
