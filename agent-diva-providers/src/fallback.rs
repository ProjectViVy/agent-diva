//! Provider fallback layer — model-level and cross-driver fallback.
//!
//! `ProviderFallbackLayer` wraps a primary provider and a chain of fallback
//! entries.  When the primary (or a preceding fallback) returns a transient or
//! rate-limited error, the layer retries with the next fallback entry.
//!
//! # Features
//!
//! - **Model fallback**: claude-haiku fails → claude-sonnet on the same driver.
//! - **Cross-driver fallback**: Anthropic fails → LiteLLM (OpenAI-compatible).
//! - **Max depth (default 3)**: prevents runaway cascading through a long chain.
//! - **Loop detection**: tracks already-attempted `(provider_name, model)` tuples
//!   and skips duplicate entries.
//!
//! # Non-fallbackable errors
//!
//! Auth, permanent, configuration, JSON, tool-schema, and invalid-response
//! errors are NOT fallbackable — the layer returns them immediately without
//! trying the next entry.

use async_trait::async_trait;
use std::collections::HashSet;
use std::fmt;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::base::{
    LLMProvider, LLMResponse, Message, ProviderError, ProviderEventStream, ProviderResult,
};

/// Maximum fallback depth when not explicitly configured.
pub const DEFAULT_MAX_DEPTH: usize = 3;

// ── FallbackEntry ──────────────────────────────────────────────────────

/// A named provider entry with an optional model override.
///
/// The `name` is used for logging and loop detection.  When `model` is `Some`,
/// the fallback layer uses that model instead of the original request model.
#[derive(Clone)]
pub struct FallbackEntry {
    /// Human-readable name for this entry (logging / loop detection).
    pub name: String,
    /// The provider instance.
    pub provider: Arc<dyn LLMProvider>,
    /// Optional model override.  When `None`, the layer reuses the original
    /// request model.
    pub model: Option<String>,
}

impl fmt::Debug for FallbackEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FallbackEntry")
            .field("name", &self.name)
            .field("model", &self.model)
            .finish_non_exhaustive()
    }
}

impl FallbackEntry {
    /// Create a new fallback entry with a name and provider.
    pub fn new(name: impl Into<String>, provider: Arc<dyn LLMProvider>) -> Self {
        Self {
            name: name.into(),
            provider,
            model: None,
        }
    }

    /// Attach a model override to this entry.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

// ── ProviderFallbackLayer ──────────────────────────────────────────────

/// A provider that delegates to a fallback chain on transient / rate-limit errors.
///
/// # Example
///
/// ```ignore
/// use agent_diva_providers::fallback::{FallbackEntry, ProviderFallbackLayer};
///
/// let primary = FallbackEntry::new("anthropic", anthropic_client)
///     .with_model("claude-haiku-3-5-sonnet-20241022");
///
/// let fallback_chain = vec![
///     FallbackEntry::new("anthropic-sonnet", anthropic_client)
///         .with_model("claude-sonnet-4-20250514"),
///     FallbackEntry::new("litellm", litellm_client)
///         .with_model("gpt-4o"),
/// ];
///
/// let layer = ProviderFallbackLayer::new(primary, fallback_chain)
///     .with_max_depth(3);
/// ```
pub struct ProviderFallbackLayer {
    /// Primary entry (depth 0).
    primary: FallbackEntry,
    /// Ordered fallback chain (depth 1..).
    fallback_chain: Vec<FallbackEntry>,
    /// Maximum fallback depth (primary is depth 0).
    max_depth: usize,
}

impl fmt::Debug for ProviderFallbackLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProviderFallbackLayer")
            .field("primary", &self.primary.name)
            .field("chain_len", &self.fallback_chain.len())
            .field("max_depth", &self.max_depth)
            .finish()
    }
}

