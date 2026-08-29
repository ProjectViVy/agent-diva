//! LLM provider integrations for agent-diva
//!
//! This crate provides abstractions and implementations for various LLM providers.

pub mod anthropic;
pub mod base;
pub mod catalog;
mod deepseek_v4_dsml;
pub mod discovery;
pub mod factory;
pub mod final_wire;
mod http_util;
pub mod ollama;
pub mod openai_compatible;
pub mod registry;
pub mod report_narrative;
pub mod request_observers;
pub mod retry;
pub mod tap;
pub mod transcription;

pub use base::{
    model_capabilities_for_model, model_capabilities_for_model_with_config,
    supports_reasoning_model, supports_reasoning_model_with_config, supports_vision_model,
    DynamicContextTransport, ImageData, ImageFile, ImageUrl, LLMProvider, LLMResponse,
    LLMStreamEvent, Message, MessageContent, MessageContentPart, ModelCapabilities,
    PromptCachePolicy, PromptCacheProfile, ProviderError, ProviderEventStream, ProviderResult,
    ToolCallRequest, ToolChoiceMode,
};
pub use catalog::{
    CustomProviderUpsert, ProviderCatalogService, ProviderModelCatalogView, ProviderModelEntry,
    ProviderModelSource, ProviderSource, ProviderView,
};
pub use discovery::{
    fetch_provider_model_catalog, ModelCatalogSource, ProviderAccess, ProviderModelCatalog,
};
pub use factory::{build_llm_provider, LlmProviderBuildOptions};
pub use final_wire::{FinalWireCacheListener, FinalWireCacheSnapshot};
pub use ollama::OllamaProvider;
pub use openai_compatible::OpenAiCompatibleClient;
pub use registry::{ProviderRegistry, ProviderSpec};
pub use report_narrative::LlmReportNarrativeGenerator;
pub use request_observers::{
    current_final_wire_cache_listener, current_retry_listener, with_provider_request_observers,
    ProviderRequestObservers,
};

use async_trait::async_trait;
use std::sync::{Arc, RwLock};

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
        tool_choice: ToolChoiceMode,
        model: Option<String>,
        max_tokens: i32,
        temperature: f64,
    ) -> ProviderResult<LLMResponse> {
        let provider = self.current();
        provider
            .chat(messages, tools, tool_choice, model, max_tokens, temperature)
            .await
    }

    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<serde_json::Value>>,
        tool_choice: ToolChoiceMode,
        model: Option<String>,
        max_tokens: i32,
        temperature: f64,
    ) -> ProviderResult<ProviderEventStream> {
        let provider = self.current();
        provider
            .chat_stream(messages, tools, tool_choice, model, max_tokens, temperature)
            .await
    }

    fn get_default_model(&self) -> String {
        self.current().get_default_model()
    }

    fn dynamic_context_transport(&self) -> DynamicContextTransport {
        self.current().dynamic_context_transport()
    }

    fn prompt_cache_profile(&self, model: &str) -> PromptCacheProfile {
        self.current().prompt_cache_profile(model)
    }

    fn set_retry_listener(&self, listener: Option<crate::retry::RetryListener>) {
        self.current().set_retry_listener(listener);
    }

    fn set_final_wire_cache_listener(&self, listener: Option<FinalWireCacheListener>) {
        self.current().set_final_wire_cache_listener(listener);
    }
}
