use crate::state::AppState;
use agent_diva_laputa::{
    PersonaError, PersonaInitialization, PersonaKind, PersonaRepair, PersonaRequestActor,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::BTreeMap, str::FromStr};

type ApiError = (StatusCode, Json<Value>);

#[derive(Debug, Deserialize)]
pub struct InitializePersonaPayload {
    pub identity: String,
    pub relationship: String,
    pub redline: String,
    pub user: String,
    pub world: String,
}

#[derive(Debug, Deserialize)]
pub struct RepairPersonaPayload {
    pub documents: BTreeMap<PersonaKind, String>,
}

#[derive(Debug, Deserialize)]
pub struct SavePersonaDocumentPayload {
    pub content: String,
    pub base_revision: u64,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct PersonaRequestQuery {
    pub kind: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePersonaRequestPayload {
    pub kind: String,
    pub base_revision: u64,
    pub base_hash: String,
    pub proposed_markdown: String,
    pub actor: PersonaRequestActor,
    pub reason: String,
}

pub async fn get_persona_status_handler(
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    state
        .persona
        .status()
        .map(|persona| Json(json!({ "status": "ok", "persona": persona })))
        .map_err(persona_error_response)
}

pub async fn initialize_persona_handler(
    State(state): State<AppState>,
    Json(payload): Json<InitializePersonaPayload>,
) -> Result<Json<Value>, ApiError> {
    state
        .persona
        .initialize(PersonaInitialization {
            identity: payload.identity,
            relationship: payload.relationship,
            redline: payload.redline,
            user: payload.user,
            world: payload.world,
        })
        .map(|persona| Json(json!({ "status": "ok", "persona": persona })))
        .map_err(persona_error_response)
}

pub async fn repair_persona_handler(
    State(state): State<AppState>,
    Json(payload): Json<RepairPersonaPayload>,
) -> Result<Json<Value>, ApiError> {
    state
        .persona
        .repair(PersonaRepair {
            documents: payload.documents,
        })
        .map(|persona| Json(json!({ "status": "ok", "persona": persona })))
        .map_err(persona_error_response)
}

pub async fn get_persona_document_handler(
    State(state): State<AppState>,
    Path(kind): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let kind = parse_kind(&kind)?;
    state
        .persona
        .get_document(kind)
        .map(|document| Json(json!({ "status": "ok", "document": document })))
        .map_err(persona_error_response)
}

pub async fn save_persona_document_handler(
    State(state): State<AppState>,
    Path(kind): Path<String>,
    Json(payload): Json<SavePersonaDocumentPayload>,
) -> Result<Json<Value>, ApiError> {
    let kind = parse_kind(&kind)?;
    state
        .persona
        .save_user_document(
            kind,
            &payload.content,
            payload.base_revision,
            &payload.reason,
        )
        .map(|outcome| Json(json!({ "status": "ok", "outcome": outcome })))
        .map_err(persona_error_response)
}

pub async fn list_persona_history_handler(
    State(state): State<AppState>,
    Path(kind): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let kind = parse_kind(&kind)?;
    match state
        .persona
        .status()
        .map_err(persona_error_response)?
        .status
    {
        agent_diva_laputa::PersonaStatus::Uninitialized => {
            return Err(persona_error_response(PersonaError::Uninitialized));
        }
        agent_diva_laputa::PersonaStatus::Ready | agent_diva_laputa::PersonaStatus::Incomplete => {}
    }
    state
        .persona
        .list_history(kind)
        .map(|history| Json(json!({ "status": "ok", "history": history })))
        .map_err(persona_error_response)
}

pub async fn get_persona_history_revision_handler(
    State(state): State<AppState>,
    Path((kind, revision)): Path<(String, u64)>,
) -> Result<Json<Value>, ApiError> {
    let kind = parse_kind(&kind)?;
    state
        .persona
        .read_history(kind, revision)
        .map(|revision| Json(json!({ "status": "ok", "revision": revision })))
        .map_err(persona_error_response)
}

pub async fn list_persona_requests_handler(
    State(state): State<AppState>,
    Query(query): Query<PersonaRequestQuery>,
) -> Result<Json<Value>, ApiError> {
    let kind = query.kind.as_deref().map(parse_kind).transpose()?;
    state
        .persona
        .list_requests(kind)
        .map(|requests| Json(json!({ "status": "ok", "requests": requests })))
        .map_err(persona_error_response)
}

pub async fn create_persona_request_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreatePersonaRequestPayload>,
) -> Result<Json<Value>, ApiError> {
    let kind = parse_kind(&payload.kind)?;
    state
        .persona
        .create_request(
            kind,
            payload.base_revision,
            &payload.base_hash,
            &payload.proposed_markdown,
            payload.actor,
            &payload.reason,
        )
        .map(|request| Json(json!({ "status": "ok", "request": request })))
        .map_err(persona_error_response)
}

pub async fn accept_persona_request_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    state
        .persona
        .accept_request(&id)
        .map(|request| Json(json!({ "status": "ok", "request": request })))
        .map_err(persona_error_response)
}

pub async fn reject_persona_request_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    state
        .persona
        .reject_request(&id)
        .map(|request| Json(json!({ "status": "ok", "request": request })))
        .map_err(persona_error_response)
}

fn parse_kind(value: &str) -> Result<PersonaKind, ApiError> {
    PersonaKind::from_str(value).map_err(persona_error_response)
}

pub(crate) fn persona_error_response(error: PersonaError) -> ApiError {
    let status = match error {
        PersonaError::Uninitialized
        | PersonaError::Incomplete
        | PersonaError::RevisionConflict { .. }
        | PersonaError::RequestExists(_)
        | PersonaError::RequestStale(_)
        | PersonaError::WorldProtectedClaim => StatusCode::CONFLICT,
        PersonaError::RequestNotFound(_) => StatusCode::NOT_FOUND,
        PersonaError::CapExceeded { .. }
        | PersonaError::KindForbidden(_)
        | PersonaError::WorldEntryGate
        | PersonaError::InvalidContent { .. } => StatusCode::UNPROCESSABLE_ENTITY,
        PersonaError::Io { .. } | PersonaError::Json(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(json!({
            "error": {
                "code": error.code(),
                "message": error.to_string(),
            }
        })),
    )
}
