//! Memory tools backed by the enhanced memory contracts.

use crate::base::{Result, Tool, ToolError};
use agent_diva_memory::{
    DiaryFilter, DiaryPartition, DiaryReadRequest, DiaryToolContract, MemoryDomain,
    MemoryGetRequest, MemoryQuery, MemoryScope, MemoryToolContract, RecallMode,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::sync::Arc;

pub struct MemoryRecallTool {
    contract: Arc<dyn MemoryToolContract>,
}

impl MemoryRecallTool {
    pub fn new(contract: Arc<dyn MemoryToolContract>) -> Self {
        Self { contract }
    }
}

pub struct MemorySearchTool {
    contract: Arc<dyn MemoryToolContract>,
}

impl MemorySearchTool {
    pub fn new(contract: Arc<dyn MemoryToolContract>) -> Self {
        Self { contract }
    }
}

pub struct MemoryGetTool {
    contract: Arc<dyn MemoryToolContract>,
}

impl MemoryGetTool {
    pub fn new(contract: Arc<dyn MemoryToolContract>) -> Self {
        Self { contract }
    }
}

pub struct DiaryReadTool {
    contract: Arc<dyn DiaryToolContract>,
}

impl DiaryReadTool {
    pub fn new(contract: Arc<dyn DiaryToolContract>) -> Self {
        Self { contract }
    }
}

pub struct DiaryListTool {
    contract: Arc<dyn DiaryToolContract>,
}

impl DiaryListTool {
    pub fn new(contract: Arc<dyn DiaryToolContract>) -> Self {
        Self { contract }
    }
}

#[async_trait]
impl Tool for MemoryRecallTool {
    fn name(&self) -> &str {
        "memory_recall"
    }

    fn description(&self) -> &str {
        "Recall compact long-term memory and diary records for answer generation."
    }

    fn parameters(&self) -> Value {
        memory_query_parameters()
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let query = parse_memory_query(&args)?;
        let result = self
            .contract
            .memory_recall(&query)
            .map_err(map_contract_error)?;
        serialize_json(&result, "memory recall")
    }
}

#[async_trait]
impl Tool for MemorySearchTool {
    fn name(&self) -> &str {
        "memory_search"
    }

    fn description(&self) -> &str {
        "Search memory broadly and return snippets, timestamps, and source locations."
    }

    fn parameters(&self) -> Value {
        memory_query_parameters()
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let query = parse_memory_query(&args)?;
        let result = self
            .contract
            .memory_search(&query)
            .map_err(map_contract_error)?;
        serialize_json(&result, "memory search")
    }
}

#[async_trait]
impl Tool for MemoryGetTool {
    fn name(&self) -> &str {
        "memory_get"
    }

    fn description(&self) -> &str {
        "Fetch a full memory record or source fragment by record id or source path."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Memory record id." },
                "source_path": { "type": "string", "description": "Optional source path locator." }
            },
            "required": []
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let request = MemoryGetRequest {
            id: optional_string(&args, "id"),
            source_path: optional_string(&args, "source_path"),
        };
        let result = self
            .contract
            .memory_get(&request)
            .map_err(map_contract_error)?;
        serialize_json(&result, "memory get")
    }
}

#[async_trait]
impl Tool for DiaryReadTool {
    fn name(&self) -> &str {
        "diary_read"
    }

    fn description(&self) -> &str {
        "Read diary entries for a specific date and partition."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "date": {
                    "type": "string",
                    "description": "Diary day in YYYY-MM-DD format."
                },
                "partition": {
                    "type": "string",
                    "enum": diary_partition_values(),
                    "description": "Diary partition to read."
                }
            },
            "required": ["date", "partition"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let date = required_string(&args, "date")?;
        let partition = required_diary_partition(&args, "partition")?;
        let result = self
            .contract
            .diary_read(&DiaryReadRequest { date, partition })
            .map_err(map_contract_error)?;
        serialize_json(&result, "diary read")
    }
}

#[async_trait]
impl Tool for DiaryListTool {
    fn name(&self) -> &str {
        "diary_list"
    }

    fn description(&self) -> &str {
        "List diary days, optionally filtered by partition, domain, scope, time range, or tag."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "partition": { "type": "string", "enum": diary_partition_values() },
                "domain": { "type": "string", "enum": memory_domain_values() },
                "scope": { "type": "string", "enum": memory_scope_values() },
                "since": { "type": "string", "description": "Optional RFC3339 start time." },
                "until": { "type": "string", "description": "Optional RFC3339 end time." },
                "tag": { "type": "string", "description": "Optional tag filter." },
                "limit": { "type": "integer", "description": "Optional max number of matched entries before day aggregation." }
            },
            "required": []
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let filter = DiaryFilter {
            partition: optional_diary_partition(&args, "partition")?,
            domain: optional_memory_domain(&args, "domain")?,
            scope: optional_memory_scope(&args, "scope")?,
            since: optional_datetime(&args, "since")?,
            until: optional_datetime(&args, "until")?,
            tag: optional_string(&args, "tag"),
            limit: optional_usize(&args, "limit"),
        };
        let result = self
            .contract
            .diary_list(&filter)
            .map_err(map_contract_error)?;
        serialize_json(&result, "diary list")
    }
}

