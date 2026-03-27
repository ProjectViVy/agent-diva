//! Workspace-scoped tool service adapter for the enhanced memory subsystem.

use crate::contracts::{
    DiaryReadRequest, DiaryStore, DiaryToolContract, DiaryToolListResult, DiaryToolReadResult,
    MemoryStore, MemoryToolContract, MemoryToolRecallResult, RecallEngine,
};
use crate::diary::FileDiaryStore;
use crate::recall::FileRecallEngine;
use crate::sqlite_recall::SqliteRecallEngine;
use crate::store::SqliteMemoryStore;
use crate::sync::{backfill_workspace_sources, canonical_record_key, fingerprint};
use crate::types::{DiaryEntry, DiaryFilter, DiaryPartition, MemoryQuery, MemoryRecord};
use chrono::Local;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct WorkspaceMemoryService {
    diary_store: FileDiaryStore,
    file_recall_engine: Arc<FileRecallEngine>,
    sqlite_recall_engine: Arc<SqliteRecallEngine>,
    memory_store: Arc<SqliteMemoryStore>,
}

impl WorkspaceMemoryService {
    pub const DEFAULT_RECALL_CONTEXT_LIMIT: usize = 3;

    pub fn new<P: AsRef<Path>>(workspace: P) -> Self {
        let workspace = workspace.as_ref();
        let diary_store = FileDiaryStore::new(workspace);
        let memory_store =
            Arc::new(SqliteMemoryStore::new(workspace).expect("sqlite memory store"));
        backfill_workspace_sources(workspace, &memory_store).expect("workspace memory backfill");
        Self {
            file_recall_engine: Arc::new(FileRecallEngine::for_workspace(workspace)),
            sqlite_recall_engine: Arc::new(SqliteRecallEngine::new(Arc::clone(&memory_store))),
            diary_store,
            memory_store,
        }
    }

    pub fn store_record(&self, record: &MemoryRecord) -> agent_diva_core::Result<()> {
        self.memory_store.store_record(record)
    }

    pub fn memory_store(&self) -> Arc<SqliteMemoryStore> {
        Arc::clone(&self.memory_store)
    }

    pub fn recall_records_for_context(
        &self,
        query: &str,
        limit: usize,
    ) -> agent_diva_core::Result<Vec<MemoryRecord>> {
        self.merged_recall(&MemoryQuery {
            query: Some(query.trim().to_string()),
            limit: limit.max(1),
            ..MemoryQuery::default()
        })
    }

    fn merged_recall(&self, query: &MemoryQuery) -> agent_diva_core::Result<Vec<MemoryRecord>> {
        let sqlite_records = self.sqlite_recall_engine.recall(query)?;
        let file_records = self.file_recall_engine.recall(query)?;
        Ok(merge_records(query, sqlite_records, file_records))
    }

    pub fn format_recall_context(records: &[MemoryRecord]) -> Option<String> {
        if records.is_empty() {
            return None;
        }

        let mut lines = vec![
            "## Auto-Recalled Memory".to_string(),
            "Use the following recalled memory only when it is relevant to the current user request.".to_string(),
        ];

        for record in records.iter().take(Self::DEFAULT_RECALL_CONTEXT_LIMIT) {
            let source = record
                .source_refs
                .iter()
                .find_map(|source| source.path.as_deref())
                .map(|path| format!("\nsource: {path}"))
                .unwrap_or_default();
            lines.push(format!(
                "- {} | {} | {:?}/{:?}\nsummary: {}{}",
                record
                    .timestamp
                    .with_timezone(&Local)
                    .format("%Y-%m-%d %H:%M"),
                record.title,
                record.domain,
                record.scope,
                record.summary.trim(),
                source,
            ));
        }

        Some(lines.join("\n"))
    }

    fn collect_matching_entries(
        &self,
        filter: &DiaryFilter,
    ) -> agent_diva_core::Result<Vec<DiaryEntry>> {
        match &filter.partition {
            Some(partition) => self.diary_store.filter_entries(&DiaryFilter {
                partition: Some(partition.clone()),
                domain: filter.domain.clone(),
                scope: filter.scope.clone(),
                since: filter.since,
                until: filter.until,
                tag: filter.tag.clone(),
                limit: filter.limit,
            }),
            None => {
                let mut entries = Vec::new();
                for partition in [DiaryPartition::Rational, DiaryPartition::Emotional] {
                    let scoped_filter = DiaryFilter {
                        partition: Some(partition),
                        domain: filter.domain.clone(),
                        scope: filter.scope.clone(),
                        since: filter.since,
                        until: filter.until,
                        tag: filter.tag.clone(),
                        limit: None,
                    };
                    entries.extend(self.diary_store.filter_entries(&scoped_filter)?);
                }
                entries.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
                if let Some(limit) = filter.limit {
                    entries.truncate(limit);
                }
                Ok(entries)
            }
        }
    }
}

