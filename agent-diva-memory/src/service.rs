//! Workspace-scoped tool service adapter for the enhanced memory subsystem.

use crate::contracts::{
    DiaryReadRequest, DiaryStore, DiaryToolContract, DiaryToolListResult, DiaryToolReadResult,
    MemoryStore, MemoryToolContract, MemoryToolRecallResult, RecallEngine,
};
use crate::diary::FileDiaryStore;
use crate::embeddings::{provider_from_config, EmbeddingProvider, EmbeddingProviderConfig};
use crate::layout::brain_db_path;
use crate::recall::FileRecallEngine;
use crate::retrieval::{
    CachedSemanticRetriever, DefaultHybridReranker, EmbeddingCacheStore, MergedKeywordRetriever,
    RetrievalEngine,
};
use crate::snapshot::{export_snapshot, hydrate_snapshot, snapshot_exists};
use crate::sqlite_recall::SqliteRecallEngine;
use crate::store::SqliteMemoryStore;
use crate::sync::backfill_workspace_sources;
use crate::types::{
    DiaryEntry, DiaryFilter, DiaryPartition, MemoryGetRequest, MemoryGetResult, MemoryQuery,
    MemoryRecord, MemorySearchResult, MemorySearchResultItem, RecallMode,
};
use chrono::Local;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct WorkspaceMemoryService {
    workspace: PathBuf,
    diary_store: FileDiaryStore,
    memory_store: Arc<SqliteMemoryStore>,
    retrieval_engine: RetrievalEngine,
}

impl WorkspaceMemoryService {
    pub const DEFAULT_RECALL_CONTEXT_LIMIT: usize = 3;

    pub fn new<P: AsRef<Path>>(workspace: P) -> Self {
        let workspace = workspace.as_ref();
        let diary_store = FileDiaryStore::new(workspace);
        let embedding_config = EmbeddingProviderConfig::from_env();
        let embedding_provider = provider_from_config(&embedding_config)
            .unwrap_or_else(|_| Box::new(crate::NoopEmbeddingProvider));
        let memory_store = Arc::new(
            initialize_memory_store(workspace).expect("sqlite memory store initialization"),
        );
        let file_recall_engine: Arc<dyn RecallEngine> =
            Arc::new(FileRecallEngine::for_workspace(workspace));
        let sqlite_recall_engine = Arc::new(SqliteRecallEngine::new(Arc::clone(&memory_store)));
        let embedding_provider: Arc<dyn EmbeddingProvider> = Arc::from(embedding_provider);
        let semantic = Arc::new(CachedSemanticRetriever::new(
            Arc::clone(&memory_store) as Arc<dyn EmbeddingCacheStore>,
            Arc::clone(&embedding_provider),
            embedding_config.clone(),
        ));
        let keyword = Arc::new(MergedKeywordRetriever::new(
            Arc::clone(&sqlite_recall_engine),
            file_recall_engine,
        ));
        let retrieval_engine =
            RetrievalEngine::new(keyword, Arc::new(DefaultHybridReranker::new(semantic)));

        let service = Self {
            workspace: workspace.to_path_buf(),
            diary_store,
            memory_store,
            retrieval_engine,
        };
        service
            .recover_and_backfill()
            .expect("workspace memory recovery");
        service
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
        self.retrieval_engine.recall(&MemoryQuery {
            query: Some(query.trim().to_string()),
            limit: limit.max(1),
            recall_mode: Some(RecallMode::HybridReady),
            ..MemoryQuery::default()
        })
    }

    fn recover_and_backfill(&self) -> agent_diva_core::Result<()> {
        let should_hydrate =
            !self.memory_store.is_healthy() || self.memory_store.is_empty().unwrap_or(true);

        if should_hydrate && snapshot_exists(&self.workspace) {
            for record in hydrate_snapshot(&self.workspace)? {
                self.memory_store.store_record(&record)?;
            }
        }

        backfill_workspace_sources(&self.workspace, &self.memory_store)?;

        if let Ok(records) = self.memory_store.list_records() {
            let _ = export_snapshot(&self.workspace, &records);
        }
        Ok(())
    }