impl ProviderFallbackLayer {
    /// Create a new fallback layer with `primary` and a fallback chain.
    ///
    /// Max depth defaults to [`DEFAULT_MAX_DEPTH`] (3).
    pub fn new(primary: FallbackEntry, fallback_chain: Vec<FallbackEntry>) -> Self {
        Self {
            primary,
            fallback_chain,
            max_depth: DEFAULT_MAX_DEPTH,
        }
    }

    /// Override the maximum fallback depth.
    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Return all entries in order (primary first, then chain).
    fn all_entries(&self) -> impl Iterator<Item = &FallbackEntry> {
        std::iter::once(&self.primary).chain(self.fallback_chain.iter())
    }

    /// Resolve the effective model for a fallback entry.
    ///
    /// The entry's model override takes priority; otherwise, `original_model`
    /// (from the caller's request) is used.
    fn resolve_model(entry: &FallbackEntry, original_model: &Option<String>) -> Option<String> {
        entry.model.clone().or_else(|| original_model.clone())
    }

    /// Execute fallback chain for non-streaming chat.
    ///
    /// Walks the entry list (primary → chain), calling each provider.
    /// On a fallback-eligible error, tries the next entry.
    /// On a non-fallbackable error, returns immediately.
    /// Respects max_depth and loop detection.
    async fn try_chain_chat(
        &self,
        messages: &[Message],
        tools: &Option<Vec<serde_json::Value>>,
        original_model: &Option<String>,
        max_tokens: i32,
        temperature: f64,
    ) -> ProviderResult<LLMResponse> {
        let mut attempted: HashSet<(String, Option<String>)> = HashSet::new();

        for (depth, entry) in self.all_entries().enumerate() {
            if depth > self.max_depth {
                warn!(
                    "Fallback max depth ({}) exceeded at entry '{}' (depth {})",
                    self.max_depth, entry.name, depth
                );
                return Err(ProviderError::Permanent {
                    message: format!(
                        "Fallback max depth {} exceeded ({} entries available)",
                        self.max_depth,
                        self.fallback_chain.len() + 1,
                    ),
                });
            }

            let effective_model = Self::resolve_model(entry, original_model);
            let key = (entry.name.clone(), effective_model.clone());

            if !attempted.insert(key) {
                debug!(
                    "Skipping fallback entry '{}' (model {:?}) — loop detected",
                    entry.name, effective_model
                );
                continue;
            }

            if depth > 0 {
                debug!(
                    "Falling back to '{}' (depth {}/{})",
                    entry.name, depth, self.max_depth
                );
            }

            match entry
                .provider
                .chat(
                    messages.to_vec(),
                    tools.clone(),
                    effective_model,
                    max_tokens,
                    temperature,
                )
                .await
            {
                Ok(response) => {
                    if depth > 0 {
                        debug!("Fallback '{}' succeeded at depth {}", entry.name, depth);
                    }
                    return Ok(response);
                }
                Err(error) => {
                    if !Self::is_fallbackable(&error) {
                        return Err(error);
                    }
                    warn!(
                        "Fallback entry '{}' (depth {}) failed with retryable error: {}",
                        entry.name, depth, error
                    );
                }
            }
        }

        Err(ProviderError::Permanent {
            message: "All fallback providers exhausted".to_string(),
        })
    }

