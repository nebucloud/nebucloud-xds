//! Error types for xDS operations.
//!
//! This module provides [`XdsError`], a comprehensive error type that covers
//! all failure modes in xDS operations and properly converts to gRPC status codes.

/// Comprehensive error type for xDS operations.
///
/// This error type is designed to:
/// - Cover all failure modes without using panics
/// - Properly convert to [`tonic::Status`] for gRPC responses
/// - Provide detailed error messages for debugging
/// - Support error chaining via the `source` field
///
/// # Example
///
/// ```rust
/// use xds_core::XdsError;
///
/// fn validate_resource(name: &str) -> Result<(), XdsError> {
///     if name.is_empty() {
///         return Err(XdsError::InvalidResource {
///             type_url: "type.googleapis.com/envoy.config.cluster.v3.Cluster".to_string(),
///             name: name.to_string(),
///             reason: "resource name cannot be empty".to_string(),
///         });
///     }
///     Ok(())
/// }
/// ```
#[derive(Debug, thiserror::Error)]
pub enum XdsError {
    /// Malformed or unknown type URL.
    #[error("invalid type URL: {type_url} - {reason}")]
    InvalidTypeUrl {
        /// The invalid type URL.
        type_url: String,
        /// Reason why the type URL is invalid.
        reason: String,
    },

    /// Requested resource doesn't exist in the cache.
    #[error("resource not found: {type_url}/{name}")]
    ResourceNotFound {
        /// The type URL of the resource.
        type_url: String,
        /// The name of the resource.
        name: String,
    },

    /// Version conflict during update.
    #[error("version mismatch for {type_url}/{name}: expected {expected}, got {actual}")]
    VersionMismatch {
        /// The type URL of the resource.
        type_url: String,
        /// The name of the resource.
        name: String,
        /// Expected version.
        expected: String,
        /// Actual version received.
        actual: String,
    },

    /// Resource validation failed.
    #[error("invalid resource {type_url}/{name}: {reason}")]
    InvalidResource {
        /// The type URL of the resource.
        type_url: String,
        /// The name of the resource.
        name: String,
        /// Reason for validation failure.
        reason: String,
    },

    /// Cache operation failed.
    #[error("cache error: {message}")]
    CacheError {
        /// Description of the cache error.
        message: String,
        /// Optional underlying error.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Watch creation or notification failed.
    #[error("watch error: {message}")]
    WatchError {
        /// Description of the watch error.
        message: String,
    },

    /// Snapshot is incomplete - missing required resource types.
    #[error("snapshot incomplete: missing {missing_types:?}")]
    SnapshotIncomplete {
        /// List of missing type URLs.
        missing_types: Vec<String>,
    },

    /// Protobuf encoding failed.
    #[error("encoding error for {type_url}: {message}")]
    EncodingError {
        /// The type URL being encoded.
        type_url: String,
        /// Error message.
        message: String,
    },

    /// Protobuf decoding failed.
    #[error("decoding error for {type_url}: {message}")]
    DecodingError {
        /// The type URL being decoded.
        type_url: String,
        /// Error message.
        message: String,
    },

    /// gRPC transport error.
    #[error("transport error: {message}")]
    TransportError {
        /// Error message.
        message: String,
        /// Optional underlying error.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Client stream closed unexpectedly.
    #[error("stream closed: {reason}")]
    StreamClosed {
        /// Reason for stream closure.
        reason: String,
    },

    /// Client rejected configuration (NACK).
    #[error("NACK received from {node_id} for {type_url}: {error_message}")]
    NackReceived {
        /// The node ID that sent the NACK.
        node_id: String,
        /// The type URL that was rejected.
        type_url: String,
        /// The nonce of the rejected response.
        nonce: String,
        /// Error message from the client.
        error_message: String,
    },

    /// Operation timed out.
    #[error("operation timed out: {operation}")]
    Timeout {
        /// Description of the operation that timed out.
        operation: String,
    },

    /// Server is shutting down.
    #[error("server is shutting down")]
    Shutdown,

    /// Too many requests (rate limited).
    #[error("rate limited: {message}")]
    RateLimited {
        /// Rate limit message.
        message: String,
    },

    /// Unexpected internal error.
    #[error("internal error: {message}")]
    Internal {
        /// Error message.
        message: String,
        /// Optional underlying error.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Watch subscription was closed.
    #[error("watch closed: watch_id={watch_id}")]
    WatchClosed {
        /// ID of the closed watch.
        watch_id: u64,
    },

    /// Configuration error.
    #[error("configuration error: {0}")]
    Configuration(String),
}

impl XdsError {
    /// Create an internal error from any error type.
    pub fn internal<E>(message: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Internal {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a cache error from any error type.
    pub fn cache<E>(message: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::CacheError {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a transport error from any error type.
    pub fn transport<E>(message: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::TransportError {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }
}

/// Convert to tonic::Status for gRPC responses.
///
/// This implementation maps each error variant to an appropriate gRPC status code.
impl From<XdsError> for tonic::Status {
    fn from(err: XdsError) -> Self {
        match &err {
            XdsError::InvalidTypeUrl { .. } | XdsError::InvalidResource { .. } => {
                tonic::Status::invalid_argument(err.to_string())
            }
            XdsError::ResourceNotFound { .. } => tonic::Status::not_found(err.to_string()),
            XdsError::VersionMismatch { .. } => tonic::Status::failed_precondition(err.to_string()),
            XdsError::CacheError { .. }
            | XdsError::WatchError { .. }
            | XdsError::SnapshotIncomplete { .. } => tonic::Status::internal(err.to_string()),
            XdsError::EncodingError { .. } | XdsError::DecodingError { .. } => {
                tonic::Status::invalid_argument(err.to_string())
            }
            XdsError::TransportError { .. } | XdsError::StreamClosed { .. } => {
                tonic::Status::unavailable(err.to_string())
            }
            XdsError::NackReceived { .. } => {
                // NACKs are informational, not necessarily errors for the server
                tonic::Status::ok(err.to_string())
            }
            XdsError::Timeout { .. } => tonic::Status::deadline_exceeded(err.to_string()),
            XdsError::Shutdown => tonic::Status::unavailable(err.to_string()),
            XdsError::RateLimited { .. } => tonic::Status::resource_exhausted(err.to_string()),
            XdsError::Internal { .. } => tonic::Status::internal(err.to_string()),
            XdsError::WatchClosed { .. } => tonic::Status::cancelled(err.to_string()),
            XdsError::Configuration(_) => tonic::Status::invalid_argument(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = XdsError::ResourceNotFound {
            type_url: "type.googleapis.com/envoy.config.cluster.v3.Cluster".to_string(),
            name: "my-cluster".to_string(),
        };
        assert!(err.to_string().contains("my-cluster"));
    }

    #[test]
    fn test_error_to_status() {
        let err = XdsError::ResourceNotFound {
            type_url: "type.googleapis.com/envoy.config.cluster.v3.Cluster".to_string(),
            name: "my-cluster".to_string(),
        };
        let status: tonic::Status = err.into();
        assert_eq!(status.code(), tonic::Code::NotFound);
    }

    #[test]
    fn test_internal_error_helper() {
        let io_err = std::io::Error::other("test error");
        let err = XdsError::internal("operation failed", io_err);
        assert!(matches!(err, XdsError::Internal { .. }));
    }
}
