//! SQLite-backed durable memory store foundation with FTS support.

use crate::contracts::MemoryStore;
use crate::layout::brain_db_path;
use crate::recall::{normalized_query_terms, query_match_score};
use crate::types::{MemoryQuery, MemoryRecord, MemorySourceRef};
use agent_diva_core::{Error, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, params_from_iter, Connection, OptionalExtension, ToSql};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const SCHEMA_VERSION: i64 = 2;

#[derive(Debug, Clone)]
pub struct SqliteMemoryStore {
    conn: Arc<Mutex<Connection>>,
    db_path: PathBuf,
}

impl SqliteMemoryStore {
    pub fn new(workspace: &Path) -> Result<Self> {
        let db_path = brain_db_path(workspace);
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path).map_err(sqlite_error)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;",
        )
        .map_err(sqlite_error)?;
        init_schema(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path,
        })
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    fn all_candidates(&self, query: &MemoryQuery) -> Result<Vec<ScoredRecord>> {
        let mut records = self.list_records()?;
        records.retain(|record| matches_record_filters(record, query));
        Ok(records
            .into_iter()
            .filter_map(|record| {
                let score = score_record(&record, query)?;
                Some(ScoredRecord {
                    source_rank: source_rank_for_id(&record.id),
                    record,
                    score,
                })
            })
            .collect())
    }

    fn fts_candidates(&self, raw_query: &str, query: &MemoryQuery) -> Result<Vec<ScoredRecord>> {
        let fts_query = build_fts_query(raw_query);
        if fts_query.is_empty() {
            return self.all_candidates(query);
        }

        let mut sql = String::from(
            "SELECT mr.id, mr.timestamp, mr.domain, mr.scope, mr.title, mr.summary, mr.content,
                    mr.tags, mr.source_refs, mr.confidence
             FROM memory_records mr
             JOIN memory_records_fts fts ON mr.rowid = fts.rowid
             WHERE memory_records_fts MATCH ?",
        );
        let mut params: Vec<Box<dyn ToSql>> = vec![Box::new(fts_query)];

        if let Some(domain) = &query.domain {
            sql.push_str(" AND mr.domain = ?");
            params.push(Box::new(serde_json::to_string(domain)?));
        }
        if let Some(scope) = &query.scope {
            sql.push_str(" AND mr.scope = ?");
            params.push(Box::new(serde_json::to_string(scope)?));
        }
        if let Some(since) = query.since {
            sql.push_str(" AND mr.timestamp >= ?");
            params.push(Box::new(since.to_rfc3339()));
        }
        if let Some(until) = query.until {
            sql.push_str(" AND mr.timestamp <= ?");
            params.push(Box::new(until.to_rfc3339()));
        }
        sql.push_str(" ORDER BY bm25(memory_records_fts), mr.timestamp DESC, mr.id ASC");

        let conn = lock_conn(&self.conn)?;
        let mut stmt = conn.prepare(&sql).map_err(sqlite_error)?;
        let rows = stmt
            .query_map(
                params_from_iter(params.iter().map(|value| value.as_ref())),
                row_to_record,
            )
            .map_err(sqlite_error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite_error)?;

        Ok(rows
            .into_iter()
            .filter_map(|record| {
                let score = score_record(&record, query)?;
                Some(ScoredRecord {
                    source_rank: source_rank_for_id(&record.id),
                    record,
                    score,
                })
            })
            .collect())
    }
}

