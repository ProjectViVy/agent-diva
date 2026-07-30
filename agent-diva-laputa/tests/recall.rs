use std::time::{Duration, Instant};

use agent_diva_core::{
    governance::AuditCorrelation,
    memory::{
        memory_content_digest, MemoryProvenance, MemoryProvenanceSource, MemoryRecord,
        MemoryRecordKind, MemoryScope, MemorySensitivity, MemoryTrust, RecallPolicy, RecallRequest,
        RecallSelectionReason, RecallStatus,
    },
};
use agent_diva_laputa::{LaputaRecallService, TypedMemoryStore};
use chrono::{DateTime, TimeZone, Utc};
use tempfile::TempDir;

fn ts(day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, day, 12, 0, 0)
        .single()
        .unwrap()
}

fn scope(session_id: Option<&str>) -> MemoryScope {
    MemoryScope {
        tenant_id: "tenant-1".into(),
        workspace_id: "workspace-1".into(),
        session_id: session_id.map(str::to_string),
    }
}

fn request(query: &str, session_id: Option<&str>) -> RecallRequest {
    RecallRequest {
        query: query.into(),
        scope: scope(session_id),
        correlation: AuditCorrelation {
            request_id: "request-1".into(),
            turn_id: "turn-1".into(),
            session_id: session_id.unwrap_or("global").into(),
            trace_id: None,
        },
        now: ts(30),
        token_budget: 4_000,
        max_candidates: 8,
        policy: RecallPolicy::default_prompt(),
    }
}

fn record(
    id: impl Into<String>,
    content: impl Into<String>,
    kind: MemoryRecordKind,
    session_id: Option<&str>,
) -> MemoryRecord {
    let id = id.into();
    let content = content.into();
    MemoryRecord {
        id,
        kind,
        provenance: MemoryProvenance {
            source: MemoryProvenanceSource::LaputaAppliedSection,
            source_id: "applied-section".into(),
            content_digest: memory_content_digest(content.as_bytes()),
            captured_at: ts(1),
            correlation: request("fixture", session_id).correlation,
        },
        content,
        evidence_refs: Vec::new(),
        confidence_bps: 9_000,
        sensitivity: MemorySensitivity::Internal,
        trust: MemoryTrust::AppliedAuthority,
        scope: scope(session_id),
        created_at: ts(1),
        effective_at: ts(1),
        expires_at: None,
        supersedes: Vec::new(),
        tombstone: None,
    }
}

async fn store(temp: &TempDir) -> TypedMemoryStore {
    TypedMemoryStore::open(temp.path(), "workspace-1")
        .await
        .unwrap()
}

#[tokio::test]
async fn shadow_recall_sees_global_and_exact_session_with_stable_bm25_order() {
    let temp = TempDir::new().unwrap();
    let store = store(&temp).await;
    store
        .put(
            record(
                "global",
                "durable project global",
                MemoryRecordKind::LongTerm,
                None,
            ),
            0,
            None,
        )
        .await
        .unwrap();
    store
        .put(
            record(
                "session",
                "durable project session",
                MemoryRecordKind::History,
                Some("session-1"),
            ),
            1,
            None,
        )
        .await
        .unwrap();
    store
        .put(
            record(
                "other-session",
                "durable project hidden",
                MemoryRecordKind::History,
                Some("session-2"),
            ),
            2,
            None,
        )
        .await
        .unwrap();
    let service = LaputaRecallService::new(store);

    let first = service
        .recall_shadow(&request("durable project", Some("session-1")))
        .await
        .unwrap();
    let second = service
        .recall_shadow(&request("durable project", Some("session-1")))
        .await
        .unwrap();
    assert_eq!(first.outcome.status, RecallStatus::Ready);
    assert_eq!(
        first.metrics.selected_record_ids,
        second.metrics.selected_record_ids
    );
    assert_eq!(first.metrics.candidate_count, 2);
    assert!(first
        .metrics
        .selected_record_ids
        .contains(&"global".to_string()));
    assert!(first
        .metrics
        .selected_record_ids
        .contains(&"session".to_string()));
    assert!(!first
        .metrics
        .selected_record_ids
        .contains(&"other-session".to_string()));
}

#[tokio::test]
async fn unicode_special_characters_and_query_bounds_are_safe() {
    let temp = TempDir::new().unwrap();
    let store = store(&temp).await;
    store
        .put(
            record(
                "unicode",
                "项目 状态 durable",
                MemoryRecordKind::LongTerm,
                None,
            ),
            0,
            None,
        )
        .await
        .unwrap();
    let service = LaputaRecallService::new(store);

    let query = format!("项目 \"状态\" durable {}", "ignored ".repeat(100));
    let shadow = service.recall_shadow(&request(&query, None)).await.unwrap();
    assert_eq!(shadow.outcome.status, RecallStatus::Ready);
    assert_eq!(shadow.metrics.selected_record_ids, ["unicode"]);
}

