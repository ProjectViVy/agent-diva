use std::{
    convert::Infallible,
    fs,
    path::{Path as FsPath, PathBuf},
    str::FromStr,
    sync::OnceLock,
};

use agent_diva_core::config::schema::MemoryAuthorityMode;
use agent_diva_core::evolution::{
    ChangelogAction, EvolutionProposal, LaputaSectionName, ProposalState, ProposalType,
};
use agent_diva_core::governance::{
    ApprovalGrant, ApprovalLedgerError, ApprovalStatus, Decision, GovernanceSubject,
    GovernanceSubjectKind,
};
use agent_diva_laputa::{
    adapt_governed_proposal, ChangelogFilter, GovernedMemoryApply, LaputaError, LaputaEventKind,
    MemoryAdapterContext, MemoryGovernanceDecision, MemoryGovernanceError, ProposalEdit,
    ProposalFilter, RollbackChangelogRequest, TypedMemoryStore,
};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::sse::{Event, Sse},
    Json,
};
use chrono::{DateTime, Utc};
use futures::{stream, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio_stream::wrappers::BroadcastStream;

use crate::state::AppState;

type JsonResult = Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)>;
static LEGACY_APPLY_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LegacyApplyState {
    Prepared,
    AuthorityCommitted,
    ReceiptConsumed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyApplyJournal {
    governance_request_id: String,
    proposal_id: String,
    idempotency_key: String,
    proposal_digest: String,
    expected_version: u64,
    applied_at: DateTime<Utc>,
    consume_at: DateTime<Utc>,
    state: LegacyApplyState,
    outcome: Option<agent_diva_laputa::ApplyOutcome>,
    governance: Option<agent_diva_core::governance::ApprovalState>,
}

#[derive(Debug, Deserialize)]
pub struct ProposalQuery {
    pub state: Option<ProposalState>,
    pub proposal_type: Option<ProposalType>,
    pub target_section: Option<LaputaSectionName>,
    pub source_run_id: Option<String>,
    pub since: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct ProposalEditPayload {
    pub proposed_patch: Option<String>,
    pub evidence_refs: Option<Vec<agent_diva_core::evolution::EvidenceRef>>,
    pub risk_level: Option<agent_diva_core::evolution::RiskLevel>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct ProposalTransitionPayload {
    pub state: ProposalState,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct ApplyProposalPayload {
    pub governance_request_id: String,
    pub expected_version: u64,
    pub idempotency_key: String,
}

#[derive(Debug, Deserialize)]
pub struct ProposalDecisionPayload {
    pub decision: Decision,
    pub grant: ApprovalGrant,
    pub expected_version: u64,
    pub idempotency_key: String,
}

#[derive(Debug, Deserialize)]
pub struct SnapshotQuery {
    pub since: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct ChangelogQuery {
    pub page: Option<usize>,
    pub page_size: Option<usize>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub target_section: Option<LaputaSectionName>,
    pub action: Option<ChangelogAction>,
    pub proposal_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EventQuery {
    pub since: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct RecallFeedbackQuery {
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct WriteLaputaSectionPayload {
    pub content: String,
    pub actor: Option<String>,
    pub summary: Option<String>,
}

pub async fn list_laputa_proposals_handler(
    State(state): State<AppState>,
    Query(query): Query<ProposalQuery>,
) -> JsonResult {
    let proposals = state
        .laputa
        .list_proposals(ProposalFilter {
            state: query.state,
            proposal_type: query.proposal_type,
            target_section: query.target_section,
            source_run_id: query.source_run_id,
            since: query.since,
        })
        .map_err(laputa_error_response)?;
    let mut governance = serde_json::Map::new();
    for proposal in &proposals {
        let view = state
            .memory_governance
            .submit(proposal, None, Utc::now())
            .await
            .map_err(memory_governance_error_response)?;
        governance.insert(
            proposal.id.clone(),
            serde_json::to_value(view).unwrap_or_default(),
        );
    }
    ok(serde_json::json!({
        "status": "ok",
        "proposals": proposals,
        "governance": governance,
    }))
}

pub async fn list_recall_feedback_handler(
    State(state): State<AppState>,
    Query(query): Query<RecallFeedbackQuery>,
) -> JsonResult {
    let storage = agent_diva_laputa::LaputaStorage::open(&state.workspace_root)
        .map_err(laputa_error_response)?;
    let events = agent_diva_laputa::RecallFeedbackStore::new(storage)
        .recent(query.limit.unwrap_or(50).min(200))
        .map_err(laputa_error_response)?;
    ok(serde_json::json!({ "feedback": events }))
}

pub async fn create_laputa_proposal_handler(
    State(state): State<AppState>,
    Json(payload): Json<EvolutionProposal>,
) -> JsonResult {
    let proposal = state
        .laputa
        .create_proposal(payload)
        .map_err(laputa_error_response)?;
    let governance = state
        .memory_governance
        .submit(&proposal, None, Utc::now())
        .await
        .map_err(memory_governance_error_response)?;
    ok(serde_json::json!({ "status": "ok", "proposal": proposal, "governance": governance }))
}

pub async fn get_laputa_proposal_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> JsonResult {
    let proposal = state
        .laputa
        .get_proposal(&id)
        .map_err(laputa_error_response)?;
    let governance = state
        .memory_governance
        .submit(&proposal, None, Utc::now())
        .await
        .map_err(memory_governance_error_response)?;
    ok(serde_json::json!({ "status": "ok", "proposal": proposal, "governance": governance }))
}

pub async fn edit_laputa_proposal_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<ProposalEditPayload>,
) -> JsonResult {
    let proposal = state
        .laputa
        .edit_proposal(
            &id,
            ProposalEdit {
                proposed_patch: payload.proposed_patch,
                evidence_refs: payload.evidence_refs,
                risk_level: payload.risk_level,
                updated_at: payload.updated_at.unwrap_or_else(Utc::now),
            },
        )
        .map_err(laputa_error_response)?;
    let governance = state
        .memory_governance
        .submit(&proposal, None, Utc::now())
        .await
        .map_err(memory_governance_error_response)?;
    ok(serde_json::json!({ "status": "ok", "proposal": proposal, "governance": governance }))
}

pub async fn transition_laputa_proposal_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<ProposalTransitionPayload>,
) -> JsonResult {
    if matches!(
        payload.state,
        ProposalState::Approved | ProposalState::Rejected
    ) {
        return Err(error_response(
            StatusCode::CONFLICT,
            "governance_decision_required",
            "approve and reject must use the governed decision endpoint",
        ));
    }
    let proposal = state
        .laputa
        .transition_proposal(
            &id,
            payload.state,
            payload.updated_at.unwrap_or_else(Utc::now),
        )
        .map_err(laputa_error_response)?;
    ok(serde_json::json!({ "status": "ok", "proposal": proposal }))
}

pub async fn decide_laputa_proposal_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<ProposalDecisionPayload>,
) -> JsonResult {
    let proposal = state
        .laputa
        .get_proposal(&id)
        .map_err(laputa_error_response)?;
    let current = state
        .memory_governance
        .submit(&proposal, None, Utc::now())
        .await
        .map_err(memory_governance_error_response)?;
    let governance = if matches!(
        current.status,
        ApprovalStatus::Allowed | ApprovalStatus::Denied
    ) {
        if current.receipt.as_ref().map(|receipt| &receipt.decision) != Some(&payload.decision) {
            return Err(error_response(
                StatusCode::CONFLICT,
                "governance_decision_conflict",
                "governance request already has a different decision",
            ));
        }
        current
    } else {
        if current.request_version != payload.expected_version {
            return Err(error_response(
                StatusCode::CONFLICT,
                "governance_version_conflict",
                "governance request version changed",
            ));
        }
        state
            .memory_governance
            .decide(
                &proposal,
                payload.expected_version,
                MemoryGovernanceDecision {
                    decision: payload.decision.clone(),
                    grant: payload.grant,
                    actor: GovernanceSubject {
                        kind: GovernanceSubjectKind::User,
                        id: "local-user".into(),
                    },
                    idempotency_key: &payload.idempotency_key,
                    decided_at: Utc::now(),
                },
            )
            .await
            .map_err(memory_governance_error_response)?
    };
    let desired_state = if payload.decision == Decision::Allow {
        ProposalState::Approved
    } else {
        ProposalState::Rejected
    };
    let proposal = if proposal.state == desired_state {
        proposal
    } else {
        state
            .laputa
            .transition_proposal(&id, desired_state, Utc::now())
            .map_err(laputa_error_response)?
    };
    ok(serde_json::json!({
        "status": "ok",
        "proposal": proposal,
        "governance": governance,
    }))
}

pub async fn apply_laputa_proposal_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<ApplyProposalPayload>,
) -> JsonResult {
    let _apply_guard = LEGACY_APPLY_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .await;
    let proposal = state
        .laputa
        .get_proposal(&id)
        .map_err(laputa_error_response)?;
    let journal_path = legacy_apply_journal_path(&state.workspace_root, &payload.idempotency_key);
    let existing_journal =
        read_legacy_apply_journal(&journal_path).map_err(legacy_apply_journal_error_response)?;
    if let Some(existing) = existing_journal.as_ref() {
        let current_digest = agent_diva_laputa::proposal_digest(&proposal).value;
        if existing.governance_request_id != payload.governance_request_id
            || existing.proposal_id != id
            || existing.idempotency_key != payload.idempotency_key
            || existing.proposal_digest != current_digest
            || existing.expected_version != payload.expected_version
        {
            return Err(error_response(
                StatusCode::CONFLICT,
                "apply_idempotency_conflict",
                "apply idempotency key is bound to another proposal or governance revision",
            ));
        }
        if existing.state == LegacyApplyState::ReceiptConsumed {
            let outcome = existing.outcome.as_ref().ok_or_else(|| {
                error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "apply_recovery_incomplete",
                    "durable apply journal has no committed outcome",
                )
            })?;
            let governance = existing.governance.as_ref().ok_or_else(|| {
                error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "apply_recovery_incomplete",
                    "durable apply journal has no consumed governance result",
                )
            })?;
            return ok(serde_json::json!({
                "status": "ok",
                "proposal": outcome.proposal,
                "changelog": outcome.changelog,
                "audit_event": outcome.audit_event,
                "rollback_request": outcome.rollback_request,
                "governance": governance,
            }));
        }
    }
    let (approval, receipt) = state
        .memory_governance
        .allowed_receipt(&proposal, Utc::now())
        .await
        .map_err(memory_governance_error_response)?;
    if approval.request.correlation.request_id != payload.governance_request_id
        || approval.version != payload.expected_version
    {
        return Err(error_response(
            StatusCode::CONFLICT,
            "governance_version_conflict",
            "approval request or version changed",
        ));
    }
    let mut journal = match existing_journal {
        Some(existing) => {
            if existing.governance_request_id != payload.governance_request_id
                || existing.proposal_id != id
                || existing.idempotency_key != payload.idempotency_key
                || existing.proposal_digest != approval.request.content_digest.value
                || existing.expected_version != payload.expected_version
            {
                return Err(error_response(
                    StatusCode::CONFLICT,
                    "apply_idempotency_conflict",
                    "apply idempotency key is bound to another proposal or governance revision",
                ));
            }
            existing
        }
        None => {
            let prepared = LegacyApplyJournal {
                governance_request_id: payload.governance_request_id.clone(),
                proposal_id: id.clone(),
                idempotency_key: payload.idempotency_key.clone(),
                proposal_digest: approval.request.content_digest.value.clone(),
                expected_version: payload.expected_version,
                applied_at: Utc::now(),
                consume_at: Utc::now(),
                state: LegacyApplyState::Prepared,
                outcome: None,
                governance: None,
            };
            write_legacy_apply_journal(&journal_path, &prepared)
                .map_err(legacy_apply_journal_error_response)?;
            prepared
        }
    };
    let outcome = match journal.state {
        LegacyApplyState::Prepared => {
            let outcome = match state
                .laputa
                .recover_apply_outcome(&id, journal.applied_at)
                .map_err(laputa_error_response)?
            {
                Some(recovered) => recovered,
                None => {
                    if state.memory_authority_mode == MemoryAuthorityMode::Typed {
                        let store = TypedMemoryStore::open_existing(
                            &state.workspace_root,
                            state.workspace_root.to_string_lossy().to_string(),
                        )
                        .await
                        .map_err(typed_store_error_response)?;
                        let metadata =
                            store.metadata().await.map_err(typed_store_error_response)?;
                        let record = adapt_governed_proposal(
                            &proposal,
                            &MemoryAdapterContext {
                                tenant_id: "local".into(),
                                workspace_id: state.workspace_root.to_string_lossy().to_string(),
                                session_id: None,
                                correlation: approval.request.correlation.clone(),
                                captured_at: journal.applied_at,
                            },
                        );
                        if proposal.proposal_type
                            == agent_diva_core::evolution::ProposalType::Deprecation
                            && record.tombstone.is_none()
                        {
                            return Err(error_response(
                                StatusCode::UNPROCESSABLE_ENTITY,
                                "typed_deprecation_invalid",
                                "deprecation patch is invalid",
                            ));
                        }
                        let expected_record_revision = store
                            .get(&record.id)
                            .await
                            .map_err(typed_store_error_response)?
                            .map(|stored| stored.revision);
                        if let Some(tombstone) = &record.tombstone {
                            if store
                                .get(&tombstone.target_record_id)
                                .await
                                .map_err(typed_store_error_response)?
                                .is_none()
                            {
                                return Err(error_response(
                                    StatusCode::CONFLICT,
                                    "typed_tombstone_target_missing",
                                    "deprecation target does not exist in typed authority",
                                ));
                            }
                        }
                        store
                            .put_governed(
                                record,
                                metadata.store_revision,
                                expected_record_revision,
                                GovernedMemoryApply {
                                    proposal_id: &proposal.id,
                                    idempotency_key: &payload.idempotency_key,
                                    request: &approval.request,
                                    receipt: &receipt,
                                    applied_at: journal.applied_at,
                                },
                            )
                            .await
                            .map_err(typed_store_error_response)?;
                        state
                            .laputa
                            .finalize_typed_proposal(
                                &id,
                                receipt.decided_by.id.clone(),
                                journal.applied_at,
                            )
                            .map_err(laputa_error_response)?
                    } else {
                        state
                            .laputa
                            .apply_proposal(&id, receipt.decided_by.id.clone(), journal.applied_at)
                            .map_err(laputa_error_response)?
                    }
                }
            };
            journal.state = LegacyApplyState::AuthorityCommitted;
            journal.outcome = Some(outcome.clone());
            write_legacy_apply_journal(&journal_path, &journal)
                .map_err(legacy_apply_journal_error_response)?;
            outcome
        }
        LegacyApplyState::AuthorityCommitted | LegacyApplyState::ReceiptConsumed => {
            journal.outcome.clone().ok_or_else(|| {
                error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "apply_recovery_incomplete",
                    "durable apply journal has no committed outcome",
                )
            })?
        }
    };
    let governance = match journal.state {
        LegacyApplyState::ReceiptConsumed => journal.governance.clone().ok_or_else(|| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "apply_recovery_incomplete",
                "durable apply journal has no consumed governance result",
            )
        })?,
        LegacyApplyState::Prepared | LegacyApplyState::AuthorityCommitted => {
            let governance = state
                .memory_governance
                .consume_once(
                    &payload.governance_request_id,
                    payload.expected_version,
                    &payload.idempotency_key,
                    journal.consume_at,
                )
                .await
                .map_err(memory_governance_error_response)?;
            journal.state = LegacyApplyState::ReceiptConsumed;
            journal.outcome = Some(outcome.clone());
            journal.governance = Some(governance.clone());
            write_legacy_apply_journal(&journal_path, &journal)
                .map_err(legacy_apply_journal_error_response)?;
            governance
        }
    };
    ok(serde_json::json!({
        "status": "ok",
        "proposal": outcome.proposal,
        "changelog": outcome.changelog,
        "audit_event": outcome.audit_event,
        "rollback_request": outcome.rollback_request,
        "governance": governance,
    }))
}