impl MemoryStore for SqliteMemoryStore {
    fn store_record(&self, record: &MemoryRecord) -> Result<()> {
        let conn = lock_conn(&self.conn)?;
        let tags_text = joined_tags(record);
        let source_paths_text = joined_source_paths(record);
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO memory_records (
                id, timestamp, domain, scope, title, summary, content, tags, source_refs,
                tags_text, source_paths_text, confidence, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
             ON CONFLICT(id) DO UPDATE SET
                timestamp = excluded.timestamp,
                domain = excluded.domain,
                scope = excluded.scope,
                title = excluded.title,
                summary = excluded.summary,
                content = excluded.content,
                tags = excluded.tags,
                source_refs = excluded.source_refs,
                tags_text = excluded.tags_text,
                source_paths_text = excluded.source_paths_text,
                confidence = excluded.confidence,
                updated_at = excluded.updated_at",
            params![
                record.id,
                record.timestamp.to_rfc3339(),
                serde_json::to_string(&record.domain)?,
                serde_json::to_string(&record.scope)?,
                record.title,
                record.summary,
                record.content,
                serde_json::to_string(&record.tags)?,
                serde_json::to_string(&record.source_refs)?,
                tags_text,
                source_paths_text,
                record.confidence,
                record.timestamp.to_rfc3339(),
                now,
            ],
        )
        .map_err(sqlite_error)?;
        Ok(())
    }

    fn get_record(&self, id: &str) -> Result<Option<MemoryRecord>> {
        let conn = lock_conn(&self.conn)?;
        let mut stmt = conn
            .prepare(
                "SELECT id, timestamp, domain, scope, title, summary, content, tags, source_refs,
                        confidence
                 FROM memory_records
                 WHERE id = ?1",
            )
            .map_err(sqlite_error)?;
        stmt.query_row([id], row_to_record)
            .optional()
            .map_err(sqlite_error)
    }

    fn list_records(&self) -> Result<Vec<MemoryRecord>> {
        let conn = lock_conn(&self.conn)?;
        let mut stmt = conn
            .prepare(
                "SELECT id, timestamp, domain, scope, title, summary, content, tags, source_refs,
                        confidence
                 FROM memory_records
                 ORDER BY timestamp DESC, updated_at DESC, id ASC",
            )
            .map_err(sqlite_error)?;
        let rows = stmt
            .query_map([], row_to_record)
            .map_err(sqlite_error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite_error)?;
        Ok(rows)
    }

    fn forget_record(&self, id: &str) -> Result<bool> {
        let conn = lock_conn(&self.conn)?;
        let deleted = conn
            .execute("DELETE FROM memory_records WHERE id = ?1", [id])
            .map_err(sqlite_error)?;
        Ok(deleted > 0)
    }

    fn recall(&self, query: &MemoryQuery) -> Result<Vec<MemoryRecord>> {
        let mut scored =
            if let Some(raw_query) = query.query.as_deref().filter(|q| !q.trim().is_empty()) {
                self.fts_candidates(raw_query, query)?
            } else {
                self.all_candidates(query)?
            };

        scored.retain(|candidate| score_record(&candidate.record, query).is_some());
        scored.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then(right.source_rank.cmp(&left.source_rank))
                .then(right.record.timestamp.cmp(&left.record.timestamp))
                .then(left.record.id.cmp(&right.record.id))
        });
        scored.truncate(query.limit.max(1));
        Ok(scored
            .into_iter()
            .map(|candidate| candidate.record)
            .collect())
    }
}

fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL
         );

         CREATE TABLE IF NOT EXISTS memory_records (
            id TEXT PRIMARY KEY,
            timestamp TEXT NOT NULL,
            domain TEXT NOT NULL,
            scope TEXT NOT NULL,
            title TEXT NOT NULL,
            summary TEXT NOT NULL,
            content TEXT NOT NULL,
            tags TEXT NOT NULL,
            source_refs TEXT NOT NULL,
            tags_text TEXT NOT NULL DEFAULT '',
            source_paths_text TEXT NOT NULL DEFAULT '',
            confidence REAL NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
         );

         CREATE INDEX IF NOT EXISTS idx_memory_records_timestamp
         ON memory_records(timestamp DESC);",
    )
    .map_err(sqlite_error)?;

    conn.execute(
        "INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
        params![1_i64, Utc::now().to_rfc3339()],
    )
    .map_err(sqlite_error)?;

    ensure_column(
        conn,
        "memory_records",
        "tags_text",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        conn,
        "memory_records",
        "source_paths_text",
        "TEXT NOT NULL DEFAULT ''",
    )?;

    conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS memory_records_fts USING fts5(
            title,
            summary,
            content,
            tags_text,
            source_paths_text,
            content='memory_records',
            content_rowid='rowid'
         );

         CREATE TRIGGER IF NOT EXISTS memory_records_ai AFTER INSERT ON memory_records BEGIN
            INSERT INTO memory_records_fts(rowid, title, summary, content, tags_text, source_paths_text)
            VALUES (new.rowid, new.title, new.summary, new.content, new.tags_text, new.source_paths_text);
         END;

         CREATE TRIGGER IF NOT EXISTS memory_records_ad AFTER DELETE ON memory_records BEGIN
            INSERT INTO memory_records_fts(memory_records_fts, rowid, title, summary, content, tags_text, source_paths_text)
            VALUES ('delete', old.rowid, old.title, old.summary, old.content, old.tags_text, old.source_paths_text);
         END;

         CREATE TRIGGER IF NOT EXISTS memory_records_au AFTER UPDATE ON memory_records BEGIN
            INSERT INTO memory_records_fts(memory_records_fts, rowid, title, summary, content, tags_text, source_paths_text)
            VALUES ('delete', old.rowid, old.title, old.summary, old.content, old.tags_text, old.source_paths_text);
            INSERT INTO memory_records_fts(rowid, title, summary, content, tags_text, source_paths_text)
            VALUES (new.rowid, new.title, new.summary, new.content, new.tags_text, new.source_paths_text);
         END;",
    )
    .map_err(sqlite_error)?;

    backfill_derived_columns(conn)?;
    conn.execute(
        "INSERT INTO memory_records_fts(memory_records_fts) VALUES ('rebuild')",
        [],
    )
    .map_err(sqlite_error)?;
    conn.execute(
        "INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
        params![SCHEMA_VERSION, Utc::now().to_rfc3339()],
    )
    .map_err(sqlite_error)?;

    Ok(())
}

fn ensure_column(conn: &Connection, table: &str, column: &str, definition: &str) -> Result<()> {
    let pragma = format!("PRAGMA table_info({table})");
    let mut stmt = conn.prepare(&pragma).map_err(sqlite_error)?;
    let names = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(sqlite_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(sqlite_error)?;
    if !names.iter().any(|name| name == column) {
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
            [],
        )
        .map_err(sqlite_error)?;
    }
    Ok(())
}

fn backfill_derived_columns(conn: &Connection) -> Result<()> {
    let mut stmt = conn
        .prepare("SELECT id, tags, source_refs FROM memory_records")
        .map_err(sqlite_error)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(sqlite_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(sqlite_error)?;

    for (id, tags_json, source_refs_json) in rows {
        let tags: Vec<String> = serde_json::from_str(&tags_json)?;
        let source_refs: Vec<MemorySourceRef> = serde_json::from_str(&source_refs_json)?;
        let source_paths_text = source_refs
            .iter()
            .filter_map(|source| source.path.as_deref())
            .collect::<Vec<_>>()
            .join("\n");
        conn.execute(
            "UPDATE memory_records SET tags_text = ?1, source_paths_text = ?2 WHERE id = ?3",
            params![tags.join(" "), source_paths_text, id],
        )
        .map_err(sqlite_error)?;
    }

    Ok(())
}

fn lock_conn(conn: &Arc<Mutex<Connection>>) -> Result<std::sync::MutexGuard<'_, Connection>> {
    conn.lock()
        .map_err(|_| Error::Internal("sqlite memory store mutex poisoned".into()))
}

fn sqlite_error(err: rusqlite::Error) -> Error {
    Error::Internal(format!("sqlite memory store error: {err}"))
}

