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

use async_trait::async_trait;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use futures::Stream;
use std::pin::Pin;

/// A provider that allows hot-swapping the underlying implementation
pub struct DynamicProvider {
    inner: RwLock<Arc<dyn LLMProvider>>,
}

impl DynamicProvider {
    /// Create a new dynamic provider
    pub fn new(initial: Arc<dyn LLMProvider>) -> Self {
        Self {
            inner: RwLock::new(initial),
        }
    }

    /// Update the underlying provider
    pub fn update(&self, new_provider: Arc<dyn LLMProvider>) {
        if let Ok(mut lock) = self.inner.write() {
            *lock = new_provider;
        }
    }
    
    /// Get the current provider (for read operations)
    pub fn current(&self) -> Arc<dyn LLMProvider> {
        self.inner.read().unwrap().clone()
    }
}

#[async_trait]
impl LLMProvider for DynamicProvider {
    async fn chat(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<serde_json::Value>>,
        model: Option<String>,
        max_tokens: i32,
        temperature: f64,
    ) -> ProviderResult<LLMResponse> {
        let provider = self.current();
        provider.chat(messages, tools, model, max_tokens, temperature).await
    }

    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<serde_json::Value>>,
        model: Option<String>,
        max_tokens: i32,
        temperature: f64,
    ) -> ProviderResult<ProviderEventStream> {
        let provider = self.current();
        provider.chat_stream(messages, tools, model, max_tokens, temperature).await
    }

    fn get_default_model(&self) -> String {
        self.current().get_default_model()
    }
}