fn legacy_apply_journal_path(workspace_root: &FsPath, idempotency_key: &str) -> PathBuf {
    let mut digest = 0xcbf29ce484222325_u64;
    for byte in idempotency_key.as_bytes() {
        digest ^= u64::from(*byte);
        digest = digest.wrapping_mul(0x100000001b3);
    }
    workspace_root
        .join(".laputa")
        .join("legacy-apply-journal")
        .join(format!("{digest:016x}.json"))
}

fn read_legacy_apply_journal(path: &FsPath) -> Result<Option<LegacyApplyJournal>, std::io::Error> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(std::io::Error::other),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn write_legacy_apply_journal(
    path: &FsPath,
    journal: &LegacyApplyJournal,
) -> Result<(), std::io::Error> {
    let bytes = serde_json::to_vec_pretty(journal).map_err(std::io::Error::other)?;
    agent_diva_laputa::atomic_write(path, &bytes)
        .map_err(|error| std::io::Error::other(error.to_string()))
}

fn legacy_apply_journal_error_response(
    error: std::io::Error,
) -> (StatusCode, Json<serde_json::Value>) {
    error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        "apply_recovery_persistence_failed",
        format!("failed to persist apply recovery state: {error}"),
    )
}

pub async fn get_laputa_snapshot_handler(
    State(state): State<AppState>,
    Query(query): Query<SnapshotQuery>,
) -> JsonResult {
    let snapshot = state
        .laputa
        .read_snapshot(query.since)
        .map_err(laputa_error_response)?;
    ok(serde_json::json!({ "status": "ok", "snapshot": snapshot }))
}

