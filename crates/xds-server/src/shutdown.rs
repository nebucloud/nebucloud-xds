//! Graceful shutdown handling for the xDS server.
//!
//! This module provides signal handling and graceful shutdown coordination
//! for the xDS server, ensuring in-flight requests complete before termination.
//!
//! # Example
//!
//! ```rust,ignore
//! use xds_server::shutdown::ShutdownController;
//! use std::time::Duration;
//!
//! let controller = ShutdownController::new();
//!
//! // In your server startup
//! let shutdown_rx = controller.subscribe();
//!
//! // When shutdown is triggered
//! controller.shutdown(Duration::from_secs(30)).await;
//! ```

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;
use tokio::time::timeout;
use tracing::{info, warn};

/// Controller for coordinating graceful shutdown.
///
/// Provides mechanisms to:
/// - Subscribe to shutdown signals
/// - Trigger shutdown programmatically
/// - Wait for in-flight operations to complete
/// - Set shutdown timeout
#[derive(Debug, Clone)]
pub struct ShutdownController {
    inner: Arc<ShutdownInner>,
}

#[derive(Debug)]
struct ShutdownInner {
    /// Whether shutdown has been initiated.
    initiated: AtomicBool,
    /// Sender for shutdown signal.
    tx: watch::Sender<bool>,
    /// Receiver for shutdown signal.
    rx: watch::Receiver<bool>,
    /// Active operation counter.
    active_ops: AtomicCounter,
}

/// Atomic counter for tracking active operations.
#[derive(Debug, Default)]
struct AtomicCounter {
    count: std::sync::atomic::AtomicUsize,
}

impl AtomicCounter {
    fn increment(&self) -> usize {
        self.count.fetch_add(1, Ordering::SeqCst) + 1
    }

    fn decrement(&self) -> usize {
        self.count.fetch_sub(1, Ordering::SeqCst) - 1
    }

    fn get(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }
}

impl Default for ShutdownController {
    fn default() -> Self {
        Self::new()
    }
}

impl ShutdownController {
    /// Create a new shutdown controller.
    pub fn new() -> Self {
        let (tx, rx) = watch::channel(false);
        Self {
            inner: Arc::new(ShutdownInner {
                initiated: AtomicBool::new(false),
                tx,
                rx,
                active_ops: AtomicCounter::default(),
            }),
        }
    }

    /// Subscribe to shutdown notifications.
    ///
    /// Returns a receiver that will be notified when shutdown is initiated.
    pub fn subscribe(&self) -> watch::Receiver<bool> {
        self.inner.rx.clone()
    }

    /// Check if shutdown has been initiated.
    pub fn is_shutdown(&self) -> bool {
        self.inner.initiated.load(Ordering::SeqCst)
    }

    /// Get a future that resolves when shutdown is initiated.
    pub fn shutdown_signal(&self) -> ShutdownSignal {
        ShutdownSignal {
            rx: self.inner.rx.clone(),
        }
    }

    /// Initiate graceful shutdown.
    ///
    /// This will:
    /// 1. Set the shutdown flag
    /// 2. Notify all subscribers
    /// 3. Wait for active operations to complete (with timeout)
    ///
    /// Returns `true` if all operations completed gracefully, `false` if timed out.
    pub async fn shutdown(&self, grace_period: Duration) -> bool {
        // Mark as initiated
        if self
            .inner
            .initiated
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            // Already initiated
            return true;
        }

        info!("initiating graceful shutdown with {:?} grace period", grace_period);

        // Notify all subscribers
        let _ = self.inner.tx.send(true);

        // Wait for active operations to complete
        let result = timeout(grace_period, self.wait_for_completion()).await;

