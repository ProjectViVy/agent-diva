//! Backfill and sync helpers for bridging file memory into SQLite.

use crate::compat_source::MemoryMdChunkSource;
use crate::contracts::{DiaryStore, MemoryStore};
use crate::diary::FileDiaryStore;
use crate::store::SqliteMemoryStore;
use crate::types::{DiaryEntry, DiaryPartition, MemoryRecord};
use std::path::Path;

const DIARY_PREFIX: &str = "diary:";
const COMPAT_PREFIX: &str = "compat:";

pub fn stored_diary_record(entry: &DiaryEntry) -> MemoryRecord {
    MemoryRecord {
        id: format!("{DIARY_PREFIX}{}", entry.id),
        timestamp: entry.timestamp,
        domain: entry.domain.clone(),
        scope: entry.scope.clone(),
        title: entry.title.clone(),
        summary: entry.summary.clone(),
        content: entry.body.clone(),
        tags: entry.tags.clone(),
        source_refs: entry.source_refs.clone(),
        confidence: entry.confidence,
    }
}

pub fn stored_compat_record(record: &MemoryRecord) -> MemoryRecord {
    let mut stored = record.clone();
    stored.id = format!("{COMPAT_PREFIX}{}", record.id);
    stored
}

pub fn sync_diary_entry_to_sqlite<P: AsRef<Path>>(
    workspace: P,
    entry: &DiaryEntry,
) -> agent_diva_core::Result<()> {
    let store = SqliteMemoryStore::new(workspace.as_ref())?;
    store.store_record(&stored_diary_record(entry))
}

pub fn backfill_workspace_sources<P: AsRef<Path>>(
    workspace: P,
    store: &SqliteMemoryStore,
) -> agent_diva_core::Result<()> {
    let workspace = workspace.as_ref();
    let diary_store = FileDiaryStore::new(workspace);
    for partition in [DiaryPartition::Rational, DiaryPartition::Emotional] {
        for day in diary_store.list_days(partition.clone())? {
            for entry in diary_store.load_day(&day, partition.clone())? {
                store.store_record(&stored_diary_record(&entry))?;
            }
        }
    }

    let compat_source = MemoryMdChunkSource::new(workspace);
    for record in compat_source.load_records()? {
        store.store_record(&stored_compat_record(&record))?;
    }

    Ok(())
}

pub(crate) fn canonical_record_key(record: &MemoryRecord) -> String {
    if record.id.starts_with(DIARY_PREFIX) || record.id.starts_with(COMPAT_PREFIX) {
        return record.id.clone();
    }

    if record
        .source_refs
        .iter()
        .any(|source| source.path.as_deref() == Some("memory/MEMORY.md"))
        || record.id.starts_with("compat-memory-md-")
    {
        return format!("{COMPAT_PREFIX}{}", record.id);
    }

    format!("{DIARY_PREFIX}{}", record.id)
}

pub(crate) fn fingerprint(record: &MemoryRecord) -> String {
    let path = record
        .source_refs
        .iter()
        .find_map(|source| source.path.as_deref())
        .unwrap_or("");
    format!(
        "{}|{}|{}",
        record.title.trim().to_lowercase(),
        record.summary.trim().to_lowercase(),
        path
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DiaryPartition, MemoryDomain, MemoryScope, MemorySourceRef};
    use chrono::{DateTime, Utc};
    use tempfile::TempDir;

    fn sample_entry() -> DiaryEntry {
        let mut entry = DiaryEntry::new(
            DiaryPartition::Rational,
            MemoryDomain::Workspace,
            MemoryScope::Workspace,
            "Architecture note",
            "Mapped the split",
            "The sqlite backfill should stay idempotent.",
        );
        entry.id = "entry-1".into();
        entry.timestamp = DateTime::parse_from_rfc3339("2026-03-27T08:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        entry.source_refs = vec![MemorySourceRef {
            path: Some("agent-diva-agent/src/diary.rs".into()),
            section: None,
            note: None,
        }];
        entry
    }

    #[test]
    fn test_stored_diary_record_prefixes_id() {
        let entry = sample_entry();
        let record = stored_diary_record(&entry);
        assert_eq!(record.id, "diary:entry-1");
    }

    #[test]
    fn test_backfill_workspace_sources_is_idempotent() {
        let temp = TempDir::new().unwrap();
        let diary_store = FileDiaryStore::new(temp.path());
        diary_store.append_entry(&sample_entry()).unwrap();

        std::fs::create_dir_all(temp.path().join("memory")).unwrap();
        std::fs::write(
            temp.path().join("memory").join("MEMORY.md"),
            "## Decisions\nKeep compatibility stable.\n",
        )
        .unwrap();

        let store = SqliteMemoryStore::new(temp.path()).unwrap();
        backfill_workspace_sources(temp.path(), &store).unwrap();
        backfill_workspace_sources(temp.path(), &store).unwrap();

        let records = store.list_records().unwrap();
        assert_eq!(records.len(), 2);
        assert!(records.iter().any(|record| record.id == "diary:entry-1"));
        assert!(records
            .iter()
            .any(|record| record.id.starts_with("compat:compat-memory-md-")));
    }
}