pub async fn get_laputa_section_handler(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> JsonResult {
    let section = LaputaSectionName::from_str(&name).map_err(|_| {
        error_response(
            StatusCode::NOT_FOUND,
            "unknown_section",
            format!("unknown Laputa section: {name}"),
        )
    })?;
    let section = state
        .laputa
        .read_section(section)
        .map_err(laputa_error_response)?;
    ok(serde_json::json!({ "status": "ok", "section": section }))
}

pub async fn write_laputa_section_handler(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(payload): Json<WriteLaputaSectionPayload>,
) -> JsonResult {
    let section = LaputaSectionName::from_str(&name).map_err(|_| {
        error_response(
            StatusCode::NOT_FOUND,
            "unknown_section",
            format!("unknown Laputa section: {name}"),
        )
    })?;

    let proposal = state
        .laputa
        .create_user_edit_proposal(
            section,
            payload.content,
            payload.actor.unwrap_or_else(|| "api".to_string()),
            payload.summary,
            Utc::now(),
        )
        .map_err(laputa_error_response)?;
    let governance = state
        .memory_governance
        .submit(&proposal, None, Utc::now())
        .await
        .map_err(memory_governance_error_response)?;

    ok(serde_json::json!({
        "status": "ok",
        "proposal_id": proposal.id,
        "proposal_type": proposal.proposal_type,
        "risk_level": proposal.risk_level,
        "state": proposal.state,
        "changelog_id": null,
        "applied_at": null,
        "governance": governance,
    }))
}

pub async fn list_laputa_changelog_handler(
    State(state): State<AppState>,
    Query(query): Query<ChangelogQuery>,
) -> JsonResult {
    let page = state
        .laputa
        .list_changelog(ChangelogFilter {
            page: query.page,
            page_size: query.page_size,
            since: query.since,
            until: query.until,
            target_section: query.target_section,
            action: query.action,
            proposal_id: query.proposal_id,
        })
        .map_err(laputa_error_response)?;
    ok(serde_json::json!({ "status": "ok", "changelog": page }))
}

pub async fn get_laputa_changelog_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> JsonResult {
    let record = state
        .laputa
        .get_changelog(&id)
        .map_err(laputa_error_response)?;
    ok(serde_json::json!({ "status": "ok", "record": record }))
}

pub async fn rollback_laputa_changelog_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<RollbackChangelogRequest>,
) -> JsonResult {
    let outcome = if state.memory_authority_mode == MemoryAuthorityMode::Typed {
        let changelog = state
            .laputa
            .get_changelog(&id)
            .map_err(laputa_error_response)?;
        let proposal_id = changelog.proposal_id.ok_or_else(|| {
            error_response(
                StatusCode::CONFLICT,
                "typed_rollback_ineligible",
                "typed changelog is not bound to a proposal",
            )
        })?;
        let store = TypedMemoryStore::open_existing(
            &state.workspace_root,
            state.workspace_root.to_string_lossy().to_string(),
        )
        .await
        .map_err(typed_store_error_response)?;
        let revision = store
            .metadata()
            .await
            .map_err(typed_store_error_response)?
            .store_revision;
        if !store
            .rollback_governed(&proposal_id, revision)
            .await
            .map_err(typed_store_error_response)?
        {
            return Err(error_response(
                StatusCode::CONFLICT,
                "typed_rollback_missing_record",
                "typed proposal record is not available for rollback",
            ));
        }
        state
            .laputa
            .finalize_typed_rollback(&id, "api", Utc::now())
            .map_err(laputa_error_response)?
    } else {
        state
            .laputa
            .rollback_changelog(&id, payload, "api", Utc::now())
            .map_err(laputa_error_response)?
    };
    ok(serde_json::json!({ "status": "ok", "outcome": outcome }))
}

