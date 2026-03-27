//! Stable domain types for the enhanced memory system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryDomain {
    Fact,
    Event,
    Task,
    Workspace,
    Relationship,
    SelfModel,
    DiaryRational,
    DiaryEmotional,
    SoulSignal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiaryPartition {
    Rational,
    Emotional,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryScope {
    Global,
    Workspace,
    Session,
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct MemorySourceRef {
    pub path: Option<String>,
    pub section: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryRecord {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub domain: MemoryDomain,
    pub scope: MemoryScope,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub tags: Vec<String>,
    pub source_refs: Vec<MemorySourceRef>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiaryEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub partition: DiaryPartition,
    pub domain: MemoryDomain,
    pub scope: MemoryScope,
    pub title: String,
    pub summary: String,
    pub body: String,
    pub tags: Vec<String>,
    pub source_refs: Vec<MemorySourceRef>,
    pub confidence: f32,
    pub observations: Vec<String>,
    pub confirmed: Vec<String>,
    pub unknowns: Vec<String>,
    pub next_steps: Vec<String>,
}

impl DiaryEntry {
    pub fn new(
        partition: DiaryPartition,
        domain: MemoryDomain,
        scope: MemoryScope,
        title: impl Into<String>,
        summary: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            partition,
            domain,
            scope,
            title: title.into(),
            summary: summary.into(),
            body: body.into(),
            tags: Vec::new(),
            source_refs: Vec::new(),
            confidence: 0.5,
            observations: Vec::new(),
            confirmed: Vec::new(),
            unknowns: Vec::new(),
            next_steps: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryQuery {
    pub query: Option<String>,
    pub domain: Option<MemoryDomain>,
    pub scope: Option<MemoryScope>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub recall_mode: Option<RecallMode>,
    pub limit: usize,
}

impl Default for MemoryQuery {
    fn default() -> Self {
        Self {
            query: None,
            domain: None,
            scope: None,
            since: None,
            until: None,
            recall_mode: None,
            limit: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecallMode {
    KeywordOnly,
    SemanticDisabled,
    HybridReady,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemorySearchResultItem {
    pub id: String,
    pub title: String,
    pub snippet: String,
    pub timestamp: DateTime<Utc>,
    pub domain: MemoryDomain,
    pub scope: MemoryScope,
    pub source_refs: Vec<MemorySourceRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemorySearchResult {
    pub results: Vec<MemorySearchResultItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct MemoryGetRequest {
    pub id: Option<String>,
    pub source_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryGetResult {
    pub record: Option<MemoryRecord>,
    pub source_fragment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiaryFilter {
    pub partition: Option<DiaryPartition>,
    pub domain: Option<MemoryDomain>,
    pub scope: Option<MemoryScope>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub tag: Option<String>,
    pub limit: Option<usize>,
}

impl DiaryFilter {
    pub fn rational_default() -> Self {
        Self {
            partition: Some(DiaryPartition::Rational),
            domain: None,
            scope: None,
            since: None,
            until: None,
            tag: None,
            limit: Some(20),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diary_entry_defaults() {
        let entry = DiaryEntry::new(
            DiaryPartition::Rational,
            MemoryDomain::Workspace,
            MemoryScope::Workspace,
            "Repo analysis",
            "Summary",
            "Body",
        );
        assert_eq!(entry.partition, DiaryPartition::Rational);
        assert_eq!(entry.domain, MemoryDomain::Workspace);
        assert!(!entry.id.is_empty());
    }

    #[test]
    fn test_rational_filter_defaults() {
        let filter = DiaryFilter::rational_default();
        assert_eq!(filter.partition, Some(DiaryPartition::Rational));
        assert_eq!(filter.limit, Some(20));
    }
}
