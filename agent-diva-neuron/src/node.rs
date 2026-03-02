//! Core trait and error types for neuron nodes.

use crate::events::NeuronEvent;
use crate::types::{NeuronRequest, NeuronResponse};
use async_trait::async_trait;
use thiserror::Error;
use tokio::sync::mpsc;

/// Error type for neuron execution.
#[derive(Debug, Error)]
pub enum NeuronError {
    #[error("Provider error: {0}")]
    Provider(#[from] agent_diva_providers::ProviderError),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Abstraction for a single-turn neuron node.
#[async_trait]
pub trait NeuronNode: Send + Sync {
    /// Run one non-looping LLM turn and return aggregated output.
    async fn run_once(&self, req: NeuronRequest) -> Result<NeuronResponse, NeuronError>;

    /// Run one non-looping LLM turn and optionally emit neuron-local events.
    async fn run_once_stream(
        &self,
        req: NeuronRequest,
        event_tx: Option<mpsc::UnboundedSender<NeuronEvent>>,
    ) -> Result<NeuronResponse, NeuronError>;
}
