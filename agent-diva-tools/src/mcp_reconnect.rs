//! MCP auto-reconnect manager.
//!
//! Provides background health checking and automatic reconnection for MCP
//! servers with exponential backoff. The health check loop runs in a
//! background tokio task and is cancellable via [`Notify`].
//!
//! # Architecture
//!
//! ```text
//! McpReconnectManager::start()
//!   │
//!   ├─ tokio::spawn ──► loop {
//!   │                     │ cancel? ──► return
//!   │                     │ health check interval (15s)
//!   │                     │ client.is_some()? ──► continue
//!   │                     │ reconnect with exponential backoff (1s…16s)
//!   │                   }
//!   │
//!   └─ JoinHandle (for lifetime management)
//!
//! McpReconnectManager::cancel()
//!   └─ notify_waiters() ──► loop exits
//! ```

use crate::mcp_sdk::{McpClientWrapper, SharedMcpClient};
use agent_diva_core::config::MCPServerConfig;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tracing::{error, info, warn};

// ---------------------------------------------------------------------------
// Constants — production defaults overridden in test cfg for fast CI
// ---------------------------------------------------------------------------

/// Initial delay before the first health check.
#[cfg(not(test))]
const INITIAL_DELAY: Duration = Duration::from_secs(15);
#[cfg(test)]
const INITIAL_DELAY: Duration = Duration::from_millis(100);

/// Interval between health check polls.
#[cfg(not(test))]
const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(15);
#[cfg(test)]
const HEALTH_CHECK_INTERVAL: Duration = Duration::from_millis(100);

/// Base milliseconds for exponential backoff (2^attempt * BASE_MS).
#[cfg(not(test))]
const BACKOFF_BASE_MS: u64 = 1000;
#[cfg(test)]
const BACKOFF_BASE_MS: u64 = 10;

/// Maximum number of reconnection attempts before giving up.
const MAX_ATTEMPTS: u32 = 5;

// ---------------------------------------------------------------------------
// McpReconnectManager
// ---------------------------------------------------------------------------

/// Manages automatic reconnection for an MCP server.
///
/// Create one per MCP server configuration, share the same
/// [`SharedMcpClient`] that tools use, and call [`start`] to begin the
/// background health check loop. Call [`cancel`] to stop the loop.
pub struct McpReconnectManager {
    /// Display name of the MCP server (used in log messages).
    server_name: String,
    /// Connection configuration (command, args, env, url, timeout).
    config: MCPServerConfig,
    /// Shared client reference updated atomically on reconnect.
    client: SharedMcpClient,
    /// Cancellation signal for the background loop.
    cancel_token: Arc<Notify>,
}

impl McpReconnectManager {
    /// Create a new reconnect manager.
    ///
    /// The manager does **not** start until [`start`] is called.
    pub fn new(
        server_name: String,
        config: MCPServerConfig,
        client: SharedMcpClient,
    ) -> Self {
        Self {
            server_name,
            config,
            client,
            cancel_token: Arc::new(Notify::new()),
        }
    }

