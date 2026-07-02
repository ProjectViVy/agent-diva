//! Usage source tracking for token consumption attribution
//!
//! Provides types to record *where* tokens were consumed within the agent
//! lifecycle, enabling fine-grained cost attribution and observability.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::config::schema::TokenUsage;

/// Classification of which system component consumed tokens.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UsageSource {
    /// Main agent turn (the primary conversation loop)
    AgentTurn,
    /// Subagent execution (spawned child agents)
    Subagent,
    /// Built-in or custom tool invocation
    Tool,
    /// Context consolidation / compaction pass
    Consolidation,
    /// Background run (cron, scheduled job, etc.)
    BackgroundRun,
}

impl fmt::Display for UsageSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UsageSource::AgentTurn => write!(f, "agent_turn"),
            UsageSource::Subagent => write!(f, "subagent"),
            UsageSource::Tool => write!(f, "tool"),
            UsageSource::Consolidation => write!(f, "consolidation"),
            UsageSource::BackgroundRun => write!(f, "background_run"),
        }
    }
}

/// A single token-usage record with full attribution metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UsageRecord {
    /// Token counts (prompt / completion / total)
    pub usage: TokenUsage,
    /// Which component consumed the tokens
    pub source: UsageSource,
    /// Session identifier
    pub session_id: String,
    /// Turn identifier within the session
    pub turn_id: String,
    /// Optional run identifier (for background jobs, subagent batches, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    /// Optional subagent identifier (when source == Subagent)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent_id: Option<String>,
    /// Unix timestamp in milliseconds when the usage was recorded
    pub timestamp_ms: i64,
}

impl UsageRecord {
    /// Create a new usage record for an agent turn.
    pub fn new_agent_turn(
        session_id: impl Into<String>,
        turn_id: impl Into<String>,
        usage: TokenUsage,
        timestamp_ms: i64,
    ) -> Self {
        Self {
            usage,
            source: UsageSource::AgentTurn,
            session_id: session_id.into(),
            turn_id: turn_id.into(),
            run_id: None,
            subagent_id: None,
            timestamp_ms,
        }
    }

    /// Create a new usage record for a subagent.
    pub fn new_subagent(
        session_id: impl Into<String>,
        turn_id: impl Into<String>,
        subagent_id: impl Into<String>,
        usage: TokenUsage,
        timestamp_ms: i64,
    ) -> Self {
        Self {
            usage,
            source: UsageSource::Subagent,
            session_id: session_id.into(),
            turn_id: turn_id.into(),
            run_id: None,
            subagent_id: Some(subagent_id.into()),
            timestamp_ms,
        }
    }

    /// Create a new usage record for a tool call.
    pub fn new_tool(
        session_id: impl Into<String>,
        turn_id: impl Into<String>,
        usage: TokenUsage,
        timestamp_ms: i64,
    ) -> Self {
        Self {
            usage,
            source: UsageSource::Tool,
            session_id: session_id.into(),
            turn_id: turn_id.into(),
            run_id: None,
            subagent_id: None,
            timestamp_ms,
        }
    }

    /// Create a new usage record for a consolidation pass.
    pub fn new_consolidation(
        session_id: impl Into<String>,
        turn_id: impl Into<String>,
        usage: TokenUsage,
        timestamp_ms: i64,
    ) -> Self {
        Self {
            usage,
            source: UsageSource::Consolidation,
            session_id: session_id.into(),
            turn_id: turn_id.into(),
            run_id: None,
            subagent_id: None,
            timestamp_ms,
        }
    }

    /// Create a new usage record for a background run.
    pub fn new_background_run(
        session_id: impl Into<String>,
        turn_id: impl Into<String>,
        run_id: impl Into<String>,
        usage: TokenUsage,
        timestamp_ms: i64,
    ) -> Self {
        Self {
            usage,
            source: UsageSource::BackgroundRun,
            session_id: session_id.into(),
            turn_id: turn_id.into(),
            run_id: Some(run_id.into()),
            subagent_id: None,
            timestamp_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_usage() -> TokenUsage {
        TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
        }
    }

    #[test]
    fn usage_source_display() {
        assert_eq!(UsageSource::AgentTurn.to_string(), "agent_turn");
        assert_eq!(UsageSource::Subagent.to_string(), "subagent");
        assert_eq!(UsageSource::Tool.to_string(), "tool");
        assert_eq!(UsageSource::Consolidation.to_string(), "consolidation");
        assert_eq!(UsageSource::BackgroundRun.to_string(), "background_run");
    }

    #[test]
    fn usage_source_serde_roundtrip() {
        for source in [
            UsageSource::AgentTurn,
            UsageSource::Subagent,
            UsageSource::Tool,
            UsageSource::Consolidation,
            UsageSource::BackgroundRun,
        ] {
            let json = serde_json::to_string(&source).unwrap();
            let back: UsageSource = serde_json::from_str(&json).unwrap();
            assert_eq!(back, source, "roundtrip failed for {:?}", source);
        }
    }

    #[test]
    fn usage_record_serde_roundtrip() {
        let record =
            UsageRecord::new_agent_turn("sess-123", "turn-456", sample_usage(), 1_700_000_000_000);
        let json = serde_json::to_string(&record).unwrap();
        let back: UsageRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back, record);
    }

    #[test]
    fn usage_record_with_subagent_id() {
        let record = UsageRecord::new_subagent(
            "sess-123",
            "turn-456",
            "sub-789",
            sample_usage(),
            1_700_000_000_000,
        );
        assert_eq!(record.source, UsageSource::Subagent);
        assert_eq!(record.subagent_id, Some("sub-789".to_string()));
        assert_eq!(record.run_id, None);

        let json = serde_json::to_string(&record).unwrap();
        let back: UsageRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back, record);
    }

    #[test]
    fn usage_record_with_run_id() {
        let record = UsageRecord::new_background_run(
            "sess-123",
            "turn-456",
            "run-789",
            sample_usage(),
            1_700_000_000_000,
        );
        assert_eq!(record.source, UsageSource::BackgroundRun);
        assert_eq!(record.run_id, Some("run-789".to_string()));
        assert_eq!(record.subagent_id, None);

        let json = serde_json::to_string(&record).unwrap();
        let back: UsageRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back, record);
    }

    #[test]
    fn usage_record_optional_fields_omitted() {
        let record =
            UsageRecord::new_agent_turn("sess-123", "turn-456", sample_usage(), 1_700_000_000_000);
        let json = serde_json::to_string(&record).unwrap();
        // run_id and subagent_id should not appear when None
        assert!(!json.contains("run_id"));
        assert!(!json.contains("subagent_id"));
    }
}
