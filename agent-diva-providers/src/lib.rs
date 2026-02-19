//! LLM provider integrations for agent-diva
//!
//! This crate provides abstractions and implementations for various LLM providers.

pub mod base;
pub mod litellm;
pub mod registry;
pub mod transcription;

pub use base::{
    LLMProvider, LLMResponse, LLMStreamEvent, Message, ProviderError, ProviderEventStream,
    ProviderResult, ToolCallRequest,
};
pub use litellm::LiteLLMClient;
pub use registry::{ProviderRegistry, ProviderSpec};