    pub fn format_recall_context(records: &[MemoryRecord]) -> Option<String> {
        if records.is_empty() {
            return None;
        }

        let mut lines = vec![
            "## Auto-Recalled Memory".to_string(),
            "Use only the compact recalled memory below when it is relevant to the current user request.".to_string(),
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
            records: self.retrieval_engine.recall(query)?,
        })
    }

    fn memory_search(&self, query: &MemoryQuery) -> agent_diva_core::Result<MemorySearchResult> {
        let records = self.retrieval_engine.recall(query)?;
        Ok(MemorySearchResult {
            results: records
                .into_iter()
                .map(|record| {
                    let snippet = build_snippet(&record);
                    MemorySearchResultItem {
                        id: record.id,
                        title: record.title,
                        snippet,
                        timestamp: record.timestamp,
                        domain: record.domain,
                        scope: record.scope,
                        source_refs: record.source_refs,
                    }
                })
                .collect(),
        })
    }

    fn memory_get(&self, request: &MemoryGetRequest) -> agent_diva_core::Result<MemoryGetResult> {
        if let Some(id) = request.id.as_deref() {
            let record = self.memory_store.get_record(id)?;
            let source_fragment = record.as_ref().map(build_source_fragment);
            return Ok(MemoryGetResult {
                record,
                source_fragment,
            });
        }

        if let Some(source_path) = request.source_path.as_deref() {
            let record = self
                .memory_store
                .list_records()?
                .into_iter()
                .find(|record| {
                    record
                        .source_refs
                        .iter()
                        .any(|source| source.path.as_deref() == Some(source_path))
                });
            let source_fragment = record.as_ref().map(build_source_fragment);
            return Ok(MemoryGetResult {
                record,
                source_fragment,
            });
        }

        Ok(MemoryGetResult {
            record: None,
            source_fragment: None,
        })
    }
}

fn initialize_memory_store(workspace: &Path) -> agent_diva_core::Result<SqliteMemoryStore> {
    match SqliteMemoryStore::new(workspace) {
        Ok(store) if store.is_healthy() => Ok(store),
        Ok(_) => recreate_memory_store(workspace),
        Err(_) => recreate_memory_store(workspace),
    }
}

fn recreate_memory_store(workspace: &Path) -> agent_diva_core::Result<SqliteMemoryStore> {
    let db_path = brain_db_path(workspace);
    if db_path.exists() {
        let backup = db_path.with_extension(format!(
            "corrupt-{}",
            chrono::Utc::now().format("%Y%m%d%H%M%S")
        ));
        let _ = std::fs::rename(&db_path, backup);
    }
    SqliteMemoryStore::new(workspace)
}

fn build_snippet(record: &MemoryRecord) -> String {
    let body = if record.summary.trim().is_empty() {
        record.content.trim()
    } else {
        record.summary.trim()
    };
    let mut snippet = body.chars().take(160).collect::<String>();
    if body.chars().count() > 160 {
        snippet.push_str("...");
    }
    snippet
}

