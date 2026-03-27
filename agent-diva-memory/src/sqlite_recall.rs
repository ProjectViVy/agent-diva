//! SQLite-backed recall engine using the durable memory store.

use crate::contracts::{MemoryStore, RecallEngine};
use crate::store::SqliteMemoryStore;
use crate::types::{MemoryQuery, MemoryRecord};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct SqliteRecallEngine {
    store: Arc<SqliteMemoryStore>,
}

impl SqliteRecallEngine {
    pub fn new(store: Arc<SqliteMemoryStore>) -> Self {
        Self { store }
    }
}

impl RecallEngine for SqliteRecallEngine {
    fn recall(&self, query: &MemoryQuery) -> agent_diva_core::Result<Vec<MemoryRecord>> {
        self.store.recall(query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::MemoryStore;
    use crate::sync::stored_compat_record;
    use crate::types::{MemoryDomain, MemoryQuery, MemoryRecord, MemoryScope};
    use chrono::{DateTime, Utc};
    use std::sync::Arc;
    use tempfile::TempDir;

    fn sample_record() -> MemoryRecord {
        MemoryRecord {
            id: "rec-1".into(),
            timestamp: DateTime::parse_from_rfc3339("2026-03-27T10:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            domain: MemoryDomain::Workspace,
            scope: MemoryScope::Workspace,
            title: "Architecture split".into(),
            summary: "Keep memory core minimal".into(),
            content: "The sqlite recall engine should sit behind the workspace facade.".into(),
            tags: vec!["memory".into(), "sqlite".into()],
            source_refs: Vec::new(),
            confidence: 0.8,
        }
    }

    #[test]
    fn test_sqlite_recall_engine_returns_matches() {
        let temp = TempDir::new().unwrap();
        let store = Arc::new(SqliteMemoryStore::new(temp.path()).unwrap());
        store.store_record(&sample_record()).unwrap();
        let engine = SqliteRecallEngine::new(store);

        let results = engine
            .recall(&MemoryQuery {
                query: Some("sqlite recall".into()),
                limit: 5,
                ..MemoryQuery::default()
            })
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Architecture split");
    }

    #[test]
    fn test_sqlite_recall_engine_prefers_diary_over_compat() {
        let temp = TempDir::new().unwrap();
        let store = Arc::new(SqliteMemoryStore::new(temp.path()).unwrap());
        let mut diary = sample_record();
        diary.id = "diary:entry-1".into();
        diary.content = "Keep memory split notes in sqlite recall.".into();
        store.store_record(&diary).unwrap();

        let compat = stored_compat_record(&MemoryRecord {
            id: "compat-memory-md-1".into(),
            title: "Compatibility".into(),
            summary: "Keep compatibility layer stable".into(),
            content: "The compatibility layer should remain stable for MEMORY.md.".into(),
            ..sample_record()
        });
        store.store_record(&compat).unwrap();

        let engine = SqliteRecallEngine::new(store);
        let results = engine
            .recall(&MemoryQuery {
                query: Some("memory".into()),
                limit: 5,
                ..MemoryQuery::default()
            })
            .unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "diary:entry-1");
    }
}
