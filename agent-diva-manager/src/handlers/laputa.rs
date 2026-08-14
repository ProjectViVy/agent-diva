use std::{
    convert::Infallible,
    fs,
    path::{Path as FsPath, PathBuf},
    str::FromStr,
    sync::OnceLock,
    time::Instant,
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
    adapt_governed_proposal, ChangelogFilter, CognitiveFileKind, GovernedMemoryApply, LaputaError,
    LaputaEventKind, LaputaService, MemoryAdapterContext, MemoryGovernanceDecision,
    MemoryGovernanceError, ProposalEdit, ProposalFilter, RollbackChangelogRequest,
    TypedMemoryStore,
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
    ReceiptConsumedPendingApply,
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
pub struct PersonaWorkspaceQuery {
    pub session_key: Option<String>,
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
        governance.insert(
            proposal.id.clone(),
            governance_projection(&state, proposal).await,
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
    ok(serde_json::json!({
        "status": "ok",
        "feedback": events,
    }))
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
    let governance = governance_projection(&state, &proposal).await;
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
    let decision_started = Instant::now();
    let proposal = state
        .laputa
        .get_proposal(&id)
        .map_err(laputa_error_response)?;
    let current = state
        .memory_governance
        .existing(&proposal, Utc::now())
        .await
        .map_err(memory_governance_error_response)?
        .ok_or_else(|| memory_governance_error_response(MemoryGovernanceError::ApprovalRequired))?;
    let decision_was_new = !matches!(
        current.status,
        ApprovalStatus::Allowed | ApprovalStatus::Denied
    );
    let governance = if !decision_was_new {
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
    if decision_was_new {
        let human_wait_ms = Utc::now()
            .signed_duration_since(proposal.created_at)
            .num_milliseconds()
            .max(0) as u64;
        LaputaService::record_governance_decision_metrics(
            decision_started
                .elapsed()
                .as_millis()
                .min(u128::from(u64::MAX)) as u64,
            human_wait_ms,
            payload.decision == Decision::Deny,
        );
    }
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
    let (approval, receipt) = if existing_journal
        .as_ref()
        .is_some_and(|journal| journal.state == LegacyApplyState::ReceiptConsumedPendingApply)
    {
        let approval = state
            .memory_governance
            .state(&payload.governance_request_id, Utc::now())
            .await
            .map_err(memory_governance_error_response)?;
        if approval.status != ApprovalStatus::Consumed
            || approval.request.content_digest.value
                != agent_diva_laputa::proposal_digest(&proposal).value
        {
            return Err(error_response(
                StatusCode::CONFLICT,
                "apply_recovery_incomplete",
                "consumed journal no longer matches governance state",
            ));
        }
        let receipt = approval.receipt.clone().ok_or_else(|| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "apply_recovery_incomplete",
                "consumed governance state has no receipt",
            )
        })?;
        (approval, receipt)
    } else {
        state
            .memory_governance
            .allowed_receipt(&proposal, Utc::now())
            .await
            .map_err(memory_governance_error_response)?
    };
    let expected_state_version = if existing_journal
        .as_ref()
        .is_some_and(|journal| journal.state == LegacyApplyState::ReceiptConsumedPendingApply)
    {
        payload.expected_version.saturating_add(1)
    } else {
        payload.expected_version
    };
    if approval.request.correlation.request_id != payload.governance_request_id
        || approval.version != expected_state_version
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
    let governance = match journal.state {
        LegacyApplyState::Prepared => {
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
            journal.state = LegacyApplyState::ReceiptConsumedPendingApply;
            journal.governance = Some(governance.clone());
            write_legacy_apply_journal(&journal_path, &journal)
                .map_err(legacy_apply_journal_error_response)?;
            governance
        }
        LegacyApplyState::AuthorityCommitted => {
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
            journal.governance = Some(governance.clone());
            governance
        }
        LegacyApplyState::ReceiptConsumedPendingApply | LegacyApplyState::ReceiptConsumed => {
            journal.governance.clone().ok_or_else(|| {
                error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "apply_recovery_incomplete",
                    "durable apply journal has no consumed governance result",
                )
            })?
        }
    };
    let outcome = match journal.state {
        LegacyApplyState::ReceiptConsumedPendingApply => {
            let outcome = match state
                .laputa
                .recover_apply_outcome(&id, journal.applied_at)
                .map_err(laputa_error_response)?
            {
                Some(recovered) => recovered,
                None => {
                    if state.memory_authority_mode == MemoryAuthorityMode::Typed {
                        let store = TypedMemoryStore::open_canonical(&state.workspace_root)
                            .await
                            .map_err(typed_store_error_response)?;
                        let metadata =
                            store.metadata().await.map_err(typed_store_error_response)?;
                        let record = adapt_governed_proposal(
                            &proposal,
                            &MemoryAdapterContext {
                                tenant_id: "local".into(),
                                workspace_id:
                                    agent_diva_core::workspace_identity::canonical_workspace_id(
                                        &state.workspace_root,
                                    ),
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
            journal.state = LegacyApplyState::ReceiptConsumed;
            journal.outcome = Some(outcome.clone());
            journal.governance = Some(governance.clone());
            write_legacy_apply_journal(&journal_path, &journal)
                .map_err(legacy_apply_journal_error_response)?;
            outcome
        }
        LegacyApplyState::AuthorityCommitted => {
            let outcome = journal.outcome.clone().ok_or_else(|| {
                error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "apply_recovery_incomplete",
                    "durable apply journal has no committed outcome",
                )
            })?;
            journal.state = LegacyApplyState::ReceiptConsumed;
            journal.governance = Some(governance.clone());
            write_legacy_apply_journal(&journal_path, &journal)
                .map_err(legacy_apply_journal_error_response)?;
            outcome
        }
        LegacyApplyState::ReceiptConsumed => journal.outcome.clone().ok_or_else(|| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "apply_recovery_incomplete",
                "durable apply journal has no committed outcome",
            )
        })?,
        LegacyApplyState::Prepared => unreachable!("prepared journal is consumed before apply"),
    };
    notify_typed_memory_projection(&state, &outcome.changelog.id).await;
    if state.memory_authority_mode == MemoryAuthorityMode::Typed {
        let correlation_complete = !payload.governance_request_id.is_empty()
            && !id.is_empty()
            && !outcome.changelog.id.is_empty()
            && !outcome.audit_event.id.is_empty()
            && !outcome.rollback_request.changelog_id.is_empty()
            && governance.request.correlation.request_id == payload.governance_request_id;
        LaputaService::record_typed_apply_metrics(correlation_complete);
    }
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

#[allow(dead_code)] // Legacy proposal recovery is retained until the S5 symbol-removal slice.
pub(crate) async fn recover_memory_approvals(state: &AppState) -> Result<usize, String> {
    let mut recovered = reconcile_memory_proposal_governance(state).await?;
    let journal_dir = state
        .workspace_root
        .join(".laputa")
        .join("legacy-apply-journal");
    let mut journals = Vec::new();
    if journal_dir.exists() {
        for entry in fs::read_dir(&journal_dir).map_err(|error| error.to_string())? {
            let path = entry.map_err(|error| error.to_string())?.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            if let Some(journal) =
                read_legacy_apply_journal(&path).map_err(|error| error.to_string())?
            {
                journals.push(journal);
            }
        }
    }

    let now = Utc::now();
    let workspace_id =
        agent_diva_core::workspace_identity::canonical_workspace_id(&state.workspace_root);
    let mut cursor = None;
    loop {
        let page = state
            .memory_governance
            .states_page(cursor.as_deref(), 128, now)
            .await
            .map_err(|error| error.to_string())?;
        for approval in page.states {
            if approval.request.capability != agent_diva_core::governance::Capability::MemoryApply
                || approval.request.resource.workspace_id != workspace_id
            {
                continue;
            }
            let journal = journals
                .iter()
                .find(|journal| {
                    journal.governance_request_id == approval.request.correlation.request_id
                })
                .cloned();
            match (approval.status.clone(), journal) {
                (ApprovalStatus::Pending, _) => {}
                (ApprovalStatus::Allowed, Some(journal)) => {
                    let _ = apply_laputa_proposal_handler(
                        State(state.clone()),
                        Path(journal.proposal_id.clone()),
                        Json(ApplyProposalPayload {
                            governance_request_id: journal.governance_request_id,
                            expected_version: journal.expected_version,
                            idempotency_key: journal.idempotency_key,
                        }),
                    )
                    .await
                    .map_err(|(_, body)| body.0.to_string())?;
                    recovered += 1;
                }
                (ApprovalStatus::Allowed, None) => {
                    let proposal = state
                        .laputa
                        .get_proposal(&approval.request.resource.resource_id)
                        .map_err(|error| error.to_string())?;
                    if agent_diva_laputa::proposal_digest(&proposal)
                        != approval.request.content_digest
                    {
                        return Err(format!(
                            "memory recovery digest mismatch for {}",
                            proposal.id
                        ));
                    }
                    state
                        .memory_governance
                        .revoke_and_resubmit(&proposal, &approval, now)
                        .await
                        .map_err(|error| error.to_string())?;
                    recovered += 1;
                }
                (ApprovalStatus::Consumed, Some(journal))
                    if journal.state == LegacyApplyState::ReceiptConsumedPendingApply =>
                {
                    let _ = apply_laputa_proposal_handler(
                        State(state.clone()),
                        Path(journal.proposal_id.clone()),
                        Json(ApplyProposalPayload {
                            governance_request_id: journal.governance_request_id,
                            expected_version: journal.expected_version,
                            idempotency_key: journal.idempotency_key,
                        }),
                    )
                    .await
                    .map_err(|(_, body)| body.0.to_string())?;
                    recovered += 1;
                }
                (ApprovalStatus::Consumed, None) => {
                    tracing::error!(
                        request_id = %approval.request.correlation.request_id,
                        "consumed Memory approval has no prepared apply journal"
                    );
                }
                _ => {}
            }
        }
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }
    Ok(recovered)
}

#[allow(dead_code)]
async fn reconcile_memory_proposal_governance(state: &AppState) -> Result<usize, String> {
    let proposals = state
        .laputa
        .list_proposals(ProposalFilter::default())
        .map_err(|error| error.to_string())?;
    let now = Utc::now();
    let mut recovered = 0;
    for proposal in proposals {
        let projection = state.memory_governance.existing(&proposal, now).await;
        match projection {
            Ok(Some(_)) => {}
            Ok(None) => {
                recovered +=
                    recover_missing_proposal_governance(state, &proposal, false, now).await?;
            }
            Err(MemoryGovernanceError::Ledger(ApprovalLedgerError::NotFound)) => {
                recovered +=
                    recover_missing_proposal_governance(state, &proposal, true, now).await?;
            }
            Err(MemoryGovernanceError::StaleProposal) => {
                if is_reviewable_state(&proposal.state) {
                    state
                        .memory_governance
                        .submit(&proposal, None, now)
                        .await
                        .map_err(|error| error.to_string())?;
                    recovered += 1;
                }
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(recovered)
}

#[allow(dead_code)]
async fn recover_missing_proposal_governance(
    state: &AppState,
    proposal: &EvolutionProposal,
    mapping_exists: bool,
    now: DateTime<Utc>,
) -> Result<usize, String> {
    if is_reviewable_state(&proposal.state) {
        state
            .memory_governance
            .restore_pending_request(proposal, now)
            .await
            .map_err(|error| error.to_string())?;
        return Ok(1);
    }
    if proposal.state == ProposalState::Approved {
        if mapping_exists {
            state
                .memory_governance
                .clear_mapping(&proposal.id)
                .await
                .map_err(|error| error.to_string())?;
        }
        state
            .laputa
            .transition_proposal(&proposal.id, ProposalState::NeedsAttention, now)
            .map_err(|error| error.to_string())?;
        return Ok(1);
    }
    Ok(0)
}

#[allow(dead_code)]
fn is_reviewable_state(state: &ProposalState) -> bool {
    matches!(
        state,
        ProposalState::PendingReview | ProposalState::Edited | ProposalState::Deferred
    )
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

/// Aggregate the canonical persona authority and its real governance/session
/// lifecycle into one read-only desktop projection.
pub async fn get_laputa_persona_workspace_handler(
    State(state): State<AppState>,
    Query(query): Query<PersonaWorkspaceQuery>,
) -> JsonResult {
    let snapshot = state
        .laputa
        .read_snapshot(None)
        .map_err(laputa_error_response)?;
    let proposals = state
        .laputa
        .list_proposals(ProposalFilter::default())
        .map_err(laputa_error_response)?;
    let mut proposal_values = Vec::with_capacity(proposals.len());
    for proposal in proposals {
        let mut value = serde_json::to_value(&proposal).unwrap_or_default();
        value["governance"] = governance_projection(&state, &proposal).await;
        proposal_values.push(value);
    }
    let changelog = state
        .laputa
        .list_changelog(ChangelogFilter {
            page: Some(1),
            page_size: Some(100),
            ..ChangelogFilter::default()
        })
        .map_err(laputa_error_response)?;

    let mut authority_versions = serde_json::Map::new();
    for name in agent_diva_laputa::FROZEN_CORE_SECTIONS.iter() {
        if let Some(section) = snapshot.sections.get(name.as_str()) {
            let canonical = if section.content.is_null() {
                String::new()
            } else {
                serde_json::to_string(&section.content).unwrap_or_default()
            };
            authority_versions.insert(
                name.as_str().to_string(),
                serde_json::Value::String(agent_diva_laputa::content_version(&canonical)),
            );
        }
    }

    let session = query.session_key.as_deref().and_then(|session_key| {
        agent_diva_laputa::frozen_core_session_projection(&state.workspace_root, session_key)
    });
    let session_value = session.map(|projection| {
        let versions = projection
            .section_versions
            .into_iter()
            .map(|(name, version)| (name.as_str().to_string(), serde_json::json!(version)))
            .collect::<serde_json::Map<_, _>>();
        serde_json::json!({
            "session_key": projection.session_key,
            "captured_at": projection.captured_at,
            "section_versions": versions,
        })
    });

    let memrules = state
        .laputa
        .read_cognitive_file(CognitiveFileKind::Memrules)
        .map_err(laputa_error_response)?;
    let world = state
        .laputa
        .read_cognitive_file(CognitiveFileKind::World)
        .map_err(laputa_error_response)?;

    ok(serde_json::json!({
        "status": "ok",
        "workspace": {
            "snapshot": snapshot,
            "authority_versions": authority_versions,
            "session": session_value,
            "proposals": proposal_values,
            "changelog": changelog.items,
            "cognitive": {
                "memrules": memrules,
                "world": world,
            },
        }
    }))
}

/// Queue an internal workspace-scoped projection refresh after a Typed BML
/// authority commit. The authority write remains successful even when the
/// AgentLoop is unavailable; a restarted provider opens the latest revision.
async fn notify_typed_memory_projection(state: &AppState, change_id: &str) {
    if state.memory_authority_mode != MemoryAuthorityMode::Typed {
        return;
    }
    let Some(runtime_control_tx) = state.runtime_control_tx.as_ref() else {
        return;
    };
    let store = match TypedMemoryStore::open_canonical(&state.workspace_root).await {
        Ok(store) => store,
        Err(error) => {
            tracing::error!(
                change_id,
                error = %error,
                "failed to open Typed BML store for runtime refresh"
            );
            return;
        }
    };
    let revision = match store.metadata().await {
        Ok(metadata) => match u64::try_from(metadata.store_revision) {
            Ok(revision) => revision,
            Err(_) => {
                tracing::error!(
                    change_id,
                    store_revision = metadata.store_revision,
                    "Typed BML store revision cannot be represented for runtime refresh"
                );
                return;
            }
        },
        Err(error) => {
            tracing::error!(
                change_id,
                error = %error,
                "failed to read Typed BML revision for runtime refresh"
            );
            return;
        }
    };
    let workspace_id =
        agent_diva_core::workspace_identity::canonical_workspace_id(&state.workspace_root);
    if runtime_control_tx
        .send(
            agent_diva_agent::runtime_control::RuntimeControlCommand::RefreshMemoryAuthority {
                workspace_id,
                authority_revision: revision,
                change_id: change_id.to_string(),
            },
        )
        .is_err()
    {
        tracing::warn!(
            change_id,
            authority_revision = revision,
            "AgentLoop Memory projection refresh channel is unavailable"
        );
    }
}

pub async fn get_laputa_cognitive_handler(
    State(state): State<AppState>,
    Path(kind): Path<String>,
) -> JsonResult {
    let kind = match kind.as_str() {
        "memrules" => CognitiveFileKind::Memrules,
        "world" => CognitiveFileKind::World,
        other => {
            return Err(error_response(
                StatusCode::NOT_FOUND,
                "unknown_cognitive_file",
                format!("unknown cognitive governance file: {other}"),
            ))
        }
    };
    let content = state
        .laputa
        .read_cognitive_file(kind)
        .map_err(laputa_error_response)?;
    ok(serde_json::json!({ "status": "ok", "content": content }))
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
        let store = TypedMemoryStore::open_canonical(&state.workspace_root)
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
    if state.memory_authority_mode == MemoryAuthorityMode::Typed {
        notify_typed_memory_projection(&state, &outcome.changelog.id).await;
        LaputaService::record_typed_rollback_metrics(
            !outcome.changelog.id.is_empty()
                && !outcome.audit_event.id.is_empty()
                && outcome.changelog.proposal_id.is_some(),
        );
    }
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
    if matches!(
        &error,
        MemoryGovernanceError::StaleProposal
            | MemoryGovernanceError::Ledger(ApprovalLedgerError::VersionConflict)
            | MemoryGovernanceError::Ledger(ApprovalLedgerError::AlreadyConsumed)
            | MemoryGovernanceError::Ledger(ApprovalLedgerError::Expired)
    ) {
        LaputaService::record_stale_receipt_metric();
    }
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

async fn governance_projection(
    state: &AppState,
    proposal: &EvolutionProposal,
) -> serde_json::Value {
    match state.memory_governance.existing(proposal, Utc::now()).await {
        Ok(Some(view)) => serde_json::to_value(view).unwrap_or(serde_json::Value::Null),
        Ok(None) => serde_json::Value::Null,
        Err(error) => {
            tracing::warn!(
                proposal_id = %proposal.id,
                error = %error,
                "Memory governance projection is unavailable"
            );
            serde_json::Value::Null
        }
    }
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

// Recovery fixtures stay adjacent to the handler they exercise.
#[allow(clippy::items_after_test_module)]
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
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::sync::Arc;

    async fn governed_state(
        root: &FsPath,
    ) -> (AppState, agent_diva_core::governance::ApprovalCoordinator) {
        let governance_dir = root.join(".laputa");
        std::fs::create_dir_all(&governance_dir).unwrap();
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(governance_dir.join("governance.db"))
                    .create_if_missing(true),
            )
            .await
            .unwrap();
        let governance = agent_diva_core::governance::ApprovalCoordinator::new(Arc::new(
            agent_diva_core::governance::SqliteGovernanceLedger::new(pool)
                .await
                .unwrap(),
        ));
        let workspace_id = agent_diva_core::workspace_identity::canonical_workspace_id(root);
        let command = agent_diva_sandbox::CommandApprovalCoordinator::default()
            .governed(governance.clone(), workspace_id.clone());
        let planning_service = Arc::new(crate::planning_service::PlanningService::governed(
            root.to_path_buf(),
            governance.clone(),
            workspace_id,
        ));
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let state = AppState::new_with_runtime_governance(
            api_tx,
            MessageBus::new(),
            root,
            command,
            agent_diva_core::ask_user::AskUserCoordinator::default(),
            MemoryAuthorityMode::Legacy,
            governance.clone(),
            planning_service,
        )
        .unwrap();
        (state, governance)
    }

    #[test]
    fn stale_governance_errors_increment_payload_free_metric() {
        let before = LaputaService::metrics_snapshot().laputa_stale_receipts_total;
        let response = memory_governance_error_response(MemoryGovernanceError::StaleProposal);
        assert_eq!(response.0, StatusCode::CONFLICT);
        assert!(
            LaputaService::metrics_snapshot().laputa_stale_receipts_total > before,
            "stale receipt metric must increment"
        );
    }

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

    async fn ledger_event_count(root: &FsPath) -> i64 {
        let pool = SqlitePoolOptions::new()
            .connect_with(SqliteConnectOptions::new().filename(root.join(".laputa/governance.db")))
            .await
            .unwrap();
        sqlx::query_scalar("SELECT COUNT(*) FROM governance_ledger_events")
            .fetch_one(&pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn read_projections_do_not_append_governance_events() {
        let temp = tempfile::tempdir().unwrap();
        let (state, _) = governed_state(temp.path()).await;
        let proposal = state.laputa.create_proposal(proposal()).unwrap();
        state
            .memory_governance
            .submit(&proposal, None, Utc::now())
            .await
            .unwrap();
        let before = ledger_event_count(temp.path()).await;

        let list = list_laputa_proposals_handler(
            State(state.clone()),
            Query(ProposalQuery {
                state: None,
                proposal_type: None,
                target_section: None,
                source_run_id: None,
                since: None,
            }),
        )
        .await
        .unwrap();
        assert_eq!(
            list.0["governance"][&proposal.id]["status"],
            serde_json::json!("pending")
        );
        let _ = get_laputa_proposal_handler(State(state.clone()), Path(proposal.id.clone()))
            .await
            .unwrap();
        let workspace = get_laputa_persona_workspace_handler(
            State(state),
            Query(PersonaWorkspaceQuery { session_key: None }),
        )
        .await
        .unwrap();
        assert_eq!(
            workspace.0["workspace"]["proposals"][0]["governance"]["status"],
            serde_json::json!("pending")
        );

        assert_eq!(ledger_event_count(temp.path()).await, before);
    }

    #[tokio::test]
    async fn startup_restores_private_pending_request_into_shared_ledger_once() {
        let temp = tempfile::tempdir().unwrap();
        let service = LaputaService::open(temp.path()).unwrap();
        let proposal = service.create_proposal(proposal()).unwrap();
        let retired = agent_diva_laputa::MemoryGovernanceCoordinator::open_lazy(
            temp.path(),
            agent_diva_core::workspace_identity::canonical_workspace_id(temp.path()),
        )
        .unwrap();
        let old = retired.submit(&proposal, None, Utc::now()).await.unwrap();
        let (state, governance) = governed_state(temp.path()).await;

        assert_eq!(recover_memory_approvals(&state).await.unwrap(), 1);
        assert_eq!(recover_memory_approvals(&state).await.unwrap(), 0);
        let restored = governance.state(&old.request_id, Utc::now()).await.unwrap();
        assert_eq!(restored.status, ApprovalStatus::Pending);
        assert!(restored.receipt.is_none());
    }

    #[tokio::test]
    async fn startup_never_imports_private_allow_without_authoritative_receipt() {
        let temp = tempfile::tempdir().unwrap();
        let service = LaputaService::open(temp.path()).unwrap();
        let proposal = service.create_proposal(proposal()).unwrap();
        let retired = agent_diva_laputa::MemoryGovernanceCoordinator::open_lazy(
            temp.path(),
            agent_diva_core::workspace_identity::canonical_workspace_id(temp.path()),
        )
        .unwrap();
        let pending = retired.submit(&proposal, None, Utc::now()).await.unwrap();
        retired
            .decide(
                &proposal,
                pending.request_version,
                MemoryGovernanceDecision {
                    decision: Decision::Allow,
                    grant: ApprovalGrant::Once,
                    actor: GovernanceSubject {
                        kind: GovernanceSubjectKind::User,
                        id: "retired-reviewer".into(),
                    },
                    idempotency_key: "retired-allow",
                    decided_at: Utc::now(),
                },
            )
            .await
            .unwrap();
        service
            .transition_proposal(&proposal.id, ProposalState::Approved, Utc::now())
            .unwrap();
        let (state, governance) = governed_state(temp.path()).await;

        assert_eq!(recover_memory_approvals(&state).await.unwrap(), 1);
        assert_eq!(
            state.laputa.get_proposal(&proposal.id).unwrap().state,
            ProposalState::NeedsAttention
        );
        assert!(matches!(
            governance.state(&pending.request_id, Utc::now()).await,
            Err(ApprovalLedgerError::NotFound)
        ));
    }

    #[tokio::test]
    async fn prepared_journal_consumes_receipt_before_recovering_commit() {
        let temp = tempfile::tempdir().unwrap();
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let state = AppState::new(api_tx, MessageBus::new(), temp.path()).unwrap();
        let proposal = state.laputa.create_proposal(proposal()).unwrap();
        let now = Utc::now();
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
        let applied_at = now + chrono::Duration::seconds(1);
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
                consume_at: now + chrono::Duration::seconds(2),
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
                governance_request_id: allowed.request_id.clone(),
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

        let replay = apply_laputa_proposal_handler(
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
        assert_eq!(replay.0["changelog"]["id"], committed.changelog.id);
        assert_eq!(
            state
                .laputa
                .list_changelog(ChangelogFilter::default())
                .unwrap()
                .total,
            1
        );
    }

    #[tokio::test]
    async fn restart_revokes_dangling_memory_allow_and_resubmits_pending() {
        let temp = tempfile::tempdir().unwrap();
        let (state, governance) = governed_state(temp.path()).await;
        let proposal = state.laputa.create_proposal(proposal()).unwrap();
        let now = Utc::now();
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
                    idempotency_key: "memory-dangling-allow",
                    decided_at: now,
                },
            )
            .await
            .unwrap();
        assert_eq!(recover_memory_approvals(&state).await.unwrap(), 1);
        assert_eq!(
            governance
                .state(&allowed.request_id, Utc::now())
                .await
                .unwrap()
                .status,
            ApprovalStatus::Revoked
        );
        let page = governance.states_page(None, 100, Utc::now()).await.unwrap();
        let replacement = page
            .states
            .iter()
            .find(|candidate| {
                candidate.request.resource.resource_id == proposal.id
                    && candidate.request.correlation.request_id != allowed.request_id
            })
            .unwrap();
        assert_eq!(replacement.status, ApprovalStatus::Pending);
        let database = temp.path().join(".laputa").join("governance.db");
        let pool = SqlitePoolOptions::new()
            .connect_with(SqliteConnectOptions::new().filename(database))
            .await
            .unwrap();
        let events: Vec<String> =
            sqlx::query_scalar("SELECT event_json FROM governance_ledger_events")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert!(!events.join("\n").contains(&proposal.proposed_patch));
    }

    #[tokio::test]
    async fn consumed_prepared_memory_apply_recovers_exactly_once() {
        let temp = tempfile::tempdir().unwrap();
        let (state, _) = governed_state(temp.path()).await;
        let proposal = state.laputa.create_proposal(proposal()).unwrap();
        let now = Utc::now();
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
                    idempotency_key: "memory-prepared-allow",
                    decided_at: now,
                },
            )
            .await
            .unwrap();
        state
            .laputa
            .transition_proposal(&proposal.id, ProposalState::Approved, now)
            .unwrap();
        let consumed = state
            .memory_governance
            .consume_once(
                &allowed.request_id,
                allowed.request_version,
                "memory-prepared-apply",
                now,
            )
            .await
            .unwrap();
        let path = legacy_apply_journal_path(temp.path(), "memory-prepared-apply");
        write_legacy_apply_journal(
            &path,
            &LegacyApplyJournal {
                governance_request_id: allowed.request_id,
                proposal_id: proposal.id.clone(),
                idempotency_key: "memory-prepared-apply".into(),
                proposal_digest: agent_diva_laputa::proposal_digest(&proposal).value,
                expected_version: allowed.request_version,
                applied_at: now,
                consume_at: now,
                state: LegacyApplyState::ReceiptConsumedPendingApply,
                outcome: None,
                governance: Some(consumed),
            },
        )
        .unwrap();
        assert_eq!(
            state
                .laputa
                .list_changelog(ChangelogFilter::default())
                .unwrap()
                .total,
            0
        );
        assert_eq!(recover_memory_approvals(&state).await.unwrap(), 1);
        assert_eq!(recover_memory_approvals(&state).await.unwrap(), 0);
        assert_eq!(
            state
                .laputa
                .list_changelog(ChangelogFilter::default())
                .unwrap()
                .total,
            1
        );
    }

    #[tokio::test]
    async fn tampered_prepared_memory_digest_never_applies() {
        let temp = tempfile::tempdir().unwrap();
        let (state, _) = governed_state(temp.path()).await;
        let proposal = state.laputa.create_proposal(proposal()).unwrap();
        let now = Utc::now();
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
                    idempotency_key: "memory-tamper-allow",
                    decided_at: now,
                },
            )
            .await
            .unwrap();
        let path = legacy_apply_journal_path(temp.path(), "memory-tamper-apply");
        write_legacy_apply_journal(
            &path,
            &LegacyApplyJournal {
                governance_request_id: allowed.request_id.clone(),
                proposal_id: proposal.id.clone(),
                idempotency_key: "memory-tamper-apply".into(),
                proposal_digest: "tampered".into(),
                expected_version: allowed.request_version,
                applied_at: now,
                consume_at: now,
                state: LegacyApplyState::Prepared,
                outcome: None,
                governance: None,
            },
        )
        .unwrap();
        assert!(apply_laputa_proposal_handler(
            State(state.clone()),
            Path(proposal.id.clone()),
            Json(ApplyProposalPayload {
                governance_request_id: allowed.request_id,
                expected_version: allowed.request_version,
                idempotency_key: "memory-tamper-apply".into(),
            }),
        )
        .await
        .is_err());
        assert_eq!(
            state
                .laputa
                .list_changelog(ChangelogFilter::default())
                .unwrap()
                .total,
            0
        );
    }

    #[tokio::test]
    async fn revoked_and_expired_memory_requests_never_apply() {
        let temp = tempfile::tempdir().unwrap();
        let (state, _) = governed_state(temp.path()).await;
        let revoked_proposal = state.laputa.create_proposal(proposal()).unwrap();
        let revoked_pending = state
            .memory_governance
            .submit(&revoked_proposal, None, Utc::now())
            .await
            .unwrap();
        let revoked = state
            .memory_governance
            .revoke(
                &revoked_pending.request_id,
                revoked_pending.request_version,
                "memory-revoke-test",
                GovernanceSubject {
                    kind: GovernanceSubjectKind::User,
                    id: "reviewer".into(),
                },
                Utc::now(),
            )
            .await
            .unwrap();
        assert_eq!(revoked.status, ApprovalStatus::Revoked);

        let mut second = proposal();
        second.id = "expire-recovery".into();
        let expired_proposal = state.laputa.create_proposal(second).unwrap();
        let expired_pending = state
            .memory_governance
            .submit(&expired_proposal, None, Utc::now())
            .await
            .unwrap();
        let expired = state
            .memory_governance
            .expire(
                &expired_pending.request_id,
                expired_pending.request_version,
                "memory-expire-test",
                expired_pending
                    .receipt
                    .as_ref()
                    .map(|receipt| receipt.expires_at)
                    .unwrap_or_else(|| Utc::now() + chrono::Duration::hours(25)),
            )
            .await
            .unwrap();
        assert_eq!(expired.status, ApprovalStatus::Expired);
        assert_eq!(
            state
                .laputa
                .list_changelog(ChangelogFilter::default())
                .unwrap()
                .total,
            0
        );
    }

    #[tokio::test]
    async fn cognitive_handler_returns_memrules_world_and_rejects_unknown() {
        let temp = tempfile::tempdir().unwrap();
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let state = AppState::new(api_tx, MessageBus::new(), temp.path()).unwrap();

        let memrules =
            get_laputa_cognitive_handler(State(state.clone()), Path("memrules".to_string()))
                .await
                .unwrap();
        assert_eq!(memrules.0["status"], "ok");
        assert!(
            memrules.0["content"].as_str().unwrap().contains("## R1"),
            "default MEMRULES rulebook expected"
        );

        let world = get_laputa_cognitive_handler(State(state.clone()), Path("world".to_string()))
            .await
            .unwrap();
        assert_eq!(world.0["status"], "ok");
        assert!(
            world.0["content"].as_str().unwrap().is_empty(),
            "missing WORLD should be returned as an empty world"
        );

        assert!(
            get_laputa_cognitive_handler(State(state), Path("nope".to_string()))
                .await
                .is_err(),
            "unknown cognitive file must fail"
        );
    }

    #[tokio::test]
    async fn typed_commit_queues_workspace_memory_refresh_command() {
        let temp = tempfile::tempdir().unwrap();
        agent_diva_laputa::TypedMemoryStore::open_canonical(temp.path())
            .await
            .unwrap();
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let (runtime_tx, mut runtime_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut state = AppState::new(api_tx, MessageBus::new(), temp.path()).unwrap();
        state.runtime_control_tx = Some(runtime_tx);

        notify_typed_memory_projection(&state, "changelog-1").await;
        let command = runtime_rx.recv().await.expect("refresh command");
        assert!(matches!(
            command,
            agent_diva_agent::runtime_control::RuntimeControlCommand::RefreshMemoryAuthority {
                authority_revision: 0,
                change_id,
                ..
            } if change_id == "changelog-1"
        ));
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