    /// Execute fallback chain for streaming chat.
    async fn try_chain_chat_stream(
        &self,
        messages: &[Message],
        tools: &Option<Vec<serde_json::Value>>,
        original_model: &Option<String>,
        max_tokens: i32,
        temperature: f64,
    ) -> ProviderResult<ProviderEventStream> {
        let mut attempted: HashSet<(String, Option<String>)> = HashSet::new();

        for (depth, entry) in self.all_entries().enumerate() {
            if depth > self.max_depth {
                warn!(
                    "Fallback max depth ({}) exceeded at entry '{}' (depth {})",
                    self.max_depth, entry.name, depth
                );
                return Err(ProviderError::Permanent {
                    message: format!(
                        "Fallback max depth {} exceeded ({} entries available)",
                        self.max_depth,
                        self.fallback_chain.len() + 1,
                    ),
                });
            }

            let effective_model = Self::resolve_model(entry, original_model);
            let key = (entry.name.clone(), effective_model.clone());

            if !attempted.insert(key) {
                debug!(
                    "Skipping fallback entry '{}' (model {:?}) — loop detected",
                    entry.name, effective_model
                );
                continue;
            }

            if depth > 0 {
                debug!(
                    "Falling back to '{}' (depth {}/{})",
                    entry.name, depth, self.max_depth
                );
            }

            match entry
                .provider
                .chat_stream(
                    messages.to_vec(),
                    tools.clone(),
                    effective_model,
                    max_tokens,
                    temperature,
                )
                .await
            {
                Ok(stream) => {
                    if depth > 0 {
                        debug!("Fallback '{}' succeeded at depth {}", entry.name, depth);
                    }
                    return Ok(stream);
                }
                Err(error) => {
                    if !Self::is_fallbackable(&error) {
                        return Err(error);
                    }
                    warn!(
                        "Fallback entry '{}' (depth {}) failed with retryable error: {}",
                        entry.name, depth, error
                    );
                }
            }
        }

        Err(ProviderError::Permanent {
            message: "All fallback providers exhausted".to_string(),
        })
    }

    /// Determine if an error is eligible for fallback.
    ///
    /// Returns `true` for `RateLimited` and `Transient` variants only.
    /// All other errors (Auth, Permanent, ToolSchema, ConfigError, JsonError,
    /// InvalidResponse, HttpError, ApiError) are NOT fallbackable.
    pub fn is_fallbackable(error: &ProviderError) -> bool {
        matches!(
            error,
            ProviderError::RateLimited { .. } | ProviderError::Transient { .. }
        )
    }
}

// ── LLMProvider impl ───────────────────────────────────────────────────