fn score_record(record: &MemoryRecord, query: &MemoryQuery) -> Option<usize> {
    if !matches_record_filters(record, query) {
        return None;
    }
    let haystack = [
        record.title.to_lowercase(),
        record.summary.to_lowercase(),
        record.content.to_lowercase(),
        joined_tags(record).to_lowercase(),
        joined_source_paths(record).to_lowercase(),
    ]
    .join("\n");
    let text_score = query_match_score(&haystack, query.query.as_deref())?;
    Some(text_score + source_rank_for_id(&record.id) as usize)
}

fn joined_tags(record: &MemoryRecord) -> String {
    record.tags.join(" ")
}

fn joined_source_paths(record: &MemoryRecord) -> String {
    record
        .source_refs
        .iter()
        .filter_map(|source| source.path.as_deref())
        .collect::<Vec<_>>()
        .join("\n")
}

fn matches_record_filters(record: &MemoryRecord, query: &MemoryQuery) -> bool {
    if let Some(domain) = &query.domain {
        if &record.domain != domain {
            return false;
        }
    }
    if let Some(scope) = &query.scope {
        if &record.scope != scope {
            return false;
        }
    }
    if let Some(since) = query.since {
        if record.timestamp < since {
            return false;
        }
    }
    if let Some(until) = query.until {
        if record.timestamp > until {
            return false;
        }
    }
    true
}

fn build_fts_query(raw_query: &str) -> String {
    let trimmed = raw_query.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let direct = trimmed.replace('"', "\"\"");
    let mut parts = vec![format!("\"{direct}\"")];
    parts.extend(
        normalized_query_terms(trimmed)
            .into_iter()
            .map(|term| format!("\"{}\"", term.replace('"', "\"\""))),
    );
    parts.sort();
    parts.dedup();
    parts.join(" OR ")
}

fn source_rank_for_id(id: &str) -> u8 {
    if id.starts_with("diary:") {
        3
    } else if id.starts_with("compat:") {
        1
    } else {
        2
    }
}

fn row_to_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<MemoryRecord> {
    let timestamp = row.get::<_, String>(1)?;
    let domain = row.get::<_, String>(2)?;
    let scope = row.get::<_, String>(3)?;
    let tags = row.get::<_, String>(7)?;
    let source_refs = row.get::<_, String>(8)?;

    Ok(MemoryRecord {
        id: row.get(0)?,
        timestamp: parse_timestamp(&timestamp)?,
        domain: parse_json_field(&domain)?,
        scope: parse_json_field(&scope)?,
        title: row.get(4)?,
        summary: row.get(5)?,
        content: row.get(6)?,
        tags: parse_json_field(&tags)?,
        source_refs: parse_json_field(&source_refs)?,
        confidence: row.get(9)?,
    })
}

fn parse_timestamp(value: &str) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|ts| ts.with_timezone(&Utc))
        .map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(
                value.len(),
                rusqlite::types::Type::Text,
                Box::new(err),
            )
        })
}

fn parse_json_field<T>(value: &str) -> rusqlite::Result<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_str(value).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(
            value.len(),
            rusqlite::types::Type::Text,
            Box::new(err),
        )
    })
}

#[derive(Debug, Clone)]
struct ScoredRecord {
    record: MemoryRecord,
    score: usize,
    source_rank: u8,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::MemoryStore;
    use crate::types::{MemoryDomain, MemoryScope, MemorySourceRef};
    use tempfile::TempDir;