fn memory_query_parameters() -> Value {
    json!({
        "type": "object",
        "properties": {
            "query": { "type": "string", "description": "Optional search text." },
            "domain": { "type": "string", "enum": memory_domain_values() },
            "scope": { "type": "string", "enum": memory_scope_values() },
            "since": { "type": "string", "description": "Optional RFC3339 start time." },
            "until": { "type": "string", "description": "Optional RFC3339 end time." },
            "recall_mode": {
                "type": "string",
                "enum": ["keyword_only", "semantic_disabled", "hybrid_ready"],
                "description": "Optional internal recall mode override."
            },
            "limit": { "type": "integer", "description": "Max records to return. Defaults to 5." }
        },
        "required": []
    })
}

fn parse_memory_query(args: &Value) -> Result<MemoryQuery> {
    Ok(MemoryQuery {
        query: optional_string(args, "query"),
        domain: optional_memory_domain(args, "domain")?,
        scope: optional_memory_scope(args, "scope")?,
        since: optional_datetime(args, "since")?,
        until: optional_datetime(args, "until")?,
        recall_mode: optional_recall_mode(args, "recall_mode")?,
        limit: optional_usize(args, "limit").unwrap_or(5),
    })
}

fn serialize_json<T: serde::Serialize>(value: &T, label: &str) -> Result<String> {
    serde_json::to_string_pretty(value).map_err(|error| {
        ToolError::ExecutionFailed(format!("Failed to serialize {label} result: {error}"))
    })
}

fn required_string(args: &Value, key: &str) -> Result<String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| ToolError::InvalidArguments(format!("'{key}' is required")))
}

fn optional_string(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(Value::as_str).map(ToOwned::to_owned)
}

fn optional_usize(args: &Value, key: &str) -> Option<usize> {
    args.get(key)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn optional_datetime(args: &Value, key: &str) -> Result<Option<DateTime<Utc>>> {
    let Some(value) = args.get(key).and_then(Value::as_str) else {
        return Ok(None);
    };
    DateTime::parse_from_rfc3339(value)
        .map(|value| Some(value.with_timezone(&Utc)))
        .map_err(|error| {
            ToolError::InvalidArguments(format!("Invalid RFC3339 datetime for '{key}': {error}"))
        })
}

fn required_diary_partition(args: &Value, key: &str) -> Result<DiaryPartition> {
    optional_diary_partition(args, key)?
        .ok_or_else(|| ToolError::InvalidArguments(format!("'{key}' is required")))
}

fn optional_diary_partition(args: &Value, key: &str) -> Result<Option<DiaryPartition>> {
    match args.get(key).and_then(Value::as_str) {
        Some("rational") => Ok(Some(DiaryPartition::Rational)),
        Some("emotional") => Ok(Some(DiaryPartition::Emotional)),
        Some(other) => Err(ToolError::InvalidArguments(format!(
            "Invalid diary partition '{other}'"
        ))),
        None => Ok(None),
    }
}

fn optional_recall_mode(args: &Value, key: &str) -> Result<Option<RecallMode>> {
    match args.get(key).and_then(Value::as_str) {
        Some("keyword_only") => Ok(Some(RecallMode::KeywordOnly)),
        Some("semantic_disabled") => Ok(Some(RecallMode::SemanticDisabled)),
        Some("hybrid_ready") => Ok(Some(RecallMode::HybridReady)),
        Some(other) => Err(ToolError::InvalidArguments(format!(
            "Invalid recall mode '{other}'"
        ))),
        None => Ok(None),
    }
}

fn optional_memory_domain(args: &Value, key: &str) -> Result<Option<MemoryDomain>> {
    match args.get(key).and_then(Value::as_str) {
        Some("fact") => Ok(Some(MemoryDomain::Fact)),
        Some("event") => Ok(Some(MemoryDomain::Event)),
        Some("task") => Ok(Some(MemoryDomain::Task)),
        Some("workspace") => Ok(Some(MemoryDomain::Workspace)),
        Some("relationship") => Ok(Some(MemoryDomain::Relationship)),
        Some("self_model") => Ok(Some(MemoryDomain::SelfModel)),
        Some("diary_rational") => Ok(Some(MemoryDomain::DiaryRational)),
        Some("diary_emotional") => Ok(Some(MemoryDomain::DiaryEmotional)),
        Some("soul_signal") => Ok(Some(MemoryDomain::SoulSignal)),
        Some(other) => Err(ToolError::InvalidArguments(format!(
            "Invalid memory domain '{other}'"
        ))),
        None => Ok(None),
    }
}

fn optional_memory_scope(args: &Value, key: &str) -> Result<Option<MemoryScope>> {
    match args.get(key).and_then(Value::as_str) {
        Some("global") => Ok(Some(MemoryScope::Global)),
        Some("workspace") => Ok(Some(MemoryScope::Workspace)),
        Some("session") => Ok(Some(MemoryScope::Session)),
        Some("user") => Ok(Some(MemoryScope::User)),
        Some(other) => Err(ToolError::InvalidArguments(format!(
            "Invalid memory scope '{other}'"
        ))),
        None => Ok(None),
    }
}

fn map_contract_error(error: agent_diva_core::Error) -> ToolError {
    ToolError::ExecutionFailed(error.to_string())
}

fn diary_partition_values() -> Vec<&'static str> {
    vec!["rational", "emotional"]
}