#[async_trait]
impl LLMProvider for ProviderFallbackLayer {
    async fn chat(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<serde_json::Value>>,
        model: Option<String>,
        max_tokens: i32,
        temperature: f64,
    ) -> ProviderResult<LLMResponse> {
        self.try_chain_chat(&messages, &tools, &model, max_tokens, temperature)
            .await
    }

    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<serde_json::Value>>,
        model: Option<String>,
        max_tokens: i32,
        temperature: f64,
    ) -> ProviderResult<ProviderEventStream> {
        self.try_chain_chat_stream(&messages, &tools, &model, max_tokens, temperature)
            .await
    }

    fn get_default_model(&self) -> String {
        self.primary
            .model
            .clone()
            .unwrap_or_else(|| self.primary.provider.get_default_model())
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::{LLMResponse, LLMStreamEvent};
    use std::sync::Mutex;

    // A mock provider that can be programmed to succeed / fail.
    struct MockProvider {
        default_model: String,
        // (call_count, behavior): returns Ok(call_count) or specific error
        behavior: Mutex<MockBehavior>,
    }

    enum MockBehavior {
        /// Always succeed with this content.
        AlwaysOk(String),
        /// Always fail with this error variant and message.
        AlwaysFail {
            variant: MockErrorVariant,
            message: String,
        },
    }

    /// Error variant to construct (avoids cloning ProviderError).
    #[derive(Clone, Copy)]
    enum MockErrorVariant {
        Transient,
        RateLimited,
        Auth,
        Permanent,
    }

    impl MockErrorVariant {
        fn to_error(&self, message: &str) -> ProviderError {
            match self {
                MockErrorVariant::Transient => ProviderError::Transient {
                    message: message.to_string(),
                },
                MockErrorVariant::RateLimited => ProviderError::RateLimited {
                    retry_after: Some(std::time::Duration::from_secs(1)),
                },
                MockErrorVariant::Auth => ProviderError::Auth {
                    message: message.to_string(),
                },
                MockErrorVariant::Permanent => ProviderError::Permanent {
                    message: message.to_string(),
                },
            }
        }
    }

    impl MockProvider {
        fn always_ok(_name: &str, model: &str, content: &str) -> Self {
            Self {
                default_model: model.to_string(),
                behavior: Mutex::new(MockBehavior::AlwaysOk(content.to_string())),
            }
        }

        fn always_err(_name: &str, model: &str, variant: MockErrorVariant, msg: &str) -> Self {
            Self {
                default_model: model.to_string(),
                behavior: Mutex::new(MockBehavior::AlwaysFail {
                    variant,
                    message: msg.to_string(),
                }),
            }
        }

        fn transient_err(name: &str, model: &str, msg: &str) -> Self {
            Self::always_err(name, model, MockErrorVariant::Transient, msg)
        }

        fn rate_limited(name: &str, model: &str) -> Self {
            Self::always_err(name, model, MockErrorVariant::RateLimited, "rate limited")
        }

        fn auth_err(name: &str, model: &str, msg: &str) -> Self {
            Self::always_err(name, model, MockErrorVariant::Auth, msg)
        }

        fn permanent_err(name: &str, model: &str, msg: &str) -> Self {
            Self::always_err(name, model, MockErrorVariant::Permanent, msg)
        }
    }

    #[async_trait]
    impl LLMProvider for MockProvider {
        async fn chat(
            &self,
            _messages: Vec<Message>,
            _tools: Option<Vec<serde_json::Value>>,
            _model: Option<String>,
            _max_tokens: i32,
            _temperature: f64,
        ) -> ProviderResult<LLMResponse> {
            let behavior = self.behavior.lock().unwrap();
            match &*behavior {
                MockBehavior::AlwaysOk(content) => Ok(LLMResponse {
                    content: Some(content.clone()),
                    tool_calls: vec![],
                    finish_reason: "stop".to_string(),
                    usage: None,
                    reasoning_content: None,
                }),
                MockBehavior::AlwaysFail { variant, message } => Err(variant.to_error(message)),
            }
        }

        async fn chat_stream(
            &self,
            messages: Vec<Message>,
            tools: Option<Vec<serde_json::Value>>,
            model: Option<String>,
            max_tokens: i32,
            temperature: f64,
        ) -> ProviderResult<ProviderEventStream> {
            use crate::base::LLMStreamEvent;
            // Delegate to non-streaming, wrap result in a stream.
            self.chat(messages, tools, model, max_tokens, temperature)
                .await
                .map(|resp| {
                    Box::pin(futures::stream::iter(vec![Ok(LLMStreamEvent::Completed(
                        resp,
                    ))])) as ProviderEventStream
                })
        }

        fn get_default_model(&self) -> String {
            self.default_model.clone()
        }
    }

    // ── Helpers ───────────────────────────────────────────────────

    fn make_entry(name: &str, provider: Arc<MockProvider>, model: Option<&str>) -> FallbackEntry {
        let mut entry = FallbackEntry::new(name, provider);
        if let Some(m) = model {
            entry = entry.with_model(m);
        }
        entry
    }

    fn dummy_messages() -> Vec<Message> {
        vec![Message::user("hello")]
    }

    // ── Model fallback tests ──────────────────────────────────────

    #[tokio::test]
    async fn primary_succeeds_no_fallback_called() {
        let primary = Arc::new(MockProvider::always_ok("p1", "m1", "from-primary"));
        let fallback = Arc::new(MockProvider::always_ok("p2", "m2", "from-fallback"));

        let layer = ProviderFallbackLayer::new(
            make_entry("primary", primary.clone(), None),
            vec![make_entry("fallback", fallback.clone(), None)],
        );

        let resp = layer
            .chat(dummy_messages(), None, None, 1024, 0.0)
            .await
            .unwrap();

        assert_eq!(resp.content.unwrap(), "from-primary");
    }

    #[tokio::test]
    async fn model_fallback_on_transient() {
        let primary = Arc::new(MockProvider::transient_err("p1", "m1", "timeout"));
        let fallback = Arc::new(MockProvider::always_ok("p2", "m2", "from-fallback"));

        let layer = ProviderFallbackLayer::new(
            make_entry("primary", primary, Some("claude-haiku")),
            vec![make_entry("fallback", fallback, Some("claude-sonnet"))],
        );

        let resp = layer
            .chat(dummy_messages(), None, None, 1024, 0.0)
            .await
            .unwrap();

        assert_eq!(resp.content.unwrap(), "from-fallback");
    }

    #[tokio::test]
    async fn model_fallback_on_rate_limited() {
        let primary = Arc::new(MockProvider::rate_limited("p1", "m1"));
        let fallback = Arc::new(MockProvider::always_ok("p2", "m2", "from-fallback"));

        let layer = ProviderFallbackLayer::new(
            make_entry("primary", primary, Some("claude-haiku")),
            vec![make_entry("fallback", fallback, Some("claude-sonnet"))],
        );

        let resp = layer
            .chat(dummy_messages(), None, None, 1024, 0.0)
            .await
            .unwrap();

        assert_eq!(resp.content.unwrap(), "from-fallback");
    }

    // ── Cross-driver fallback tests ───────────────────────────────

    #[tokio::test]
    async fn cross_driver_fallback_anthropic_to_litellm() {
        // Anthropic-like provider fails with transient → LiteLLM-like succeeds.
        let anthropic = Arc::new(MockProvider::transient_err(
            "anthropic",
            "claude-haiku",
            "service overload",
        ));
        let litellm = Arc::new(MockProvider::always_ok("litellm", "gpt-4o", "from-litellm"));

        let layer = ProviderFallbackLayer::new(
            make_entry("anthropic", anthropic, Some("claude-haiku")),
            vec![make_entry("litellm", litellm, Some("gpt-4o"))],
        );

        let resp = layer
            .chat(dummy_messages(), None, None, 1024, 0.0)
            .await
            .unwrap();

        assert_eq!(resp.content.unwrap(), "from-litellm");
    }

    #[tokio::test]
    async fn cross_driver_both_fail_exhausted() {
        let p1 = Arc::new(MockProvider::transient_err("anthropic", "m1", "down"));
        let p2 = Arc::new(MockProvider::transient_err("litellm", "m2", "also down"));

        let layer = ProviderFallbackLayer::new(
            make_entry("anthropic", p1, None),
            vec![make_entry("litellm", p2, None)],
        );

        let result = layer.chat(dummy_messages(), None, None, 1024, 0.0).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        match err {
            ProviderError::Permanent { message } => {
                assert!(message.contains("exhausted"));
            }
            other => panic!("expected Permanent, got: {:?}", other),
        }
    }

    // ── Non-fallbackable error tests ──────────────────────────────

    #[tokio::test]
    async fn non_fallbackable_auth_returns_immediately() {
        let primary = Arc::new(MockProvider::auth_err("p1", "m1", "invalid key"));
        let fallback = Arc::new(MockProvider::always_ok("p2", "m2", "should-not-reach"));

        let layer = ProviderFallbackLayer::new(
            make_entry("primary", primary, None),
            vec![make_entry("fallback", fallback, None)],
        );

        let result = layer.chat(dummy_messages(), None, None, 1024, 0.0).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ProviderError::Auth { .. } => {} // expected
            other => panic!("expected Auth, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn non_fallbackable_permanent_returns_immediately() {
        let primary = Arc::new(MockProvider::permanent_err("p1", "m1", "bad request"));
        let fallback = Arc::new(MockProvider::always_ok("p2", "m2", "should-not-reach"));

        let layer = ProviderFallbackLayer::new(
            make_entry("primary", primary, None),
            vec![make_entry("fallback", fallback, None)],
        );

        let result = layer.chat(dummy_messages(), None, None, 1024, 0.0).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ProviderError::Permanent { .. } => {} // expected
            other => panic!("expected Permanent, got: {:?}", other),
        }
    }

    // ── Max depth tests ───────────────────────────────────────────

    #[tokio::test]
    async fn max_depth_exceeded_returns_error() {
        let p1 = Arc::new(MockProvider::transient_err("p1", "m1", "fail-1"));
        let p2 = Arc::new(MockProvider::transient_err("p2", "m2", "fail-2"));
        let p3 = Arc::new(MockProvider::transient_err("p3", "m3", "fail-3"));

        // max_depth = 0: only primary allowed.
        let layer = ProviderFallbackLayer::new(
            make_entry("p1", p1, None),
            vec![make_entry("p2", p2, None), make_entry("p3", p3, None)],
        )
        .with_max_depth(0);

        let result = layer.chat(dummy_messages(), None, None, 1024, 0.0).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            // depth 0 > max_depth 0 → Permanent, not the transient from p1
            ProviderError::Permanent { .. } => {}
            other => panic!("expected Permanent (max depth exceeded), got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn third_entry_in_chain_fails_but_within_max_depth() {
        // Three entries, max_depth=3: all tried and fail → exhausted.
        let p1 = Arc::new(MockProvider::transient_err("p1", "m1", "fail-1"));
        let p2 = Arc::new(MockProvider::transient_err("p2", "m2", "fail-2"));
        let p3 = Arc::new(MockProvider::transient_err("p3", "m3", "fail-3"));

        let layer = ProviderFallbackLayer::new(
            make_entry("p1", p1, None),
            vec![make_entry("p2", p2, None), make_entry("p3", p3, None)],
        );

        let result = layer.chat(dummy_messages(), None, None, 1024, 0.0).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ProviderError::Permanent { message } => {
                assert!(message.contains("exhausted"));
            }
            other => panic!("expected Permanent, got: {:?}", other),
        }
    }

    // ── Loop detection tests ──────────────────────────────────────

    #[tokio::test]
    async fn loop_detection_skips_duplicate_entry() {
        // Same (name, model) tuple appears twice — second one skipped.
        let p1 = Arc::new(MockProvider::transient_err(
            "dup-provider",
            "m1",
            "fail-dup",
        ));
        let p2 = Arc::new(MockProvider::always_ok("dup-provider", "m1", "from-dup"));

        let layer = ProviderFallbackLayer::new(
            make_entry("dup-provider", p1, Some("same-model")),
            vec![make_entry("dup-provider", p2, Some("same-model"))],
        );

        // The second entry has the same (name, model) tuple → loop detected → skipped.
        // After skipping, all entries are exhausted.
        let result = layer.chat(dummy_messages(), None, None, 1024, 0.0).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ProviderError::Permanent { message } => {
                assert!(message.contains("exhausted"));
            }
            other => panic!("expected Permanent, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn no_loop_when_same_name_different_model() {
        // Same name but different model → NOT a loop, should proceed.
        let p1 = Arc::new(MockProvider::transient_err("same-name", "m1", "fail-1"));
        let p2 = Arc::new(MockProvider::always_ok("same-name", "m2", "from-same-name"));

        let layer = ProviderFallbackLayer::new(
            make_entry("same-name", p1, Some("model-a")),
            vec![make_entry("same-name", p2, Some("model-b"))],
        );

        let resp = layer
            .chat(dummy_messages(), None, None, 1024, 0.0)
            .await
            .unwrap();

        assert_eq!(resp.content.unwrap(), "from-same-name");
    }

    // ── Model override tests ──────────────────────────────────────

    #[tokio::test]
    async fn entry_model_override_takes_priority() {
        let primary = Arc::new(MockProvider::transient_err("p1", "default", "fail"));
        let fallback = Arc::new(MockProvider::always_ok(
            "p2",
            "override-model",
            "from-override",
        ));

        let layer = ProviderFallbackLayer::new(
            make_entry("p1", primary, None),
            vec![make_entry("p2", fallback, Some("claude-sonnet"))],
        );

        // Even though the request passes model=None, fallback uses its override.
        let resp = layer
            .chat(dummy_messages(), None, None, 1024, 0.0)
            .await
            .unwrap();

        assert_eq!(resp.content.unwrap(), "from-override");
    }

    // ── is_fallbackable tests ─────────────────────────────────────

    #[test]
    fn is_fallbackable_rate_limited() {
        assert!(ProviderFallbackLayer::is_fallbackable(
            &ProviderError::RateLimited {
                retry_after: Some(std::time::Duration::from_secs(1))
            }
        ));
    }

    #[test]
    fn is_fallbackable_transient() {
        assert!(ProviderFallbackLayer::is_fallbackable(
            &ProviderError::Transient {
                message: "timeout".into()
            }
        ));
    }

    #[test]
    fn is_not_fallbackable_auth() {
        assert!(!ProviderFallbackLayer::is_fallbackable(
            &ProviderError::Auth {
                message: "bad key".into()
            }
        ));
    }

    #[test]
    fn is_not_fallbackable_permanent() {
        assert!(!ProviderFallbackLayer::is_fallbackable(
            &ProviderError::Permanent {
                message: "bad request".into()
            }
        ));
    }

    #[test]
    fn is_not_fallbackable_config() {
        assert!(!ProviderFallbackLayer::is_fallbackable(
            &ProviderError::ConfigError("missing key".into())
        ));
    }

    #[test]
    fn is_not_fallbackable_json() {
        assert!(!ProviderFallbackLayer::is_fallbackable(
            &ProviderError::JsonError(
                serde_json::from_str::<serde_json::Value>("not json").unwrap_err()
            )
        ));
    }

    // ── get_default_model tests ───────────────────────────────────

    #[test]
    fn get_default_model_uses_primary_override() {
        let primary = Arc::new(MockProvider::always_ok("p1", "inner-default", "ok"));
        let layer =
            ProviderFallbackLayer::new(make_entry("p1", primary, Some("override-model")), vec![]);

        assert_eq!(layer.get_default_model(), "override-model");
    }

    #[test]
    fn get_default_model_falls_back_to_inner() {
        let primary = Arc::new(MockProvider::always_ok("p1", "inner-default", "ok"));
        let layer = ProviderFallbackLayer::new(make_entry("p1", primary, None), vec![]);

        assert_eq!(layer.get_default_model(), "inner-default");
    }

    // ── Stream fallback test ──────────────────────────────────────

    #[tokio::test]
    async fn stream_fallback_on_transient() {
        use futures::StreamExt;

        let primary = Arc::new(MockProvider::transient_err("p1", "m1", "stream-fail"));
        let fallback = Arc::new(MockProvider::always_ok("p2", "m2", "stream-from-fallback"));

        let layer = ProviderFallbackLayer::new(
            make_entry("p1", primary, None),
            vec![make_entry("p2", fallback, None)],
        );

        let mut stream = layer
            .chat_stream(dummy_messages(), None, None, 1024, 0.0)
            .await
            .unwrap();

        // Collect stream events.
        let mut found_completed = false;
        while let Some(event) = stream.next().await {
            match event.unwrap() {
                LLMStreamEvent::Completed(resp) => {
                    assert_eq!(resp.content.unwrap(), "stream-from-fallback");
                    found_completed = true;
                }
                _ => {}
            }
        }
        assert!(found_completed);
    }

    // ── FallbackEntry builder tests ───────────────────────────────

    #[test]
    fn fallback_entry_builder() {
        let provider = Arc::new(MockProvider::always_ok("test", "m", "ok"));
        let entry = FallbackEntry::new("test-entry", provider.clone()).with_model("override-model");

        assert_eq!(entry.name, "test-entry");
        assert_eq!(entry.model, Some("override-model".to_string()));
    }

    #[test]
    fn fallback_entry_model_handling() {
        let provider = Arc::new(MockProvider::always_ok("test", "m", "ok"));

        let entry_no_model = FallbackEntry::new("e1", provider.clone());
        assert_eq!(entry_no_model.model, None);

        let entry_with_model = FallbackEntry::new("e2", provider).with_model("m1");
        assert_eq!(entry_with_model.model, Some("m1".to_string()));
    }
}