    /// Start the background health check loop.
    ///
    /// Returns a [`JoinHandle`] that can be used to await completion or abort
    /// the task. Call [`cancel`] on the manager to signal a graceful stop.
    ///
    /// The loop:
    /// 1. Waits for the initial delay (cancelable).
    /// 2. Enters a loop: waits for the health-check interval (cancelable),
    ///    checks the client state, and reconnects with exponential backoff
    ///    if the client is `None`.
    /// 3. If all [`MAX_ATTEMPTS`] reconnect attempts fail, the loop stops
    ///    permanently.
    pub fn start(&self) -> tokio::task::JoinHandle<()> {
        let server_name = self.server_name.clone();
        let config = self.config.clone();
        let client = self.client.clone();
        let notify = self.cancel_token.clone();

        tokio::spawn(async move {
            // ── Initial delay before first health check (cancelable) ──
            tokio::select! {
                _ = notify.notified() => {
                    info!(
                        "MCP reconnect cancelled for '{}' (during initial delay)",
                        server_name
                    );
                    return;
                }
                _ = tokio::time::sleep(INITIAL_DELAY) => {}
            }

            let mut attempt: u32 = 0;

            loop {
                // ── Health check interval (cancelable) ──
                tokio::select! {
                    _ = notify.notified() => {
                        info!("MCP reconnect cancelled for '{}'", server_name);
                        return;
                    }
                    _ = tokio::time::sleep(HEALTH_CHECK_INTERVAL) => {}
                }

                // ── Check connection health ──
                let is_alive = {
                    let guard: tokio::sync::RwLockReadGuard<'_, Option<Arc<McpClientWrapper>>> =
                        client.read().await;
                    guard.is_some()
                };

                if is_alive {
                    attempt = 0; // Reset attempt counter on healthy connection
                    continue;
                }

                // ── Reconnect with exponential backoff ──
                attempt += 1;
                let exponent = attempt.min(MAX_ATTEMPTS) - 1;
                let delay = Duration::from_millis(BACKOFF_BASE_MS * 2u64.pow(exponent));

                warn!(
                    "MCP server '{}' disconnected, reconnecting in {:.1}s (attempt {}/{})",
                    server_name,
                    delay.as_secs_f64(),
                    attempt,
                    MAX_ATTEMPTS
                );

                tokio::time::sleep(delay).await;

                match McpClientWrapper::new_stdio(&server_name, &config).await {
                    Ok(new_client) => {
                        let mut guard = client.write().await;
                        *guard = Some(Arc::new(new_client));
                        info!(
                            "MCP server '{}' reconnected successfully",
                            server_name
                        );
                        attempt = 0;
                    }
                    Err(e) => {
                        error!(
                            "MCP server '{}' reconnect failed: {}",
                            server_name, e
                        );
                        if attempt >= MAX_ATTEMPTS {
                            error!(
                                "MCP server '{}' max reconnect attempts ({}) reached, giving up",
                                server_name, MAX_ATTEMPTS
                            );
                            return;
                        }
                    }
                }
            }
        })
    }