        match result {
            Ok(()) => {
                info!("graceful shutdown completed");
                true
            }
            Err(_) => {
                let remaining = self.inner.active_ops.get();
                warn!(
                    remaining_ops = remaining,
                    "graceful shutdown timed out, forcing shutdown"
                );
                false
            }
        }
    }

    /// Wait for all active operations to complete.
    async fn wait_for_completion(&self) {
        loop {
            if self.inner.active_ops.get() == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Register an active operation.
    ///
    /// Returns a guard that decrements the counter when dropped.
    pub fn register_operation(&self) -> OperationGuard {
        self.inner.active_ops.increment();
        OperationGuard {
            controller: self.clone(),
        }
    }

    /// Get the number of active operations.
    pub fn active_operations(&self) -> usize {
        self.inner.active_ops.get()
    }
}

/// Guard for tracking an active operation.
///
/// Decrements the active operation counter when dropped.
#[derive(Debug)]
pub struct OperationGuard {
    controller: ShutdownController,
}

impl Drop for OperationGuard {
    fn drop(&mut self) {
        self.controller.inner.active_ops.decrement();
    }
}

/// Future that resolves when shutdown is initiated.
#[derive(Debug, Clone)]
pub struct ShutdownSignal {
    rx: watch::Receiver<bool>,
}

impl ShutdownSignal {
    /// Wait for the shutdown signal.
    pub async fn wait(mut self) {
        loop {
            if *self.rx.borrow() {
                return;
            }
            if self.rx.changed().await.is_err() {
                // Channel closed, treat as shutdown
                return;
            }
            if *self.rx.borrow() {
                return;
            }
        }
    }
}

/// Wait for OS shutdown signals (SIGTERM, SIGINT).
///
/// This function returns when either signal is received.
pub async fn wait_for_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigterm = signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("failed to install SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {
                info!("received SIGTERM");
            }
            _ = sigint.recv() => {
                info!("received SIGINT");
            }
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
        info!("received Ctrl+C");
    }
}

/// Shutdown configuration.
#[derive(Debug, Clone)]
pub struct ShutdownConfig {
    /// Grace period for shutdown.
    pub grace_period: Duration,
    /// Whether to listen for OS signals.
    pub listen_for_signals: bool,
}

impl Default for ShutdownConfig {
    fn default() -> Self {
        Self {
            grace_period: Duration::from_secs(30),
            listen_for_signals: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shutdown_controller_creation() {
        let controller = ShutdownController::new();
        assert!(!controller.is_shutdown());
        assert_eq!(controller.active_operations(), 0);
    }

    #[test]
    fn operation_tracking() {
        let controller = ShutdownController::new();

        {
            let _guard1 = controller.register_operation();
            assert_eq!(controller.active_operations(), 1);

            let _guard2 = controller.register_operation();
            assert_eq!(controller.active_operations(), 2);
        }

        assert_eq!(controller.active_operations(), 0);
    }

    #[tokio::test]
    async fn shutdown_signal() {
        let controller = ShutdownController::new();
        let mut rx = controller.subscribe();

        // Trigger shutdown in background
        let controller_clone = controller.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            controller_clone.shutdown(Duration::from_millis(100)).await;
        });

        // Wait for signal
        rx.changed().await.expect("should receive shutdown signal");
        assert!(controller.is_shutdown());
    }

    #[tokio::test]
    async fn shutdown_waits_for_operations() {
        let controller = ShutdownController::new();

        // Register an operation
        let guard = controller.register_operation();
        assert_eq!(controller.active_operations(), 1);

        // Start shutdown in background
        let controller_clone = controller.clone();
        let handle = tokio::spawn(async move {
            controller_clone.shutdown(Duration::from_secs(5)).await
        });

        // Wait a bit then drop the guard
        tokio::time::sleep(Duration::from_millis(100)).await;
        drop(guard);

        // Shutdown should complete
        let result = handle.await.expect("shutdown task should complete");
        assert!(result);
    }

    #[test]
    fn shutdown_config_defaults() {
        let config = ShutdownConfig::default();
        assert_eq!(config.grace_period, Duration::from_secs(30));
        assert!(config.listen_for_signals);
    }
}