#[tokio::test]
async fn policy_dedupe_and_metrics_fail_closed_without_raw_content() {
    let temp = TempDir::new().unwrap();
    let store = store(&temp).await;
    store
        .put(
            record(
                "allowed",
                "private needle",
                MemoryRecordKind::Identity,
                None,
            ),
            0,
            None,
        )
        .await
        .unwrap();
    store
        .put(
            record(
                "duplicate",
                "private needle",
                MemoryRecordKind::Preference,
                None,
            ),
            1,
            None,
        )
        .await
        .unwrap();
    let mut denied = record(
        "restricted",
        "private needle restricted",
        MemoryRecordKind::LongTerm,
        None,
    );
    denied.sensitivity = MemorySensitivity::Restricted;
    store.put(denied, 2, None).await.unwrap();
    let service = LaputaRecallService::new(store);

    let shadow = service
        .recall_shadow(&request("private needle identity", None))
        .await
        .unwrap();
    assert_eq!(shadow.metrics.duplicate_count, 1);
    assert_eq!(
        shadow
            .metrics
            .reason_counts
            .iter()
            .find(|count| count.reason == RecallSelectionReason::SensitivityDenied)
            .unwrap()
            .count,
        1
    );
    let metrics_json = serde_json::to_string(&shadow.metrics).unwrap();
    assert!(!metrics_json.contains("private needle"));
    assert!(!metrics_json.contains("restricted"));
}

#[tokio::test]
async fn retrieval_scope_failure_is_degraded_without_stale_prompt() {
    let temp = TempDir::new().unwrap();
    let service = LaputaRecallService::new(store(&temp).await);
    let mut invalid_scope = request("durable", None);
    invalid_scope.scope.workspace_id = "other-workspace".into();

    let shadow = service.recall_shadow(&invalid_scope).await.unwrap();
    assert_eq!(shadow.outcome.status, RecallStatus::Degraded);
    assert!(shadow.outcome.prompt_block.is_none());
    assert!(shadow.outcome.selected_records.is_empty());
}

#[tokio::test]
async fn g2c_labeled_fixture_has_perfect_recall_and_zero_injection_or_duplicates() {
    let temp = TempDir::new().unwrap();
    let store = store(&temp).await;
    let fixtures = [
        (
            "identity",
            "persona codename aurora",
            MemoryRecordKind::Identity,
        ),
        (
            "preference",
            "preference beverage jasmine",
            MemoryRecordKind::Preference,
        ),
        (
            "commitment",
            "commitment release nebula",
            MemoryRecordKind::Commitment,
        ),
        (
            "history",
            "history milestone zephyr",
            MemoryRecordKind::History,
        ),
    ];
    for (revision, (id, content, kind)) in fixtures.into_iter().enumerate() {
        store
            .put(record(id, content, kind, None), revision as i64, None)
            .await
            .unwrap();
    }
    let mut restricted = record(
        "restricted",
        "persona codename aurora restricted",
        MemoryRecordKind::Identity,
        None,
    );
    restricted.sensitivity = MemorySensitivity::Restricted;
    store.put(restricted, 4, None).await.unwrap();
    let service = LaputaRecallService::new(store);
    let labels = [
        ("aurora", "identity"),
        ("jasmine", "preference"),
        ("nebula", "commitment"),
        ("zephyr", "history"),
    ];
    let mut recalled = 0_u32;
    let mut injected = 0_u32;
    let mut duplicates = 0_u32;
    for (query, expected) in labels {
        let shadow = service.recall_shadow(&request(query, None)).await.unwrap();
        recalled += u32::from(
            shadow
                .metrics
                .selected_record_ids
                .contains(&expected.to_string()),
        );
        injected += shadow
            .outcome
            .selected_records
            .iter()
            .filter(|record| record.sensitivity == MemorySensitivity::Restricted)
            .count() as u32;
        duplicates += shadow.metrics.duplicate_count;
    }
    assert_eq!(recalled, 4, "recall@8 must be 100% for the labeled set");
    assert_eq!(injected, 0, "restricted injection rate must be zero");
    assert_eq!(duplicates, 0, "selected duplicate rate must be zero");
}

#[tokio::test]
#[ignore = "explicit G2C 10k-record performance gate"]
async fn ten_thousand_record_recall_p95_is_below_200ms() {
    let temp = TempDir::new().unwrap();
    let store = store(&temp).await;
    for index in 0..10_000 {
        store
            .put(
                record(
                    format!("record-{index:05}"),
                    format!("durable benchmark project record {index}"),
                    MemoryRecordKind::LongTerm,
                    None,
                ),
                i64::from(index),
                None,
            )
            .await
            .unwrap();
    }
    let service = LaputaRecallService::new(store);
    let request = request("durable benchmark project", None);
    let mut samples = Vec::new();
    for _ in 0..20 {
        let started = Instant::now();
        let shadow = service.recall_shadow(&request).await.unwrap();
        assert_eq!(shadow.metrics.selected_count, 8);
        samples.push(started.elapsed());
    }
    samples.sort();
    let p95 = samples[18];
    eprintln!(
        "GMH-23C machine={} records=10000 selected=8 p95_ms={}",
        std::env::consts::OS,
        p95.as_secs_f64() * 1_000.0
    );
    assert!(p95 < Duration::from_millis(200), "p95 was {p95:?}");
}
