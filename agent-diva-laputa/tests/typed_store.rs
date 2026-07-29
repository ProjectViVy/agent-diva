use std::time::{Duration, Instant};

use agent_diva_core::{
    governance::AuditCorrelation,
    memory::{
        memory_content_digest, MemoryProvenance, MemoryProvenanceSource, MemoryRecord,
        MemoryRecordKind, MemoryScope, MemorySensitivity, MemoryTombstone, MemoryTrust,
    },
};
use agent_diva_laputa::{
    TypedMemoryStore, TypedMemoryStoreError, MAX_MEMORY_CONTENT_BYTES, MAX_MEMORY_RECORDS,
};
use chrono::Utc;
use sqlx::sqlite::SqliteConnectOptions;
use tempfile::TempDir;

fn scope(workspace: &str) -> MemoryScope {
    MemoryScope {
        tenant_id: "tenant-1".into(),
        workspace_id: workspace.into(),
        session_id: None,
    }
}

fn record(id: impl Into<String>, workspace: &str, content: impl Into<String>) -> MemoryRecord {
    let id = id.into();
    let content = content.into();
    let now = Utc::now();
    MemoryRecord {
        id,
        kind: MemoryRecordKind::LongTerm,
        provenance: MemoryProvenance {
            source: MemoryProvenanceSource::UserInput,
            source_id: "user-1".into(),
            content_digest: memory_content_digest(content.as_bytes()),
            captured_at: now,
            correlation: AuditCorrelation {
                request_id: "request-1".into(),
                turn_id: "turn-1".into(),
                session_id: "session-1".into(),
                trace_id: None,
            },
        },
        content,
        evidence_refs: Vec::new(),
        confidence_bps: 9_000,
        sensitivity: MemorySensitivity::Internal,
        trust: MemoryTrust::UserAsserted,
        scope: scope(workspace),
        created_at: now,
        effective_at: now,
        expires_at: None,
        supersedes: Vec::new(),
        tombstone: None,
    }
}

async fn open_store(temp: &TempDir) -> TypedMemoryStore {
    TypedMemoryStore::open(temp.path(), "workspace-1")
        .await
        .unwrap()
}

#[tokio::test]
async fn initializes_idempotently_with_fts5_on_windows_paths() {
    let temp = tempfile::Builder::new()
        .prefix("embedded laputa ")
        .tempdir()
        .unwrap();
    let first = open_store(&temp).await;
    assert!(first.path().ends_with(".laputa/memory.sqlite3"));
    assert_eq!(first.metadata().await.unwrap().store_revision, 0);
    drop(first);

    let reopened = open_store(&temp).await;
    assert_eq!(reopened.metadata().await.unwrap().schema_version, 1);
    let stored = reopened
        .put(record("one", "workspace-1", "hello durable world"), 0, None)
        .await
        .unwrap();
    assert_eq!(stored.revision, 1);
    let hits = reopened
        .search("durable", &scope("workspace-1"), 8)
        .await
        .unwrap();
    assert_eq!(hits[0].stored.record.id, "one");
}

#[tokio::test]
async fn record_round_trip_revision_scope_and_deterministic_list() {
    let temp = TempDir::new().unwrap();
    let store = open_store(&temp).await;
    store
        .put(record("b", "workspace-1", "beta"), 0, None)
        .await
        .unwrap();
    store
        .put(record("a", "workspace-1", "alpha"), 1, None)
        .await
        .unwrap();
    assert_eq!(
        store
            .list(10)
            .await
            .unwrap()
            .into_iter()
            .map(|stored| stored.record.id)
            .collect::<Vec<_>>(),
        ["a", "b"]
    );

    let conflict = store
        .put(record("a", "workspace-1", "changed"), 2, None)
        .await
        .unwrap_err();
    assert!(matches!(
        conflict,
        TypedMemoryStoreError::RecordRevisionConflict { .. }
    ));
    let cross_scope = store
        .put(record("x", "workspace-2", "forbidden"), 2, None)
        .await
        .unwrap_err();
    assert!(matches!(
        cross_scope,
        TypedMemoryStoreError::WorkspaceMismatch { .. }
    ));
    assert_eq!(store.metadata().await.unwrap().store_revision, 2);
}

#[tokio::test]
async fn invalid_write_is_atomic_and_tombstone_leaves_no_fts_content() {
    let temp = TempDir::new().unwrap();
    let store = open_store(&temp).await;
    let mut invalid = record("bad", "workspace-1", "secret");
    invalid.provenance.content_digest = memory_content_digest(b"different");
    assert!(matches!(
        store.put(invalid, 0, None).await.unwrap_err(),
        TypedMemoryStoreError::InvalidRecord(_)
    ));
    assert_eq!(store.integrity().await.unwrap().record_count, 0);

    store
        .put(
            record("old", "workspace-1", "needle private value"),
            0,
            None,
        )
        .await
        .unwrap();
    let mut tombstone = record("forget-old", "workspace-1", "");
    tombstone.supersedes = vec!["old".into()];
    tombstone.tombstone = Some(MemoryTombstone {
        target_record_id: "old".into(),
        reason_digest: memory_content_digest(b"user-forgot"),
        actor_id: "user-1".into(),
        created_at: Utc::now(),
    });
    store.put(tombstone, 1, None).await.unwrap();
    assert!(store
        .search("needle", &scope("workspace-1"), 8)
        .await
        .unwrap()
        .is_empty());
    let integrity = store.integrity().await.unwrap();
    assert_eq!(integrity.tombstone_count, 1);
    assert_eq!(integrity.fts_row_count, 0);
    assert_eq!(integrity.supersedes_edge_count, 1);
    assert_eq!(integrity.orphan_fts_rows, 0);
}

