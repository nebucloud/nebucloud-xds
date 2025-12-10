//! Envoy integration tests
//!
//! These tests verify xDS protocol compliance with real Envoy proxy.
//!
//! ## Prerequisites
//!
//! 1. Docker must be running
//! 2. Port 18000 must be available for the xDS server
//!
//! ## Running Tests
//!
//! ```bash
//! # Start Envoy container first
//! docker compose -f tests/envoy/docker-compose.yaml up -d
//!
//! # Run the tests
//! cargo test --package envoy-tests
//!
//! # Cleanup
//! docker compose -f tests/envoy/docker-compose.yaml down
//! ```

pub mod harness;

#[cfg(test)]
mod tests;