impl MemoryToolContract for WorkspaceMemoryService {
    fn memory_recall(
        &self,
        query: &MemoryQuery,
    ) -> agent_diva_core::Result<MemoryToolRecallResult> {
        Ok(MemoryToolRecallResult {
            records: self.merged_recall(query)?,
        })
    }
}

fn merge_records(
    query: &MemoryQuery,
    sqlite_records: Vec<MemoryRecord>,
    file_records: Vec<MemoryRecord>,
) -> Vec<MemoryRecord> {
    let mut merged = Vec::new();
    let mut seen_ids = HashSet::new();
    let mut sqlite_fingerprints = HashSet::new();

    for record in sqlite_records {
        let key = canonical_record_key(&record);
        seen_ids.insert(key);
        sqlite_fingerprints.insert(fingerprint(&record));
        merged.push(record);
    }

    for record in file_records {
        let key = canonical_record_key(&record);
        if seen_ids.contains(&key) {
            continue;
        }
        if sqlite_fingerprints.contains(&fingerprint(&record)) {
            continue;
        }
        seen_ids.insert(key);
        merged.push(record);
    }

    let mut score_cache = HashMap::new();
    merged.sort_by(|left, right| {
        let right_score = *score_cache
            .entry(right.id.clone())
            .or_insert_with(|| record_score(right, query));
        let left_score = *score_cache
            .entry(left.id.clone())
            .or_insert_with(|| record_score(left, query));
        right_score
            .cmp(&left_score)
            .then(source_rank(right).cmp(&source_rank(left)))
            .then(right.timestamp.cmp(&left.timestamp))
            .then(left.id.cmp(&right.id))
    });
    merged.truncate(query.limit.max(1));
    merged
}

fn record_score(record: &MemoryRecord, query: &MemoryQuery) -> usize {
    let haystack = [
        record.title.to_lowercase(),
        record.summary.to_lowercase(),
        record.content.to_lowercase(),
        record.tags.join(" ").to_lowercase(),
        record
            .source_refs
            .iter()
            .filter_map(|source| source.path.as_deref())
            .collect::<Vec<_>>()
            .join("\n")
            .to_lowercase(),
    ]
    .join("\n");
    crate::recall::query_match_score(&haystack, query.query.as_deref()).unwrap_or(0)
}

fn source_rank(record: &MemoryRecord) -> u8 {
    if record.id.starts_with("diary:")
        || (!record.id.starts_with("compat:")
            && record
                .source_refs
                .iter()
                .all(|source| source.path.as_deref() != Some("memory/MEMORY.md")))
    {
        3
    } else if record.id.starts_with("compat:")
        || record.id.starts_with("compat-memory-md-")
        || record
            .source_refs
            .iter()
            .any(|source| source.path.as_deref() == Some("memory/MEMORY.md"))
    {
        1
    } else {
        2
    }
}

impl DiaryToolContract for WorkspaceMemoryService {
    fn diary_read(
        &self,
        request: &DiaryReadRequest,
    ) -> agent_diva_core::Result<DiaryToolReadResult> {
        Ok(DiaryToolReadResult {
            date: request.date.clone(),
            entries: self
                .diary_store
                .load_day(&request.date, request.partition.clone())?,
        })
    }

