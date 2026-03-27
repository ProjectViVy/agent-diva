//! Stable memory contracts for future backends, retrieval, and tools.

use crate::types::{DiaryEntry, DiaryFilter, DiaryPartition, MemoryQuery, MemoryRecord};

pub trait MemoryStore: Send + Sync {
    fn store_record(&self, record: &MemoryRecord) -> agent_diva_core::Result<()>;
    fn get_record(&self, id: &str) -> agent_diva_core::Result<Option<MemoryRecord>>;
    fn list_records(&self) -> agent_diva_core::Result<Vec<MemoryRecord>>;
    fn forget_record(&self, id: &str) -> agent_diva_core::Result<bool>;
    fn recall(&self, query: &MemoryQuery) -> agent_diva_core::Result<Vec<MemoryRecord>>;
}

pub trait DiaryStore: Send + Sync {
    fn append_entry(&self, entry: &DiaryEntry) -> agent_diva_core::Result<()>;
    fn load_day(
        &self,
        date: &str,
        partition: DiaryPartition,
    ) -> agent_diva_core::Result<Vec<DiaryEntry>>;
    fn list_days(&self, partition: DiaryPartition) -> agent_diva_core::Result<Vec<String>>;
    fn filter_entries(&self, filter: &DiaryFilter) -> agent_diva_core::Result<Vec<DiaryEntry>>;
}

pub trait RecallEngine: Send + Sync {
    fn recall(&self, query: &MemoryQuery) -> agent_diva_core::Result<Vec<MemoryRecord>>;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct DiaryReadRequest {
    pub date: String,
    pub partition: DiaryPartition,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct DiaryToolReadResult {
    pub date: String,
    pub entries: Vec<DiaryEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct MemoryToolRecallResult {
    pub records: Vec<MemoryRecord>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct DiaryToolListResult {
    pub days: Vec<String>,
}

pub trait MemoryToolContract: Send + Sync {
    fn memory_recall(&self, query: &MemoryQuery)
        -> agent_diva_core::Result<MemoryToolRecallResult>;
}

pub trait DiaryToolContract: Send + Sync {
    fn diary_read(
        &self,
        request: &DiaryReadRequest,
    ) -> agent_diva_core::Result<DiaryToolReadResult>;
    fn diary_list(&self, filter: &DiaryFilter) -> agent_diva_core::Result<DiaryToolListResult>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MemoryDomain, MemoryScope, MemorySourceRef};
    use chrono::Utc;

    struct StubMemoryTool;

    impl MemoryToolContract for StubMemoryTool {
        fn memory_recall(
            &self,
            _query: &MemoryQuery,
        ) -> agent_diva_core::Result<MemoryToolRecallResult> {
            Ok(MemoryToolRecallResult {
                records: vec![MemoryRecord {
                    id: "rec-1".into(),
                    timestamp: Utc::now(),
                    domain: MemoryDomain::Workspace,
                    scope: MemoryScope::Workspace,
                    title: "Workspace".into(),
                    summary: "Summary".into(),
                    content: "Content".into(),
                    tags: vec!["workspace".into()],
                    source_refs: vec![MemorySourceRef {
                        path: Some("docs/README.md".into()),
                        section: None,
                        note: None,
                    }],
                    confidence: 0.8,
                }],
            })
        }
    }

    impl DiaryToolContract for StubMemoryTool {
        fn diary_read(
            &self,
            request: &DiaryReadRequest,
        ) -> agent_diva_core::Result<DiaryToolReadResult> {
            Ok(DiaryToolReadResult {
                date: request.date.clone(),
                entries: Vec::new(),
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

    #[test]
    fn test_contract_shapes_compile() {
        let tool = StubMemoryTool;
        let result = tool.memory_recall(&MemoryQuery::default()).unwrap();
        assert_eq!(result.records.len(), 1);

        let read = tool
            .diary_read(&DiaryReadRequest {
                date: "2026-03-26".into(),
                partition: DiaryPartition::Rational,
            })
            .unwrap();
        assert_eq!(read.date, "2026-03-26");
    }
}