    /// Cancel the reconnection loop.
    ///
    /// Signals the background task to stop. Idempotent — safe to call
    /// multiple times.
    pub fn cancel(&self) {
        self.cancel_token.notify_waiters();
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp_sdk::McpError;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn make_disconnected_client() -> SharedMcpClient {
        Arc::new(RwLock::new(None))
    }

    fn make_failing_config() -> MCPServerConfig {
        // Empty command triggers McpError::Config("command is required ...")
        MCPServerConfig {
            command: String::new(),
            ..Default::default()
        }
    }

    // -----------------------------------------------------------------------
    // Test: start + cancel (basic lifecycle)
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_reconnect_manager_starts_and_cancels() {
        // Start the manager and cancel immediately — the task should finish
        // promptly because the initial-delay select! races sleep vs. notify.
        let config = MCPServerConfig::default();
        let client = make_disconnected_client();
        let manager =
            McpReconnectManager::new("cancel-test".into(), config, client);

        let handle = manager.start();
        manager.cancel();

        let result =
            tokio::time::timeout(Duration::from_secs(5), handle).await;
        assert!(
            result.is_ok(),
            "reconnect task should complete promptly after cancel"
        );
    }

    // -----------------------------------------------------------------------
    // Test: multiple cancel calls (idempotency)
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_reconnect_manager_multiple_cancel() {
        // Calling cancel() multiple times should not panic or cause issues.
        let config = MCPServerConfig::default();
        let client = make_disconnected_client();
        let manager =
            McpReconnectManager::new("multi-cancel-test".into(), config, client);

        let handle = manager.start();

        // Rapid-fire cancels
        manager.cancel();
        manager.cancel();
        manager.cancel();

        let result =
            tokio::time::timeout(Duration::from_secs(5), handle).await;
        assert!(
            result.is_ok(),
            "task should complete after multiple cancels"
        );
    }

    // -----------------------------------------------------------------------
    // Test: reattempt on failure
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_reconnect_manager_reattempts_on_failure() {
        // With an empty command, new_stdio() returns Config error.
        // The loop should retry at least once instead of giving up.
        let config = make_failing_config();
        let client = make_disconnected_client();
        let manager =
            McpReconnectManager::new("retry-test".into(), config, client);

        let handle = manager.start();

        // Wait long enough for:
        //   initial_delay(100ms) + health_check(100ms) + backoff_1(20ms) = ~220ms
        // (using cfg(test) constants: INITIAL_DELAY=100ms,
        //  HEALTH_CHECK_INTERVAL=100ms, BACKOFF_BASE_MS=10ms,
        //  so 1st backoff = 10ms * 2^0 = 10ms)
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Task should still be running (it should retry, not stop)
        assert!(
            !handle.is_finished(),
            "task should still be running after first failed attempt"
        );

        manager.cancel();
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(
            handle.is_finished(),
            "task should complete after cancel on retry"
        );
    }

    // -----------------------------------------------------------------------
    // Test: abandon after max attempts
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_reconnect_manager_abandons_after_max() {
        // Simulate full failure: after MAX_ATTEMPTS retries, the loop should
        // stop permanently.
        let config = make_failing_config();
        let client = make_disconnected_client();
        let manager =
            McpReconnectManager::new("abandon-test".into(), config, client);

        let handle = manager.start();

        // Calculate total time needed for MAX_ATTEMPTS failed retries:
        //   initial_delay      = 100ms
        //   for i in 1..=5:
        //     health_check     = 100ms
        //     backoff          = 10ms * 2^(i-1)
        //   backoffs: 10 + 20 + 40 + 80 + 160 = 310ms
        //   5 × health_check  = 500ms
        //   initial_delay     = 100ms
        //   total             ≈ 910ms
        //
        // Wait generously (3× safety margin) and then verify the task
        // has finished because all attempts were exhausted.
        tokio::time::sleep(Duration::from_secs(5)).await;
        tokio::task::yield_now().await;

        assert!(
            handle.is_finished(),
            "task should have stopped after max reconnect attempts"
        );
    }

    // -----------------------------------------------------------------------
    // Test: client stays disconnected when server never connects
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_reconnect_manager_does_not_mutate_client_on_failure() {
        // Even after unsuccessful reconnection attempts, the original client
        // reference should remain untouched (still None).
        let config = make_failing_config();
        let client = make_disconnected_client();
        let manager =
            McpReconnectManager::new("no-mutate-test".into(), config, client.clone());

        let _handle = manager.start();

        // Let it fail a couple of times
        tokio::time::sleep(Duration::from_secs(1)).await;

        // Client should still be None (no successful connection was established)
        let guard: tokio::sync::RwLockReadGuard<'_, Option<Arc<McpClientWrapper>>> =
            client.read().await;
        assert!(guard.is_none(), "client should remain None after failed reconnects");

        // Cleanup
        manager.cancel();
    }

    // -----------------------------------------------------------------------
    // Test: error type round-trip for empty-command config
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_new_stdio_fails_with_empty_command() {
        // Verify that the empty-command config produces the expected error.
        // This is the error path used by the reconnect retry tests.
        let config = make_failing_config();
        let result = McpClientWrapper::new_stdio("empty-cmd-test", &config).await;

        match result {
            Err(McpError::Config(msg)) => {
                assert!(
                    msg.contains("command is required"),
                    "unexpected config error message: {}",
                    msg
                );
            }
            other => panic!(
                "expected McpError::Config, got: {:?}",
                other.as_ref().map(|_| ())
            ),
        }
    }
}