fn memory_domain_values() -> Vec<&'static str> {
    vec![
        "fact",
        "event",
        "task",
        "workspace",
        "relationship",
        "self_model",
        "diary_rational",
        "diary_emotional",
        "soul_signal",
    ]
}

fn memory_scope_values() -> Vec<&'static str> {
    vec!["global", "workspace", "session", "user"]
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_memory::{
        DiaryEntry, DiaryToolListResult, DiaryToolReadResult, MemoryGetResult, MemoryRecord,
        MemorySearchResult, MemorySearchResultItem, MemorySourceRef,
    };
    use chrono::Utc;

    struct StubMemoryTools;

    impl MemoryToolContract for StubMemoryTools {
        fn memory_recall(
            &self,
            _query: &MemoryQuery,
        ) -> agent_diva_core::Result<agent_diva_memory::MemoryToolRecallResult> {
            Ok(agent_diva_memory::MemoryToolRecallResult {
                records: vec![MemoryRecord {
                    id: "rec-1".into(),
                    timestamp: Utc::now(),
                    domain: MemoryDomain::Workspace,
                    scope: MemoryScope::Workspace,
                    title: "Workspace".into(),
                    summary: "Summary".into(),
                    content: "Content".into(),
                    tags: vec!["workspace".into()],
                    source_refs: vec![MemorySourceRef::default()],
                    confidence: 0.8,
                }],
            })
        }

        fn memory_search(
            &self,
            _query: &MemoryQuery,
        ) -> agent_diva_core::Result<MemorySearchResult> {
            Ok(MemorySearchResult {
                results: vec![MemorySearchResultItem {
                    id: "rec-1".into(),
                    title: "Workspace".into(),
                    snippet: "Summary".into(),
                    timestamp: Utc::now(),
                    domain: MemoryDomain::Workspace,
                    scope: MemoryScope::Workspace,
                    source_refs: vec![MemorySourceRef::default()],
                }],
            })
        }

        fn memory_get(
            &self,
            _request: &MemoryGetRequest,
        ) -> agent_diva_core::Result<MemoryGetResult> {
            Ok(MemoryGetResult {
                record: None,
                source_fragment: Some("fragment".into()),
            })
        }
    }

    impl DiaryToolContract for StubMemoryTools {
        fn diary_read(
            &self,
            request: &DiaryReadRequest,
        ) -> agent_diva_core::Result<DiaryToolReadResult> {
            Ok(DiaryToolReadResult {
                date: request.date.clone(),
                entries: vec![DiaryEntry::new(
                    DiaryPartition::Rational,
                    MemoryDomain::Workspace,
                    MemoryScope::Workspace,
                    "Title",
                    "Summary",
                    "Body",
                )],
            })
        }

        fn diary_list(
            &self,
            _filter: &DiaryFilter,
        ) -> agent_diva_core::Result<DiaryToolListResult> {
            Ok(DiaryToolListResult {
                days: vec!["2026-03-26".into()],
            })
        }
    }

    #[tokio::test]
    async fn test_memory_recall_tool_returns_json() {
        let tool = MemoryRecallTool::new(Arc::new(StubMemoryTools));
        let result = tool
            .execute(json!({"query": "workspace", "limit": 3}))
            .await
            .unwrap();
        assert!(result.contains("\"records\""));
    }

    #[tokio::test]
    async fn test_memory_search_tool_returns_json() {
        let tool = MemorySearchTool::new(Arc::new(StubMemoryTools));
        let result = tool
            .execute(json!({"query": "workspace", "limit": 3}))
            .await
            .unwrap();
        assert!(result.contains("\"results\""));
    }

    #[tokio::test]
    async fn test_memory_get_tool_returns_json() {
        let tool = MemoryGetTool::new(Arc::new(StubMemoryTools));
        let result = tool.execute(json!({"id": "rec-1"})).await.unwrap();
        assert!(result.contains("fragment"));
    }

    #[tokio::test]
    async fn test_diary_read_tool_requires_partition() {
        let tool = DiaryReadTool::new(Arc::new(StubMemoryTools));
        let err = tool
            .execute(json!({"date": "2026-03-26"}))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("partition"));
    }

    #[tokio::test]
    async fn test_diary_list_tool_returns_days_json() {
        let tool = DiaryListTool::new(Arc::new(StubMemoryTools));
        let result = tool
            .execute(json!({"partition": "rational"}))
            .await
            .unwrap();
        assert!(result.contains("2026-03-26"));
    }
}