pub async fn poll_laputa_events_handler(
    State(state): State<AppState>,
    Path(kind): Path<String>,
    Query(query): Query<EventQuery>,
) -> JsonResult {
    let kind = event_kind_from_path(&kind)?;
    let events = state
        .laputa
        .poll_events(Some(kind), query.since)
        .map_err(laputa_error_response)?;
    ok(serde_json::json!({ "status": "ok", "events": events }))
}

pub async fn stream_laputa_events_handler(
    State(state): State<AppState>,
    Path(kind): Path<String>,
    headers: HeaderMap,
) -> Result<
    Sse<impl futures::Stream<Item = Result<Event, Infallible>>>,
    (StatusCode, Json<serde_json::Value>),
> {
    let kind = event_kind_from_path(&kind)?;
    let replay = state
        .laputa
        .replay_events(kind.clone(), last_event_id(&headers).as_deref())
        .into_iter()
        .filter_map(laputa_event_to_sse)
        .map(Ok);
    let live = BroadcastStream::new(state.laputa.subscribe_events()).filter_map(move |event| {
        let kind = kind.clone();
        async move {
            let Ok(event) = event else {
                return None;
            };
            if event.kind != kind && event.kind != LaputaEventKind::BufferOverflow {
                return None;
            }
            laputa_event_to_sse(event).map(Ok)
        }
    });
    Ok(Sse::new(stream::iter(replay).chain(live))
        .keep_alive(axum::response::sse::KeepAlive::default()))
}