#[tokio::test]
async fn concurrent_store_cas_has_one_winner_and_survives_restart() {
    let temp = TempDir::new().unwrap();
    let store = open_store(&temp).await;
    let left = store.clone();
    let right = store.clone();
    let (left, right) = tokio::join!(
        left.put(record("left", "workspace-1", "left"), 0, None),
        right.put(record("right", "workspace-1", "right"), 0, None)
    );
    assert_eq!(usize::from(left.is_ok()) + usize::from(right.is_ok()), 1);
    let error = left.err().or_else(|| right.err()).unwrap();
    assert!(matches!(
        error,
        TypedMemoryStoreError::StoreRevisionConflict { .. }
    ));
    drop(store);
    let reopened = open_store(&temp).await;
    assert_eq!(reopened.metadata().await.unwrap().store_revision, 1);
    assert_eq!(reopened.list(10).await.unwrap().len(), 1);
}

#[tokio::test]
async fn backup_restore_and_database_identity_are_fail_closed() {
    let temp = TempDir::new().unwrap();
    let store = open_store(&temp).await;
    store
        .put(record("before", "workspace-1", "before backup"), 0, None)
        .await
        .unwrap();
    let backup = temp.path().join("memory backup.sqlite3");
    store.backup(&backup).await.unwrap();
    store
        .put(record("after", "workspace-1", "after backup"), 1, None)
        .await
        .unwrap();
    let restored = store.restore(&backup).await.unwrap();
    assert!(restored.get("before").await.unwrap().is_some());
    assert!(restored.get("after").await.unwrap().is_none());
    drop(restored);
    assert!(matches!(
        TypedMemoryStore::open(temp.path(), "other-workspace")
            .await
            .unwrap_err(),
        TypedMemoryStoreError::DatabaseWorkspaceMismatch { .. }
    ));
}

#[tokio::test]
async fn unknown_schema_fails_without_overwriting_database() {
    let temp = TempDir::new().unwrap();
    let store = open_store(&temp).await;
    let path = store.path().to_path_buf();
    store.close().await;
    let pool = sqlx::SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(false),
    )
    .await
    .unwrap();
    sqlx::query("UPDATE schema_meta SET schema_version = 99")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    assert!(matches!(
        TypedMemoryStore::open(temp.path(), "workspace-1")
            .await
            .unwrap_err(),
        TypedMemoryStoreError::UnsupportedSchema { actual: 99 }
    ));
}

#[tokio::test]
async fn corrupt_record_is_reported_without_repair_or_content_disclosure() {
    let temp = TempDir::new().unwrap();
    let store = open_store(&temp).await;
    store
        .put(record("corrupt-me", "workspace-1", "private"), 0, None)
        .await
        .unwrap();
    let path = store.path().to_path_buf();
    store.close().await;
    let pool = sqlx::SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(false),
    )
    .await
    .unwrap();
    sqlx::query("UPDATE memory_records SET record_json = '{invalid' WHERE memory_id = ?")
        .bind("corrupt-me")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    let reopened = open_store(&temp).await;
    assert_eq!(
        reopened.integrity().await.unwrap().corrupt_record_ids,
        ["corrupt-me"]
    );
    assert!(matches!(
        reopened.get("corrupt-me").await.unwrap_err(),
        TypedMemoryStoreError::CorruptRecord
    ));
}

#[tokio::test]
async fn capacity_constants_and_bounded_search_are_enforced() {
    assert_eq!(MAX_MEMORY_RECORDS, 10_000);
    assert_eq!(MAX_MEMORY_CONTENT_BYTES, 32 * 1024 * 1024);
    let temp = TempDir::new().unwrap();
    let store = open_store(&temp).await;
    let oversized = "x".repeat((MAX_MEMORY_CONTENT_BYTES + 1) as usize);
    assert!(matches!(
        store
            .put(record("oversized", "workspace-1", oversized), 0, None)
            .await
            .unwrap_err(),
        TypedMemoryStoreError::CapacityExceeded { .. }
    ));
}

#[tokio::test]
#[ignore = "10k-record debug performance gate; run explicitly for GMH-23B"]
async fn ten_thousand_record_top_eight_search_p95_is_below_200ms() {
    let temp = TempDir::new().unwrap();
    let store = open_store(&temp).await;
    for index in 0..10_000 {
        store
            .put(
                record(
                    format!("record-{index:05}"),
                    "workspace-1",
                    format!("durable benchmark memory number {index}"),
                ),
                index,
                None,
            )
            .await
            .unwrap();
    }
    let mut timings = Vec::new();
    for _ in 0..20 {
        let started = Instant::now();
        assert_eq!(
            store
                .search("durable benchmark", &scope("workspace-1"), 8)
                .await
                .unwrap()
                .len(),
            8
        );
        timings.push(started.elapsed());
    }
    timings.sort();
    println!("10k top-8 search p95: {:?}", timings[18]);
    assert!(timings[18] < Duration::from_millis(200), "{timings:?}");
}