fn build_source_fragment(record: &MemoryRecord) -> String {
    let source = record
        .source_refs
        .iter()
        .find_map(|source| source.path.as_deref())
        .unwrap_or("unknown");
    format!("source: {source}\n\n{}", record.content.trim())
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
    use crate::layout::{brain_db_path, snapshot_path};
    use crate::types::{MemoryDomain, MemoryGetRequest, MemoryScope, MemorySourceRef};
    use chrono::{DateTime, Utc};
    use tempfile::TempDir;

    fn sample_service() -> (TempDir, WorkspaceMemoryService) {
        let temp_dir = TempDir::new().unwrap();
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
        let diary_store = FileDiaryStore::new(temp_dir.path());
        diary_store.append_entry(&entry).unwrap();

        std::fs::create_dir_all(temp_dir.path().join("memory")).unwrap();
        std::fs::write(
            temp_dir.path().join("memory").join("MEMORY.md"),
            "# Long-term Memory\n\nKeep the compatibility layer stable.\n",
        )
        .unwrap();

        let service = WorkspaceMemoryService::new(temp_dir.path());
        (temp_dir, service)
    }

    fn structured_signal_service() -> (TempDir, WorkspaceMemoryService) {
        let temp_dir = TempDir::new().unwrap();
        let mut entry = DiaryEntry::new(
            DiaryPartition::Rational,
            MemoryDomain::Workspace,
            MemoryScope::Workspace,
            "协作规则整理",
            "用户希望回复保持中文、简洁，并要求先说明动作再改文件。",
            r#"
## 结论
- 已确认：用户偏好中文回复，并希望答案尽量简洁直接。
- 已确认：所有回复必须以前缀 [I strictly follow the rules] 开头。
- 已确认：这个 agent 应该先说明动作再改文件，不要自作主张提交代码。
"#,
        );
        entry.timestamp = DateTime::parse_from_rfc3339("2026-03-27T09:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        entry.confirmed = vec![
            "用户偏好中文回复，并希望答案尽量简洁直接。".into(),
            "所有回复必须以前缀 [I strictly follow the rules] 开头。".into(),
            "这个 agent 应该先说明动作再改文件，不要自作主张提交代码。".into(),
        ];
        FileDiaryStore::new(temp_dir.path())
            .append_entry(&entry)
            .unwrap();

        let service = WorkspaceMemoryService::new(temp_dir.path());
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
        assert!(!result.records.is_empty());
        assert!(result
            .records
            .iter()
            .any(|record| record.title == "Architecture note"));
    }

    #[test]
    fn test_memory_recall_can_fall_back_to_memory_md() {
        let (_temp_dir, service) = sample_service();
        let result = service
            .memory_recall(&MemoryQuery {
                query: Some("compatibility layer".into()),
                recall_mode: Some(RecallMode::KeywordOnly),
                limit: 5,
                ..MemoryQuery::default()
            })
            .unwrap();
        assert!(result
            .records
            .iter()
            .any(|record| record.id.starts_with("compat")));
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
        assert!(formatted.contains("source: agent-diva-memory/src/service.rs"));
        assert_eq!(formatted.matches("\n- ").count(), 3);
    }

    #[test]
    fn test_recall_records_for_context_respects_limit() {
        let (_temp_dir, service) = sample_service();
        let records = service.recall_records_for_context("layer", 1).unwrap();
        assert_eq!(records.len(), 1);
    }

    #[test]
    fn test_workspace_memory_service_initializes_sqlite_store_and_snapshot() {
        let (temp_dir, service) = sample_service();
        assert!(brain_db_path(temp_dir.path()).exists());
        assert!(snapshot_path(temp_dir.path()).exists());
        assert!(!service.memory_store().list_records().unwrap().is_empty());
    }

    #[test]
    fn test_workspace_memory_service_hydrates_from_snapshot_after_db_loss() {
        let (temp_dir, service) = sample_service();
        let records_before = service.memory_store().list_records().unwrap();
        std::fs::remove_file(brain_db_path(temp_dir.path())).unwrap();
        let rebuilt = WorkspaceMemoryService::new(temp_dir.path());
        let records_after = rebuilt.memory_store().list_records().unwrap();
        assert!(!records_after.is_empty());
        assert!(records_after.len() >= records_before.len());
    }

    #[test]
    fn test_workspace_memory_service_recovers_from_corrupt_db() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir_all(temp_dir.path().join("memory")).unwrap();
        std::fs::write(brain_db_path(temp_dir.path()), "not-a-sqlite-db").unwrap();
        std::fs::write(
            temp_dir.path().join("memory").join("MEMORY.md"),
            "## Decisions\nKeep compatibility stable.\n",
        )
        .unwrap();
        let service = WorkspaceMemoryService::new(temp_dir.path());
        assert!(service.memory_store().is_healthy());
        assert!(!service.memory_store().list_records().unwrap().is_empty());
    }

    #[test]
    fn test_memory_search_returns_snippets() {
        let (_temp_dir, service) = sample_service();
        let result = service
            .memory_search(&MemoryQuery {
                query: Some("architecture".into()),
                limit: 5,
                ..MemoryQuery::default()
            })
            .unwrap();
        assert!(!result.results.is_empty());
        assert!(result
            .results
            .iter()
            .any(|item| !item.snippet.trim().is_empty()));
    }

    #[test]
    fn test_memory_recall_returns_derived_relationship_record() {
        let (_temp_dir, service) = structured_signal_service();
        let result = service
            .memory_recall(&MemoryQuery {
                query: Some("用户偏好 中文 简洁".into()),
                domain: Some(MemoryDomain::Relationship),
                limit: 5,
                ..MemoryQuery::default()
            })
            .unwrap();
        assert!(!result.records.is_empty());
        assert_eq!(result.records[0].domain, MemoryDomain::Relationship);
        assert!(result.records[0].id.starts_with("derived:relationship:"));
    }

    #[test]
    fn test_memory_search_returns_derived_soul_signal_record() {
        let (_temp_dir, service) = structured_signal_service();
        let result = service
            .memory_search(&MemoryQuery {
                query: Some("identity-signal prefix".into()),
                domain: Some(MemoryDomain::SoulSignal),
                limit: 5,
                ..MemoryQuery::default()
            })
            .unwrap();
        assert!(!result.results.is_empty());
        assert_eq!(result.results[0].domain, MemoryDomain::SoulSignal);
        assert!(result.results[0].id.starts_with("derived:soul_signal:"));
    }

    #[test]
    fn test_memory_get_returns_full_record_and_fragment() {
        let (_temp_dir, service) = sample_service();
        let search = service
            .memory_search(&MemoryQuery {
                query: Some("architecture".into()),
                limit: 1,
                ..MemoryQuery::default()
            })
            .unwrap();
        let result = service
            .memory_get(&MemoryGetRequest {
                id: Some(search.results[0].id.clone()),
                source_path: None,
            })
            .unwrap();
        assert!(result.record.is_some());
        assert!(result.source_fragment.unwrap().contains("source:"));
    }

    #[test]
    fn test_memory_recall_keyword_only_mode_still_works() {
        let (_temp_dir, service) = sample_service();
        let result = service
            .memory_recall(&MemoryQuery {
                query: Some("memory split".into()),
                recall_mode: Some(RecallMode::KeywordOnly),
                limit: 2,
                ..MemoryQuery::default()
            })
            .unwrap();
        assert!(!result.records.is_empty());
    }
}