fn last_event_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("last-event-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn laputa_event_to_sse(event: agent_diva_laputa::LaputaEvent) -> Option<Event> {
    let event_name = match event.kind {
        LaputaEventKind::Proposal => "proposal",
        LaputaEventKind::Changelog => "changelog",
        LaputaEventKind::Error => "error",
        LaputaEventKind::BufferOverflow => "buffer_overflow",
    };
    let id = event.event_id.clone();
    match serde_json::to_string(&event) {
        Ok(data) => Some(Event::default().event(event_name).id(id).data(data)),
        Err(error) => Some(Event::default().event("error").data(error.to_string())),
    }
}

fn event_kind_from_path(
    kind: &str,
) -> Result<LaputaEventKind, (StatusCode, Json<serde_json::Value>)> {
    match kind {
        "proposals" => Ok(LaputaEventKind::Proposal),
        "changelog" => Ok(LaputaEventKind::Changelog),
        "errors" => Ok(LaputaEventKind::Error),
        other => Err(error_response(
            StatusCode::NOT_FOUND,
            "unknown_event_stream",
            format!("unknown Laputa event stream: {other}"),
        )),
    }
}

fn ok(value: serde_json::Value) -> JsonResult {
    Ok(Json(value))
}

fn error_response(
    status: StatusCode,
    code: &'static str,
    message: impl Into<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        status,
        Json(serde_json::json!({
            "status": "error",
            "code": code,
            "message": message.into()
        })),
    )
}

