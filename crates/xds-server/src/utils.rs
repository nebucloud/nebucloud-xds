//! Shared utilities for xds-server.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Global counter for generating unique nonces.
static NONCE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generate a unique nonce for xDS responses.
///
/// Nonces are used to correlate requests and responses in xDS protocol.
/// They combine a timestamp with an atomic counter to ensure uniqueness
/// even under high concurrency.
///
/// # Format
///
/// The nonce format is `{timestamp_hex}-{counter_hex}` for SotW protocol
/// or `d{timestamp_hex}-{counter_hex}` for Delta protocol.
///
/// # Example
///
/// ```ignore
/// let nonce = generate_nonce(NoncePrefix::SotW);
/// // e.g., "18c5a3b2f1-0"
///
/// let delta_nonce = generate_nonce(NoncePrefix::Delta);
/// // e.g., "d18c5a3b2f1-1"
/// ```
pub fn generate_nonce(prefix: NoncePrefix) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;

    let count = NONCE_COUNTER.fetch_add(1, Ordering::Relaxed);

    match prefix {
        NoncePrefix::SotW => format!("{:x}-{:x}", timestamp, count),
        NoncePrefix::Delta => format!("d{:x}-{:x}", timestamp, count),
    }
}

/// Prefix for nonce generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoncePrefix {
    /// State-of-the-World protocol (no prefix).
    SotW,
    /// Delta protocol (prefixed with 'd').
    Delta,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonce_unique() {
        let n1 = generate_nonce(NoncePrefix::SotW);
        let n2 = generate_nonce(NoncePrefix::SotW);
        assert_ne!(n1, n2, "nonces should be unique");
    }

    #[test]
    fn nonce_format_sotw() {
        let nonce = generate_nonce(NoncePrefix::SotW);
        assert!(nonce.contains('-'), "nonce should contain separator");
        assert!(!nonce.starts_with('d'), "SotW nonce should not start with 'd'");
    }

    #[test]
    fn nonce_format_delta() {
        let nonce = generate_nonce(NoncePrefix::Delta);
        assert!(nonce.starts_with('d'), "Delta nonce should start with 'd'");
        assert!(nonce.contains('-'), "nonce should contain separator");
    }
}
