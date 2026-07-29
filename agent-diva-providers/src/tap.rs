//! Provider tap decorator for timing and audit events.
//!
//! Wraps any `LLMProvider` to emit `AuditEvent::ProviderCallCompleted`
//! after each call, capturing latency, token usage, and status.

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use async_trait::async_trait;
use futures::Stream;

use crate::{
    LLMProvider, LLMResponse, LLMStreamEvent, Message, ProviderError, ProviderEventStream,
    ProviderResult,
};

/// Decorator that times every LLM call and emits an audit event.
///
/// Wraps any `LLMProvider` implementation transparently — the caller
/// receives the exact same results, but an `AuditEvent::ProviderCallCompleted`
/// is emitted after each `chat()` or `chat_stream()` invocation.
pub struct ProviderTap<P> {
    inner: P,
    rate_limiter: Arc<agent_diva_core::rate_limiter::RateLimiter>,
}

impl<P> ProviderTap<P> {
    /// Create a new `ProviderTap` wrapping the given provider.
    pub fn new(inner: P) -> Self {
        Self {
            inner,
            rate_limiter: Arc::new(agent_diva_core::rate_limiter::RateLimiter::new()),
        }
    }

    fn rate_limit_key(provider_name: &str, model: Option<&str>, default_model: &str) -> String {
        let model_str = model.unwrap_or(default_model);
        format!("{}:{}", provider_name, model_str)
    }

    fn into_rate_limited_error(retry_after: Option<u64>) -> ProviderError {
        ProviderError::RateLimited { retry_after }
    }
}

#[async_trait]
impl<P: LLMProvider> LLMProvider for ProviderTap<P> {
    async fn chat(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<serde_json::Value>>,
        tool_choice: crate::base::ToolChoiceMode,
        model: Option<String>,
        max_tokens: i32,
        temperature: f64,
    ) -> ProviderResult<LLMResponse> {
        let provider_name = std::any::type_name::<P>().to_string();
        let default_model = self.inner.get_default_model();
        let key = Self::rate_limit_key(&provider_name, model.as_deref(), &default_model);

        if let Err(agent_diva_core::rate_limiter::RateLimitError::Exceeded { retry_after }) =
            self.rate_limiter.check(&key).await
        {
            agent_diva_core::audit::emit(
                agent_diva_core::audit::AuditEvent::ProviderCallCompleted {
                    provider: provider_name,
                    model: model.unwrap_or(default_model),
                    latency_ms: 0,
                    tokens: 0,
                    status: "rate_limited".to_string(),
                },
            );
            return Err(Self::into_rate_limited_error(retry_after));
        }

        let start = std::time::Instant::now();
        let result = self
            .inner
            .chat(
                messages,
                tools,
                tool_choice,
                model.clone(),
                max_tokens,
                temperature,
            )
            .await;
        let latency_ms = start.elapsed().as_millis() as u64;

        let (status, tokens) = match &result {
            Ok(response) => {
                let tokens = response
                    .usage
                    .get("total_tokens")
                    .copied()
                    .or_else(|| {
                        response.usage.get("prompt_tokens").copied().and_then(|p| {
                            response
                                .usage
                                .get("completion_tokens")
                                .copied()
                                .map(|c| p + c)
                        })
                    })
                    .map(|t| t.max(0) as u32)
                    .unwrap_or_else(|| {
                        response
                            .content
                            .as_ref()
                            .map(|c| c.len() as u32 / 4)
                            .unwrap_or(0)
                    });
                ("ok".to_string(), tokens)
            }
            Err(_) => ("error".to_string(), 0),
        };

        agent_diva_core::audit::emit(agent_diva_core::audit::AuditEvent::ProviderCallCompleted {
            provider: provider_name,
            model: model.unwrap_or_else(|| self.inner.get_default_model()),
            latency_ms,
            tokens,
            status,
        });

        result
    }

    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<serde_json::Value>>,
        tool_choice: crate::base::ToolChoiceMode,
        model: Option<String>,
        max_tokens: i32,
        temperature: f64,
    ) -> ProviderResult<ProviderEventStream> {
        let provider_name = std::any::type_name::<P>().to_string();
        let default_model = self.inner.get_default_model();
        let key = Self::rate_limit_key(&provider_name, model.as_deref(), &default_model);

        if let Err(agent_diva_core::rate_limiter::RateLimitError::Exceeded { retry_after }) =
            self.rate_limiter.check(&key).await
        {
            agent_diva_core::audit::emit(
                agent_diva_core::audit::AuditEvent::ProviderCallCompleted {
                    provider: provider_name,
                    model: model.unwrap_or(default_model),
                    latency_ms: 0,
                    tokens: 0,
                    status: "rate_limited".to_string(),
                },
            );
            return Err(Self::into_rate_limited_error(retry_after));
        }

        let start = std::time::Instant::now();
        let model_str = model
            .clone()
            .unwrap_or_else(|| self.inner.get_default_model());

        match self
            .inner
            .chat_stream(messages, tools, tool_choice, model, max_tokens, temperature)
            .await
        {
            Ok(stream) => {
                let tapped = TappedStream {
                    inner: stream,
                    start,
                    provider: provider_name,
                    model: model_str,
                    token_count: 0,
                    emitted: false,
                };
                Ok(Box::pin(tapped))
            }
            Err(e) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                agent_diva_core::audit::emit(
                    agent_diva_core::audit::AuditEvent::ProviderCallCompleted {
                        provider: provider_name,
                        model: model_str,
                        latency_ms,
                        tokens: 0,
                        status: "error".to_string(),
                    },
                );
                Err(e)
            }
        }
    }

    fn get_default_model(&self) -> String {
        self.inner.get_default_model()
    }
}