fn memory_governance_error_response(
    error: MemoryGovernanceError,
) -> (StatusCode, Json<serde_json::Value>) {
    let (status, code) = match &error {
        MemoryGovernanceError::ApprovalRequired => (StatusCode::CONFLICT, "approval_required"),
        MemoryGovernanceError::StaleProposal => (StatusCode::CONFLICT, "stale_proposal"),
        MemoryGovernanceError::InvalidGrant => {
            (StatusCode::UNPROCESSABLE_ENTITY, "invalid_approval_grant")
        }
        MemoryGovernanceError::Ledger(ApprovalLedgerError::NotFound) => {
            (StatusCode::NOT_FOUND, "governance_not_found")
        }
        MemoryGovernanceError::Ledger(ApprovalLedgerError::VersionConflict) => {
            (StatusCode::CONFLICT, "governance_version_conflict")
        }
        MemoryGovernanceError::Ledger(ApprovalLedgerError::IdempotencyConflict) => {
            (StatusCode::CONFLICT, "governance_idempotency_conflict")
        }
        MemoryGovernanceError::Ledger(ApprovalLedgerError::AlreadyConsumed) => {
            (StatusCode::CONFLICT, "governance_already_consumed")
        }
        MemoryGovernanceError::Ledger(ApprovalLedgerError::Expired) => {
            (StatusCode::UNPROCESSABLE_ENTITY, "governance_expired")
        }
        MemoryGovernanceError::Ledger(ApprovalLedgerError::InvalidTransition) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "governance_invalid_transition",
        ),
        MemoryGovernanceError::Ledger(ApprovalLedgerError::Validation(_)) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "governance_validation_failed",
        ),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "governance_failure"),
    };
    error_response(status, code, error.to_string())
}

