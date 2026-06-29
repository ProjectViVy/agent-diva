//! Summary data model with pointer chain support for LLM summary compaction.
//!
//! This module provides the core data structures for building a chain of
//! conversation summaries that can be traversed backwards, enabling
//! hierarchical compaction of long-running agent sessions.

use agent_diva_providers::{LLMProvider, Message, ProviderError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::time::sleep;
use uuid::Uuid;

/// A compressed summary of a conversation segment.
///
/// Each summary carries a pointer to its predecessor (`prev_summary_id`),
/// forming a singly-linked chain from the most recent summary back to the
/// oldest. The `source_message_range` records which messages in the original
/// conversation this summary was derived from.
#[derive(Clone, Serialize, Deserialize)]
pub struct Summary {
    /// Unique identifier for this summary.
    pub id: String,
    /// The summary text content.
    pub content: String,
    /// When this summary was created.
    pub created_at: DateTime<Utc>,
    /// Pointer to the previous (older) summary in the chain, if any.
    pub prev_summary_id: Option<String>,
    /// The inclusive range of source messages this summary covers
    /// `(start_index, end_index)`.
    pub source_message_range: (usize, usize),
    /// Estimated token count of this summary.
    pub token_count: u32,
}

impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Summary(id={}, created_at={}, tokens={})",
            self.id, self.created_at, self.token_count
        )
    }
}

impl fmt::Debug for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let preview: &str = if self.content.len() > 50 {
            &self.content[..50]
        } else {
            self.content.as_str()
        };
        f.debug_struct("Summary")
            .field("id", &self.id)
            .field("content", &preview)
            .field("created_at", &self.created_at)
            .field("prev_summary_id", &self.prev_summary_id)
            .field("source_message_range", &self.source_message_range)
            .field("token_count", &self.token_count)
            .finish()
    }
}

/// A doubly-linked chain of summaries anchored at the most recent entry.
///
/// New summaries are appended to the front (becoming the new `latest`), and
/// their `prev_summary_id` is automatically set to the previous latest.
/// The chain can be traversed from latest backwards via an iterator.
#[derive(Clone, Serialize, Deserialize)]
pub struct SummaryChain {
    summaries: Vec<Summary>,
    latest_id: Option<String>,
}

impl Default for SummaryChain {
    fn default() -> Self {
        Self {
            summaries: Vec::new(),
            latest_id: None,
        }
    }
}

impl SummaryChain {
    /// Create a new empty summary chain.
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a new summary onto the chain.
    ///
    /// The summary's `prev_summary_id` is automatically set to the previous
    /// latest entry (if any), linking the new summary as the head of the chain.
    pub fn push(&mut self, mut summary: Summary) {
        summary.prev_summary_id = self.latest_id.clone();
        self.latest_id = Some(summary.id.clone());
        self.summaries.push(summary);
    }

    /// Return the number of summaries in the chain.
    pub fn depth(&self) -> usize {
        self.summaries.len()
    }

    /// Return a reference to the most recent summary, if any.
    pub fn latest(&self) -> Option<&Summary> {
        self.latest_id
            .as_ref()
            .and_then(|id| self.get(id))
    }

    /// Look up a summary by its id.
    pub fn get(&self, id: &str) -> Option<&Summary> {
        self.summaries.iter().find(|s| s.id == id)
    }

    /// Return an iterator that traverses from the latest summary backwards
    /// through the `prev_summary_id` chain.
    pub fn iter_from_latest(&self) -> SummaryChainIter<'_> {
        SummaryChainIter {
            chain: self,
            current_id: self.latest_id.clone(),
        }
    }
}

impl fmt::Display for SummaryChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SummaryChain(depth={}, latest={:?})",
            self.depth(),
            self.latest_id
        )
    }
}

impl fmt::Debug for SummaryChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SummaryChain")
            .field("depth", &self.depth())
            .field("latest_id", &self.latest_id)
            .finish()
    }
}

/// An iterator that walks the summary chain backwards from the latest entry.
///
/// Each call to `next()` returns the current summary and advances to the
/// previous summary via `prev_summary_id`.
pub struct SummaryChainIter<'a> {
    chain: &'a SummaryChain,
    current_id: Option<String>,
}