/// Wrapper stream that counts tokens and emits an audit event on completion.
struct TappedStream {
    inner: ProviderEventStream,
    start: std::time::Instant,
    provider: String,
    model: String,
    token_count: u32,
    emitted: bool,
}

impl Stream for TappedStream {
    type Item = ProviderResult<LLMStreamEvent>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = &mut *self;
        match Pin::new(&mut this.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(LLMStreamEvent::TextDelta(ref text)))) => {
                this.token_count += text.len() as u32 / 4;
                Poll::Ready(Some(Ok(LLMStreamEvent::TextDelta(text.clone()))))
            }
            Poll::Ready(Some(Ok(LLMStreamEvent::ReasoningDelta(ref text)))) => {
                this.token_count += text.len() as u32 / 4;
                Poll::Ready(Some(Ok(LLMStreamEvent::ReasoningDelta(text.clone()))))
            }
            Poll::Ready(Some(Ok(LLMStreamEvent::ToolCallDelta {
                index,
                id,
                name,
                arguments_delta,
            }))) => {
                // Tool call deltas don't contribute to token count in a meaningful way here
                Poll::Ready(Some(Ok(LLMStreamEvent::ToolCallDelta {
                    index,
                    id,
                    name,
                    arguments_delta,
                })))
            }
            Poll::Ready(Some(Ok(LLMStreamEvent::Completed(ref response)))) => {
                // Extract token count from the completed response
                if let Some(total) = response.usage.get("total_tokens").copied() {
                    this.token_count = total.max(0) as u32;
                } else if let (Some(prompt), Some(completion)) = (
                    response.usage.get("prompt_tokens").copied(),
                    response.usage.get("completion_tokens").copied(),
                ) {
                    this.token_count = (prompt + completion).max(0) as u32;
                }
                this.emit_event("ok");
                Poll::Ready(Some(Ok(LLMStreamEvent::Completed(response.clone()))))
            }
            Poll::Ready(Some(Err(ref e))) => {
                let err = match e {
                    ProviderError::HttpError(inner) => ProviderError::ApiError(inner.to_string()),
                    ProviderError::JsonError(inner) => ProviderError::ApiError(inner.to_string()),
                    ProviderError::InvalidResponse(s) => ProviderError::InvalidResponse(s.clone()),
                    ProviderError::ApiError(s) => ProviderError::ApiError(s.clone()),
                    ProviderError::ConfigError(s) => ProviderError::ConfigError(s.clone()),
                    ProviderError::RateLimited { retry_after } => ProviderError::RateLimited {
                        retry_after: *retry_after,
                    },
                };
                this.emit_event("error");
                Poll::Ready(Some(Err(err)))
            }
            Poll::Ready(None) => {
                if !this.emitted {
                    this.emit_event("ok");
                }
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl TappedStream {
    fn emit_event(&mut self, status: &str) {
        if self.emitted {
            return;
        }
        self.emitted = true;
        agent_diva_core::audit::emit(agent_diva_core::audit::AuditEvent::ProviderCallCompleted {
            provider: self.provider.clone(),
            model: self.model.clone(),
            latency_ms: self.start.elapsed().as_millis() as u64,
            tokens: self.token_count,
            status: status.to_string(),
        });
    }
}

impl Drop for TappedStream {
    fn drop(&mut self) {
        if !self.emitted {
            self.emit_event("cancelled");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockProvider {
        should_fail: bool,
        call_count: AtomicUsize,
    }

    impl MockProvider {
        fn new(should_fail: bool) -> Self {
            Self {
                should_fail,
                call_count: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl LLMProvider for MockProvider {
        async fn chat(
            &self,
            _messages: Vec<Message>,
            _tools: Option<Vec<serde_json::Value>>,
            _tool_choice: crate::base::ToolChoiceMode,
            _model: Option<String>,
            _max_tokens: i32,
            _temperature: f64,
        ) -> ProviderResult<LLMResponse> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            if self.should_fail {
                Err(ProviderError::ApiError("mock error".into()))
            } else {
                let mut usage = HashMap::new();
                usage.insert("total_tokens".to_string(), 42i64);
                Ok(LLMResponse {
                    content: Some("hello".into()),
                    tool_calls: vec![],
                    finish_reason: "stop".into(),
                    usage,
                    reasoning_content: None,
                })
            }
        }

        async fn chat_stream(
            &self,
            _messages: Vec<Message>,
            _tools: Option<Vec<serde_json::Value>>,
            _tool_choice: crate::base::ToolChoiceMode,
            _model: Option<String>,
            _max_tokens: i32,
            _temperature: f64,
        ) -> ProviderResult<ProviderEventStream> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            if self.should_fail {
                Err(ProviderError::ApiError("mock stream error".into()))
            } else {
                let events: Vec<ProviderResult<LLMStreamEvent>> = vec![
                    Ok(LLMStreamEvent::TextDelta("hello ".into())),
                    Ok(LLMStreamEvent::TextDelta("world".into())),
                    Ok(LLMStreamEvent::Completed(LLMResponse {
                        content: Some("hello world".into()),
                        tool_calls: vec![],
                        finish_reason: "stop".into(),
                        usage: HashMap::new(),
                        reasoning_content: None,
                    })),
                ];
                Ok(Box::pin(futures::stream::iter(events)))
            }
        }

        fn get_default_model(&self) -> String {
            "mock-model".into()
        }
    }

    #[tokio::test]
    async fn test_chat_success_emits_ok_event() {
        let mock = MockProvider::new(false);
        let tap = ProviderTap::new(mock);

        let result = tap
            .chat(
                vec![],
                None,
                crate::base::ToolChoiceMode::Unspecified,
                Some("gpt-4".into()),
                100,
                0.7,
            )
            .await;
        assert!(result.is_ok());
        // Event is emitted via tracing; we verify the call succeeds
    }

    #[tokio::test]
    async fn test_chat_failure_emits_error_event() {
        let mock = MockProvider::new(true);
        let tap = ProviderTap::new(mock);

        let result = tap
            .chat(
                vec![],
                None,
                crate::base::ToolChoiceMode::Unspecified,
                Some("gpt-4".into()),
                100,
                0.7,
            )
            .await;
        assert!(result.is_err());
        // Event is emitted via tracing; we verify the call returns error
    }

    #[tokio::test]
    async fn test_chat_stream_success_emits_event() {
        let mock = MockProvider::new(false);
        let tap = ProviderTap::new(mock);

        let mut stream = tap
            .chat_stream(
                vec![],
                None,
                crate::base::ToolChoiceMode::Unspecified,
                Some("gpt-4".into()),
                100,
                0.7,
            )
            .await
            .unwrap();

        use futures::StreamExt;
        let mut events = vec![];
        while let Some(event) = stream.next().await {
            events.push(event);
        }
        assert_eq!(events.len(), 3);
    }

    #[tokio::test]
    async fn test_chat_stream_failure_emits_error_event() {
        let mock = MockProvider::new(true);
        let tap = ProviderTap::new(mock);

        let result = tap
            .chat_stream(
                vec![],
                None,
                crate::base::ToolChoiceMode::Unspecified,
                Some("gpt-4".into()),
                100,
                0.7,
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_default_model_delegates() {
        let mock = MockProvider::new(false);
        let tap = ProviderTap::new(mock);

        assert_eq!(tap.get_default_model(), "mock-model");
    }

    #[test]
    fn test_rate_limited_error_preserves_unknown_retry_after() {
        assert!(matches!(
            ProviderTap::<MockProvider>::into_rate_limited_error(None),
            ProviderError::RateLimited { retry_after: None }
        ));
    }

    #[test]
    fn test_rate_limited_error_preserves_known_retry_after() {
        assert!(matches!(
            ProviderTap::<MockProvider>::into_rate_limited_error(Some(3)),
            ProviderError::RateLimited {
                retry_after: Some(3)
            }
        ));
    }
}