fn typed_store_error_response(
    error: agent_diva_laputa::TypedMemoryStoreError,
) -> (StatusCode, Json<serde_json::Value>) {
    use agent_diva_laputa::TypedMemoryStoreError;
    let (status, code) = match &error {
        TypedMemoryStoreError::StoreRevisionConflict { .. }
        | TypedMemoryStoreError::RecordRevisionConflict { .. }
        | TypedMemoryStoreError::ApplyIdempotencyConflict
        | TypedMemoryStoreError::ImportConflict { .. } => {
            (StatusCode::CONFLICT, "typed_memory_conflict")
        }
        TypedMemoryStoreError::WorkspaceMismatch { .. }
        | TypedMemoryStoreError::DatabaseWorkspaceMismatch { .. }
        | TypedMemoryStoreError::InvalidReceipt(_) => {
            (StatusCode::UNPROCESSABLE_ENTITY, "typed_memory_invalid")
        }
        TypedMemoryStoreError::CapacityExceeded { .. } => {
            (StatusCode::INSUFFICIENT_STORAGE, "typed_memory_capacity")
        }
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "typed_memory_failure"),
    };
    error_response(status, code, error.to_string())
}

#[cfg(test)]
mod recovery_tests {
    use super::*;
    use agent_diva_core::{
        bus::MessageBus,
        evolution::{
            EvidenceRef, EvidenceSource, EvolutionProposal, LaputaSectionName, ProposalState,
            ProposalType, RiskLevel,
        },
        governance::{ApprovalGrant, GovernanceSubject, GovernanceSubjectKind},
    };
    use chrono::TimeZone;

    fn proposal() -> EvolutionProposal {
        let now = Utc.with_ymd_and_hms(2026, 7, 30, 12, 0, 0).unwrap();
        EvolutionProposal {
            id: "crash-recovery".into(),
            created_at: now,
            updated_at: now,
            created_by: "test".into(),
            proposal_type: ProposalType::MemoryPatch,
            target_section: LaputaSectionName::MemoryMd,
            evidence_refs: vec![EvidenceRef {
                id: "evidence-1".into(),
                source: EvidenceSource::Session,
                uri: "session://recovery".into(),
                excerpt: None,
                hash: Some("hash".into()),
                created_at: now,
            }],
            proposed_patch: r#"{"facts":[]}"#.into(),
            risk_level: RiskLevel::Medium,
            state: ProposalState::PendingReview,
            source_run_id: None,
        }
    }