impl<'a> Iterator for SummaryChainIter<'a> {
    type Item = &'a Summary;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.current_id.as_ref()?;
        let summary = self.chain.get(id)?;
        self.current_id = summary.prev_summary_id.clone();
        Some(summary)
    }
}

/// Errors that can occur during summary generation.
#[derive(Error, Debug)]
pub enum SummaryError {
    /// An error from the underlying LLM provider.
    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),

    /// The message list was empty.
    #[error("Cannot summarize empty messages")]
    EmptyMessages,

    /// The generated summary failed quality checks.
    #[error("Summary quality rejected: {reason}")]
    QualityRejected { reason: String },
}

/// LLM-powered summary engine for conversation compaction.
///
/// Wraps an [`LLMProvider`] to generate concise summaries of conversation
/// messages. Rate-limited errors are automatically retried up to
/// `max_retries` times with respect for server-suggested backoff durations.
pub struct SummaryEngine {
    provider: Arc<dyn LLMProvider>,
    max_retries: u32,
}

impl SummaryEngine {
    /// Create a new summary engine backed by the given LLM provider.
    ///
    /// By default, up to 3 retries are attempted on rate-limited responses.
    pub fn new(provider: Arc<dyn LLMProvider>) -> Self {
        Self {
            provider,
            max_retries: 3,
        }
    }

    /// Generate a summary of the given conversation messages.
    ///
    /// Returns `Ok(None)` when the message slice is empty. For a single
    /// message the content is used directly without calling the LLM.
    ///
    /// Rate-limited errors from the provider are retried up to
    /// `max_retries` times, respecting the server's `retry_after`
    /// duration when present.
    pub async fn summarize(&self, messages: &[Message]) -> Result<Option<Summary>, SummaryError> {
        if messages.is_empty() {
            return Ok(None);
        }

        let content = if messages.len() == 1 {
            messages[0].content.to_text_lossy()
        } else {
            let prompt = build_summarization_prompt(messages);
            call_with_retry(&self.provider, prompt, self.max_retries).await?
        };

        let content = content.trim().to_string();
        if content.is_empty() {
            return Ok(None);
        }

        // Rough token estimate matching the heuristic in `context_budget`.
        let token_count = ((content.chars().count() / 4).max(1) + 2) as u32;

        Ok(Some(Summary {
            id: Uuid::new_v4().to_string(),
            content,
            created_at: Utc::now(),
            prev_summary_id: None,
            source_message_range: (0, messages.len().saturating_sub(1)),
            token_count,
        }))
    }
}

/// Build an LLM prompt that asks the model to summarise the conversation.
fn build_summarization_prompt(messages: &[Message]) -> Vec<Message> {
    let conversation_text: String = messages
        .iter()
        .map(|m| format!("{}: {}", m.role, m.content.to_text_lossy()))
        .collect::<Vec<_>>()
        .join("\n");

    vec![
        Message::system("You are a helpful assistant that summarizes conversations concisely."),
        Message::user(format!(
            "Summarize the following conversation concisely, capturing the key \
             information, decisions, and context:\n\n{}",
            conversation_text
        )),
    ]
}

