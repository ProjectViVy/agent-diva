use super::TraceId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Structured runtime event written to the append-only JSONL observability log.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceEvent {
    pub ts: DateTime<Utc>,
    pub level: String,
    pub trace_id: TraceId,
    pub session_id: String,
    pub channel: String,
    pub component: String,
    pub event: String,
    pub summary: String,
    pub metadata: Value,
}

impl TraceEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        level: impl Into<String>,
        trace_id: TraceId,
        session_id: impl Into<String>,
        channel: impl Into<String>,
        component: impl Into<String>,
        event: impl Into<String>,
        summary: impl Into<String>,
        metadata: Value,
    ) -> Self {
        Self {
            ts: Utc::now(),
            level: level.into(),
            trace_id,
            session_id: session_id.into(),
            channel: channel.into(),
            component: component.into(),
            event: event.into(),
            summary: summary.into(),
            metadata,
        }
    }
}