    #[tokio::test]
    async fn prepared_journal_recovers_commit_then_consumes_receipt() {
        let temp = tempfile::tempdir().unwrap();
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let state = AppState::new(api_tx, MessageBus::new(), temp.path()).unwrap();
        let proposal = state.laputa.create_proposal(proposal()).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 7, 30, 12, 1, 0).unwrap();
        let pending = state
            .memory_governance
            .submit(&proposal, None, now)
            .await
            .unwrap();
        let allowed = state
            .memory_governance
            .decide(
                &proposal,
                pending.request_version,
                MemoryGovernanceDecision {
                    decision: Decision::Allow,
                    grant: ApprovalGrant::Once,
                    actor: GovernanceSubject {
                        kind: GovernanceSubjectKind::User,
                        id: "reviewer".into(),
                    },
                    idempotency_key: "decision-crash",
                    decided_at: now,
                },
            )
            .await
            .unwrap();
        state
            .laputa
            .transition_proposal(&proposal.id, ProposalState::Approved, now)
            .unwrap();
        let applied_at = Utc.with_ymd_and_hms(2026, 7, 30, 12, 2, 0).unwrap();
        let committed = state
            .laputa
            .apply_proposal(&proposal.id, "reviewer", applied_at)
            .unwrap();
        let path = legacy_apply_journal_path(temp.path(), "apply-crash");
        write_legacy_apply_journal(
            &path,
            &LegacyApplyJournal {
                governance_request_id: allowed.request_id.clone(),
                proposal_id: proposal.id.clone(),
                idempotency_key: "apply-crash".into(),
                proposal_digest: agent_diva_laputa::proposal_digest(&proposal).value,
                expected_version: allowed.request_version,
                applied_at,
                consume_at: Utc.with_ymd_and_hms(2026, 7, 30, 12, 3, 0).unwrap(),
                state: LegacyApplyState::Prepared,
                outcome: None,
                governance: None,
            },
        )
        .unwrap();

        let response = apply_laputa_proposal_handler(
            State(state.clone()),
            Path(proposal.id.clone()),
            Json(ApplyProposalPayload {
                governance_request_id: allowed.request_id,
                expected_version: allowed.request_version,
                idempotency_key: "apply-crash".into(),
            }),
        )
        .await
        .unwrap();

        assert_eq!(response.0["changelog"]["id"], committed.changelog.id);
        assert_eq!(
            state
                .laputa
                .list_changelog(ChangelogFilter::default())
                .unwrap()
                .total,
            1
        );
        let journal = read_legacy_apply_journal(&path).unwrap().unwrap();
        assert_eq!(journal.state, LegacyApplyState::ReceiptConsumed);
        assert!(journal.outcome.is_some());
        assert!(journal.governance.is_some());
    }
}

fn laputa_error_response(error: LaputaError) -> (StatusCode, Json<serde_json::Value>) {
    match &error {
        LaputaError::ProposalNotFound { .. } | LaputaError::ChangelogNotFound { .. } => {
            error_response(StatusCode::NOT_FOUND, "not_found", error.to_string())
        }
        LaputaError::UnknownSection { .. } => {
            error_response(StatusCode::NOT_FOUND, "unknown_section", error.to_string())
        }
        LaputaError::UnknownLayer { .. } => {
            error_response(StatusCode::NOT_FOUND, error.code(), error.to_string())
        }
        LaputaError::InvalidProposal { .. }
        | LaputaError::InvalidProposalTransition { .. }
        | LaputaError::UnauthorizedTarget { .. }
        | LaputaError::SchemaMismatch { .. }
        | LaputaError::SchemaIncompatible { .. }
        | LaputaError::UnresolvedConflict { .. }
        | LaputaError::RollbackExpired { .. }
        | LaputaError::RollbackIneligible { .. }
        | LaputaError::RollbackConflict { .. } => error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            error.code(),
            error.to_string(),
        ),
        _ => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            error.code(),
            error.to_string(),
        ),
    }
}