/// Call the LLM provider's chat endpoint with retry logic.
///
/// Rate-limited responses are retried; all other non-retryable errors
/// are propagated immediately. At most `max_retries + 1` attempts are
/// made.
async fn call_with_retry(
    provider: &Arc<dyn LLMProvider>,
    messages: Vec<Message>,
    max_retries: u32,
) -> Result<String, SummaryError> {
    for attempt in 0..=max_retries {
        match provider
            .chat(messages.clone(), None, None, 1024, 0.3)
            .await
        {
            Ok(response) => return Ok(response.content.unwrap_or_default()),
            Err(ProviderError::RateLimited { retry_after }) => {
                if attempt < max_retries {
                    let delay = retry_after.unwrap_or(Duration::from_secs(2));
                    sleep(delay).await;
                    continue;
                }
                return Err(SummaryError::Provider(ProviderError::RateLimited { retry_after }));
            }
            Err(e) => {
                if attempt < max_retries && e.is_retryable() {
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
                return Err(SummaryError::Provider(e));
            }
        }
    }

    Err(SummaryError::Provider(ProviderError::Permanent {
        message: "max retries exceeded".to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_providers::{LLMResponse, ProviderResult};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    // ── Mock provider ────────────────────────────────────────────────────

    /// Behaviour variant that a [`MockProvider`] should exhibit.
    enum MockBehavior {
        /// Always succeed on the first call.
        Success,
        /// Fail with [`ProviderError::RateLimited`] for the first two calls,
        /// then succeed on the third.
        RateLimitedThenSuccess,
        /// Always fail with [`ProviderError::Permanent`].
        PermanentError,
    }

    /// A minimal [`LLMProvider`] implementation for unit testing.
    struct MockProvider {
        behavior: MockBehavior,
        call_count: AtomicU32,
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
            let count = self.call_count.fetch_add(1, Ordering::SeqCst) + 1;

            match self.behavior {
                MockBehavior::Success => Ok(LLMResponse {
                    content: Some("This is a concise summary of the conversation.".to_string()),
                    tool_calls: vec![],
                    finish_reason: "stop".to_string(),
                    usage: None,
                    reasoning_content: None,
                }),
                MockBehavior::RateLimitedThenSuccess => {
                    if count <= 2 {
                        Err(ProviderError::RateLimited { retry_after: None })
                    } else {
                        Ok(LLMResponse {
                            content: Some("Summary after retry.".to_string()),
                            tool_calls: vec![],
                            finish_reason: "stop".to_string(),
                            usage: None,
                            reasoning_content: None,
                        })
                    }
                }
                MockBehavior::PermanentError => Err(ProviderError::Permanent {
                    message: "test provider error".to_string(),
                }),
            }
        }

        fn get_default_model(&self) -> String {
            "mock".to_string()
        }
    }

    // ── Helper ───────────────────────────────────────────────────────────

    fn make_mock_engine(behavior: MockBehavior) -> SummaryEngine {
        SummaryEngine {
            provider: Arc::new(MockProvider {
                behavior,
                call_count: AtomicU32::new(0),
            }),
            max_retries: 3,
        }
    }

    // ── Success path ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_summarize_success() {
        let engine = make_mock_engine(MockBehavior::Success);

        let messages = vec![
            Message::user("Hello, how are you?"),
            Message::assistant("I'm doing well, thank you!"),
        ];

        let result = engine.summarize(&messages).await.unwrap();
        assert!(result.is_some());
        let summary = result.unwrap();
        assert!(summary.content.contains("summary"));
        assert!(summary.token_count > 0);
        assert_eq!(summary.source_message_range, (0, 1));
    }

    // ── Edge cases ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_summarize_empty_messages() {
        let engine = make_mock_engine(MockBehavior::Success);

        let result = engine.summarize(&[]).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_summarize_single_message_uses_content_directly() {
        let engine = make_mock_engine(MockBehavior::Success);

        let messages = vec![Message::user("Just one message here.")];
        let result = engine.summarize(&messages).await.unwrap();

        // Single message path skips the LLM call entirely.
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().content,
            "Just one message here."
        );
    }

    // ── Retry behaviour ──────────────────────────────────────────────────

    #[tokio::test]
    async fn test_summarize_rate_limited_retry() {
        let engine = make_mock_engine(MockBehavior::RateLimitedThenSuccess);

        let messages = vec![
            Message::user("Hello?"),
            Message::assistant("Hi there!"),
        ];

        let result = engine.summarize(&messages).await.unwrap();
        assert!(result.is_some());
        let summary = result.unwrap();
        assert!(summary.content.contains("retry"));
    }

    // ── Provider error propagation ───────────────────────────────────────

    #[tokio::test]
    async fn test_summarize_provider_error() {
        let engine = make_mock_engine(MockBehavior::PermanentError);

        let messages = vec![
            Message::user("Hello?"),
            Message::assistant("Hi there!"),
        ];

        let err = engine.summarize(&messages).await.unwrap_err();
        assert!(
            matches!(err, SummaryError::Provider(_)),
            "Expected Provider error, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn test_summarize_provider_error_display() {
        let err = SummaryError::Provider(ProviderError::Permanent {
            message: "test permanent error".to_string(),
        });
        let display = err.to_string();
        assert!(display.contains("Provider error"));
        assert!(display.contains("test permanent error"));
    }

    #[tokio::test]
    async fn test_summary_error_from_provider_error() {
        let provider_err = ProviderError::Permanent {
            message: "converted".to_string(),
        };
        let summary_err: SummaryError = provider_err.into();
        assert!(matches!(summary_err, SummaryError::Provider(_)));
    }

    fn make_summary(id: &str, token_count: u32, range: (usize, usize)) -> Summary {
        Summary {
            id: id.to_string(),
            content: format!("Summary content for {}", id),
            created_at: Utc::now(),
            prev_summary_id: None,
            source_message_range: range,
            token_count,
        }
    }

    #[test]
    fn test_summary_push_and_chain() {
        let mut chain = SummaryChain::new();

        let s1 = make_summary("s1", 100, (0, 10));
        chain.push(s1);

        assert_eq!(chain.depth(), 1);
        assert_eq!(chain.latest().unwrap().id, "s1");
        assert!(chain.latest().unwrap().prev_summary_id.is_none());

        let s2 = make_summary("s2", 150, (11, 25));
        chain.push(s2);

        assert_eq!(chain.depth(), 2);
        assert_eq!(chain.latest().unwrap().id, "s2");
        assert_eq!(
            chain.latest().unwrap().prev_summary_id.as_deref(),
            Some("s1")
        );

        // Verify backward traversal via pointer chain
        let s2_ref = chain.get("s2").unwrap();
        let s1_ref = chain.get(s2_ref.prev_summary_id.as_ref().unwrap()).unwrap();
        assert_eq!(s1_ref.id, "s1");
        assert_eq!(s1_ref.token_count, 100);
        assert_eq!(s1_ref.source_message_range, (0, 10));
    }

    #[test]
    fn test_summary_depth() {
        let mut chain = SummaryChain::new();
        assert_eq!(chain.depth(), 0);

        chain.push(make_summary("a", 50, (0, 5)));
        assert_eq!(chain.depth(), 1);

        chain.push(make_summary("b", 60, (6, 10)));
        assert_eq!(chain.depth(), 2);

        chain.push(make_summary("c", 70, (11, 15)));
        assert_eq!(chain.depth(), 3);
    }

    #[test]
    fn test_summary_serde_roundtrip() {
        let mut chain = SummaryChain::new();
        chain.push(make_summary("s1", 120, (0, 20)));
        chain.push(make_summary("s2", 80, (21, 30)));

        let json = serde_json::to_string(&chain).unwrap();
        let deserialized: SummaryChain = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.depth(), 2);
        assert_eq!(
            deserialized.latest().unwrap().prev_summary_id.as_deref(),
            Some("s1")
        );
        assert_eq!(deserialized.latest().unwrap().token_count, 80);

        // Verify the summary content survived roundtrip
        let s1 = deserialized.get("s1").unwrap();
        assert_eq!(s1.token_count, 120);
        assert_eq!(s1.source_message_range, (0, 20));
    }

    #[test]
    fn test_summary_chain_empty() {
        let chain = SummaryChain::new();
        assert_eq!(chain.depth(), 0);
        assert!(chain.latest().is_none());
        assert!(chain.get("nonexistent").is_none());
        assert!(chain.iter_from_latest().next().is_none());
    }

    #[test]
    fn test_summary_chain_iter_from_latest() {
        let mut chain = SummaryChain::new();
        chain.push(make_summary("s1", 100, (0, 10)));
        chain.push(make_summary("s2", 150, (11, 25)));
        chain.push(make_summary("s3", 200, (26, 40)));

        let ids: Vec<&str> = chain.iter_from_latest().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, vec!["s3", "s2", "s1"]);
    }

    #[test]
    fn test_summary_display() {
        let summary = make_summary("test-1", 75, (0, 10));
        let display = format!("{}", summary);
        assert!(display.contains("test-1"));
        assert!(display.contains("75"));
    }
}