    fn sample_record() -> MemoryRecord {
        MemoryRecord {
            id: "rec-1".into(),
            timestamp: DateTime::parse_from_rfc3339("2026-03-26T09:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            domain: MemoryDomain::Workspace,
            scope: MemoryScope::Workspace,
            title: "Memory split".into(),
            summary: "Keep core minimal".into(),
            content: "The sqlite store should become the durable main path.".into(),
            tags: vec!["memory".into(), "sqlite".into()],
            source_refs: vec![MemorySourceRef {
                path: Some("docs/plan.md".into()),
                section: Some("memory".into()),
                note: None,
            }],
            confidence: 0.85,
        }
    }

    #[test]
    fn test_sqlite_memory_store_creates_brain_db() {
        let temp = TempDir::new().unwrap();
        let store = SqliteMemoryStore::new(temp.path()).unwrap();
        assert!(store.db_path().exists());
    }

    #[test]
    fn test_sqlite_memory_store_initialization_is_idempotent() {
        let temp = TempDir::new().unwrap();
        let first = SqliteMemoryStore::new(temp.path()).unwrap();
        let second = SqliteMemoryStore::new(temp.path()).unwrap();
        assert_eq!(first.db_path(), second.db_path());
    }

    #[test]
    fn test_sqlite_memory_store_crud_roundtrip() {
        let temp = TempDir::new().unwrap();
        let store = SqliteMemoryStore::new(temp.path()).unwrap();
        let record = sample_record();

        store.store_record(&record).unwrap();
        let loaded = store.get_record(&record.id).unwrap().unwrap();
        assert_eq!(loaded, record);
        assert_eq!(store.list_records().unwrap().len(), 1);
        assert!(store.forget_record(&record.id).unwrap());
        assert!(store.get_record(&record.id).unwrap().is_none());
    }

    #[test]
    fn test_sqlite_memory_store_recall_filters_records() {
        let temp = TempDir::new().unwrap();
        let store = SqliteMemoryStore::new(temp.path()).unwrap();
        let record = sample_record();
        store.store_record(&record).unwrap();

        let records = store
            .recall(&MemoryQuery {
                query: Some("sqlite durable".into()),
                limit: 5,
                ..MemoryQuery::default()
            })
            .unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, record.id);
    }

    #[test]
    fn test_sqlite_memory_store_forget_missing_record() {
        let temp = TempDir::new().unwrap();
        let store = SqliteMemoryStore::new(temp.path()).unwrap();
        assert!(!store.forget_record("missing").unwrap());
    }

    #[test]
    fn test_sqlite_memory_store_preserves_json_fields() {
        let temp = TempDir::new().unwrap();
        let store = SqliteMemoryStore::new(temp.path()).unwrap();
        let record = sample_record();
        store.store_record(&record).unwrap();

        let loaded = store.get_record(&record.id).unwrap().unwrap();
        assert_eq!(loaded.tags, record.tags);
        assert_eq!(loaded.source_refs, record.source_refs);
    }

    #[test]
    fn test_sqlite_memory_store_migrates_v1_schema_to_v2() {
        let temp = TempDir::new().unwrap();
        let db_path = brain_db_path(temp.path());
        std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);
             CREATE TABLE memory_records (
                id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                domain TEXT NOT NULL,
                scope TEXT NOT NULL,
                title TEXT NOT NULL,
                summary TEXT NOT NULL,
                content TEXT NOT NULL,
                tags TEXT NOT NULL,
                source_refs TEXT NOT NULL,
                confidence REAL NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
             );
             INSERT INTO schema_migrations (version, applied_at) VALUES (1, '2026-03-27T00:00:00Z');",
        )
        .unwrap();
        drop(conn);

        let store = SqliteMemoryStore::new(temp.path()).unwrap();
        store.store_record(&sample_record()).unwrap();
        let records = store
            .recall(&MemoryQuery {
                query: Some("sqlite durable".into()),
                limit: 5,
                ..MemoryQuery::default()
            })
            .unwrap();
        assert_eq!(records.len(), 1);
    }

    #[test]
    fn test_sqlite_memory_store_fts_recall_matches_source_paths() {
        let temp = TempDir::new().unwrap();
        let store = SqliteMemoryStore::new(temp.path()).unwrap();
        let record = sample_record();
        store.store_record(&record).unwrap();

        let records = store
            .recall(&MemoryQuery {
                query: Some("docs/plan".into()),
                limit: 5,
                ..MemoryQuery::default()
            })
            .unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, record.id);
    }
}
