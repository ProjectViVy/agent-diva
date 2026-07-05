use std::{convert::Infallible, str::FromStr};

use agent_diva_core::evolution::{
    ChangelogAction, EvolutionProposal, LaputaSectionName, ProposalState, ProposalType,
};
use agent_diva_laputa::{
    ChangelogFilter, LaputaError, LaputaEventKind, ProposalEdit, ProposalFilter,
    RollbackChangelogRequest,
};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::sse::{Event, Sse},
    Json,
};
use chrono::{DateTime, Utc};
use futures::{stream, StreamExt};
use serde::Deserialize;
use tokio_stream::wrappers::BroadcastStream;

use crate::state::AppState;

type JsonResult = Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)>;

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
    pub actor: Option<String>,
    pub applied_at: Option<DateTime<Utc>>,
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
    ok(serde_json::json!({ "status": "ok", "proposals": proposals }))
}

pub async fn create_laputa_proposal_handler(
    State(state): State<AppState>,
    Json(payload): Json<EvolutionProposal>,
) -> JsonResult {
    let proposal = state
        .laputa
        .create_proposal(payload)
        .map_err(laputa_error_response)?;
    ok(serde_json::json!({ "status": "ok", "proposal": proposal }))
}

pub async fn get_laputa_proposal_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> JsonResult {
    let proposal = state
        .laputa
        .get_proposal(&id)
        .map_err(laputa_error_response)?;
    ok(serde_json::json!({ "status": "ok", "proposal": proposal }))
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
    ok(serde_json::json!({ "status": "ok", "proposal": proposal }))
}

pub async fn transition_laputa_proposal_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<ProposalTransitionPayload>,
) -> JsonResult {
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

pub async fn apply_laputa_proposal_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<ApplyProposalPayload>,
) -> JsonResult {
    let outcome = state
        .laputa
        .apply_proposal(
            &id,
            payload.actor.unwrap_or_else(|| "api".to_string()),
            payload.applied_at.unwrap_or_else(Utc::now),
        )
        .map_err(laputa_error_response)?;
    ok(serde_json::json!({
        "status": "ok",
        "proposal": outcome.proposal,
        "changelog": outcome.changelog,
        "audit_event": outcome.audit_event,
        "rollback_request": outcome.rollback_request,
    }))
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

    let outcome = state
        .laputa
        .create_and_apply_direct_edit(
            section,
            payload.content,
            payload.actor.unwrap_or_else(|| "api".to_string()),
            Utc::now(),
        )
        .map_err(laputa_error_response)?;

    ok(serde_json::json!({
        "status": "ok",
        "changelog_id": outcome.changelog.id,
        "applied_at": outcome.changelog.created_at,
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
    let outcome = state
        .laputa
        .rollback_changelog(&id, payload, "api", Utc::now())
        .map_err(laputa_error_response)?;
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
