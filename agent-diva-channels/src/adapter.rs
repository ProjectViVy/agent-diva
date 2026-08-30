//! Native Super Channel adapter contract.
//!
//! This API does not extend or wrap the retired handler trait. The old interface
//! remains untouched until the atomic CHANNEL-EPIC C6 cutover.

use agent_diva_core::channel::{
    ChannelCapabilities, ChannelCapability, ChannelCommand, ChannelHealth, ChannelId,
    DeliveryReceipt, FabricHandle,
};
use async_trait::async_trait;
use std::time::Duration;
use thiserror::Error;
use tokio_util::sync::CancellationToken;

/// Context supplied to one long-running adapter listener.
#[derive(Debug, Clone)]
pub struct AdapterContext {
    pub fabric: FabricHandle,
    pub cancel: CancellationToken,
}

impl AdapterContext {
    pub fn with_cancel(&self, cancel: CancellationToken) -> Self {
        Self {
            fabric: self.fabric.clone(),
            cancel,
        }
    }
}

/// Typed adapter failure; unsupported operations can never become silent success.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AdapterError {
    #[error("channel capability is unsupported: {capability:?}")]
    UnsupportedCapability { capability: ChannelCapability },
    #[error("adapter rate limited; retry after {retry_after:?}")]
    RateLimited { retry_after: Duration },
    #[error("adapter execution failed ({code}): {diagnosis}")]
    Execution {
        code: String,
        diagnosis: String,
        retry_after: Option<Duration>,
        retryable: bool,
    },
    #[error("adapter stopped")]
    Stopped,
}

impl AdapterError {
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::RateLimited { retry_after } => Some(*retry_after),
            Self::Execution { retry_after, .. } => *retry_after,
            Self::UnsupportedCapability { .. } | Self::Stopped => None,
        }
    }

    pub fn is_retryable(&self) -> bool {
        match self {
            Self::RateLimited { .. } => true,
            Self::Execution { retryable, .. } => *retryable,
            Self::UnsupportedCapability { .. } | Self::Stopped => false,
        }
    }

    pub fn code(&self) -> &str {
        match self {
            Self::UnsupportedCapability { .. } => "unsupported_capability",
            Self::RateLimited { .. } => "rate_limited",
            Self::Execution { code, .. } => code,
            Self::Stopped => "adapter_stopped",
        }
    }
}

/// Clean-break adapter API used by the new Registry and supervisor.
#[async_trait]
pub trait ChannelAdapter: Send + Sync + 'static {
    fn name(&self) -> ChannelId;

    /// Return the current effective snapshot (static declaration merged with live probe).
    fn capabilities(&self) -> ChannelCapabilities;

    /// Run the listener until cancellation, stop, or a transport failure.
    async fn start(&self, context: AdapterContext) -> Result<(), AdapterError>;

    /// Execute one already capability-checked platform command.
    async fn execute(&self, command: ChannelCommand) -> Result<DeliveryReceipt, AdapterError>;

    /// Return the latest local health snapshot without performing network I/O.
    fn health(&self) -> ChannelHealth;

    /// Interrupt platform reads and release adapter-owned resources.
    async fn stop(&self) -> Result<(), AdapterError>;
}