    fn diary_list(&self, filter: &DiaryFilter) -> agent_diva_core::Result<DiaryToolListResult> {
        let mut days = BTreeSet::new();
        for entry in self.collect_matching_entries(filter)? {
            days.insert(
                entry
                    .timestamp
                    .with_timezone(&Local)
                    .format("%Y-%m-%d")
                    .to_string(),
            );
        }

        Ok(DiaryToolListResult {
            days: days.into_iter().rev().collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{DiaryToolContract, MemoryToolContract};
    use crate::layout::brain_db_path;
    use crate::types::{MemoryDomain, MemoryScope, MemorySourceRef};
    use chrono::{DateTime, Utc};
    use tempfile::TempDir;

    fn sample_service() -> (TempDir, WorkspaceMemoryService) {
        let temp_dir = TempDir::new().unwrap();
        let service = WorkspaceMemoryService::new(temp_dir.path());

        let mut entry = DiaryEntry::new(
            DiaryPartition::Rational,
            MemoryDomain::Workspace,
            MemoryScope::Workspace,
            "Architecture note",
            "Mapped the memory split",
            "The `agent-diva-memory` crate now owns diary storage.",
        );
        entry.timestamp = DateTime::parse_from_rfc3339("2026-03-26T09:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        entry.tags = vec!["memory".into(), "architecture".into()];
        entry.source_refs = vec![MemorySourceRef {
            path: Some("agent-diva-memory/src/service.rs".into()),
            section: None,
            note: None,
        }];
        service.diary_store.append_entry(&entry).unwrap();

        std::fs::create_dir_all(temp_dir.path().join("memory")).unwrap();
        std::fs::write(
            temp_dir.path().join("memory").join("MEMORY.md"),
            "# Long-term Memory\n\nKeep the compatibility layer stable.\n",
        )
        .unwrap();

        (temp_dir, service)
    }

    #[test]
    fn test_memory_recall_includes_diary_matches() {
        let (_temp_dir, service) = sample_service();
        let result = service
            .memory_recall(&MemoryQuery {
                query: Some("architecture".into()),
                limit: 5,
                ..MemoryQuery::default()
            })
            .unwrap();
        assert_eq!(result.records.len(), 1);
        assert_eq!(result.records[0].title, "Architecture note");
    }

    #[test]
    fn test_memory_recall_can_fall_back_to_memory_md() {
        let (_temp_dir, service) = sample_service();
        let result = service
            .memory_recall(&MemoryQuery {
                query: Some("compatibility layer".into()),
                limit: 5,
                ..MemoryQuery::default()
            })
            .unwrap();
        assert_eq!(result.records.len(), 1);
        assert!(result.records[0].id.starts_with("compat-memory-md-"));
    }

    #[test]
    fn test_diary_list_deduplicates_days() {
        let (_temp_dir, service) = sample_service();
        let result = service
            .diary_list(&DiaryFilter::rational_default())
            .unwrap();
        assert_eq!(result.days, vec!["2026-03-26".to_string()]);
    }

    #[test]
    fn test_format_recall_context_limits_and_includes_metadata() {
        let (_temp_dir, service) = sample_service();
        let mut records = service.recall_records_for_context("memory", 5).unwrap();
        let mut compat = records[0].clone();
        compat.id = "compat-2".into();
        compat.title = "Compatibility".into();
        records.push(compat.clone());
        let mut extra = compat;
        extra.id = "compat-3".into();
        extra.title = "Extra".into();
        records.push(extra);

        let formatted = WorkspaceMemoryService::format_recall_context(&records).unwrap();
        assert!(formatted.contains("## Auto-Recalled Memory"));
        assert!(formatted.contains("Architecture note"));
        assert!(formatted.contains("summary: Mapped the memory split"));
        assert!(formatted.contains("source: agent-diva-memory/src/service.rs"));
        assert!(!formatted.contains("compat-3"));
        assert_eq!(formatted.matches("\n- ").count(), 3);
    }

    #[test]
    fn test_recall_records_for_context_respects_limit() {
        let (_temp_dir, service) = sample_service();
        let records = service.recall_records_for_context("layer", 1).unwrap();
        assert_eq!(records.len(), 1);
    }

    #[test]
    fn test_memory_recall_returns_multiple_memory_md_chunks() {
        let (_temp_dir, service) = sample_service();
        let result = service
            .memory_recall(&MemoryQuery {
                domain: Some(MemoryDomain::Fact),
                limit: 5,
                ..MemoryQuery::default()
            })
            .unwrap();
        assert!(result
            .records
            .iter()
            .any(|record| record.id.starts_with("compat-memory-md-")));
    }

    #[test]
    fn test_workspace_memory_service_initializes_sqlite_store() {
        let (temp_dir, service) = sample_service();
        assert!(brain_db_path(temp_dir.path()).exists());
        assert!(service.memory_store().list_records().unwrap().is_empty());
    }

    #[test]
    fn test_workspace_memory_service_backfills_existing_sources_on_new() {
        let temp_dir = TempDir::new().unwrap();
        let diary_store = FileDiaryStore::new(temp_dir.path());
        let mut entry = DiaryEntry::new(
            DiaryPartition::Rational,
            MemoryDomain::Workspace,
            MemoryScope::Workspace,
            "Architecture note",
            "Mapped the memory split",
            "The agent-diva-memory crate now owns diary storage.",
        );
        entry.id = "entry-1".into();
        diary_store.append_entry(&entry).unwrap();
        std::fs::create_dir_all(temp_dir.path().join("memory")).unwrap();
        std::fs::write(
            temp_dir.path().join("memory").join("MEMORY.md"),
            "## Decisions\nKeep compatibility stable.\n",
        )
        .unwrap();

        let service = WorkspaceMemoryService::new(temp_dir.path());
        let records = service.memory_store().list_records().unwrap();
        assert!(records.iter().any(|record| record.id == "diary:entry-1"));
        assert!(records
            .iter()
            .any(|record| record.id.starts_with("compat:compat-memory-md-")));
    }

    #[test]
    fn test_memory_recall_prefers_sqlite_and_deduplicates_file_results() {
        let temp_dir = TempDir::new().unwrap();
        let diary_store = FileDiaryStore::new(temp_dir.path());
        let mut entry = DiaryEntry::new(
            DiaryPartition::Rational,
            MemoryDomain::Workspace,
            MemoryScope::Workspace,
            "Architecture note",
            "Mapped the memory split",
            "The agent-diva-memory crate now owns diary storage.",
        );
        entry.id = "entry-1".into();
        diary_store.append_entry(&entry).unwrap();

        let service = WorkspaceMemoryService::new(temp_dir.path());
        let result = service
            .memory_recall(&MemoryQuery {
                query: Some("architecture".into()),
                limit: 5,
                ..MemoryQuery::default()
            })
            .unwrap();
        assert_eq!(result.records.len(), 1);
        assert_eq!(result.records[0].id, "diary:entry-1");
    }
}
