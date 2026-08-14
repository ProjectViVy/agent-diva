use agent_diva_core::evolution::{
    CreateSkillProposal, SkillEvidence, SkillHomeError, SkillProposalSource,
};
use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{skill_service::SkillService, state::AppState};

type ApiError = (StatusCode, Json<Value>);

#[derive(Debug, Deserialize)]
pub struct UpdateSkillPayload {
    pub markdown: String,
    pub base_hash: String,
}

#[derive(Debug, Deserialize)]
pub struct SkillCasPayload {
    pub base_hash: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateSkillRequestPayload {
    pub slug: String,
    pub title: String,
    pub proposed_markdown: String,
    #[serde(default)]
    pub evidence: Vec<SkillEvidence>,
    #[serde(default)]
    pub attestation: Option<String>,
    pub base_hash: String,
    pub reason: String,
}

pub async fn get_skills_handler(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let skills = SkillService::from_home(state.skill_home.clone())
        .list_skills()
        .map_err(anyhow_skill_error_response)?;
    Ok(Json(json!({ "status": "ok", "skills": skills })))
}

pub async fn upload_skill_handler(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<Value>, ApiError> {
    let mut file_name = None;
    let mut bytes = None;
    while let Some(field) = multipart.next_field().await.map_err(bad_request)? {
        if field.name() != Some("file") {
            continue;
        }
        file_name = field.file_name().map(ToString::to_string);
        bytes = Some(field.bytes().await.map_err(bad_request)?.to_vec());
    }
    let file_name = file_name.ok_or_else(|| bad_request("missing file upload"))?;
    let bytes = bytes.ok_or_else(|| bad_request("missing file body"))?;
    let skill = SkillService::from_home(state.skill_home.clone())
        .upload_skill_zip(&file_name, bytes)
        .map_err(anyhow_skill_error_response)?;
    Ok(Json(json!({ "status": "ok", "skill": skill })))
}

pub async fn get_skill_handler(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let skill = state.skill_home.read(&slug).map_err(skill_error_response)?;
    Ok(Json(json!({ "status": "ok", "skill": skill })))
}

pub async fn update_skill_handler(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(payload): Json<UpdateSkillPayload>,
) -> Result<Json<Value>, ApiError> {
    let outcome = state
        .skill_home
        .update(&slug, &payload.markdown, &payload.base_hash)
        .map_err(skill_error_response)?;
    Ok(Json(json!({ "status": "ok", "outcome": outcome })))
}

pub async fn delete_skill_handler(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(payload): Json<SkillCasPayload>,
) -> Result<Json<Value>, ApiError> {
    state
        .skill_home
        .hard_delete(&slug, &payload.base_hash)
        .map_err(skill_error_response)?;
    Ok(Json(json!({ "status": "ok" })))
}

pub async fn disable_skill_handler(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(payload): Json<SkillCasPayload>,
) -> Result<Json<Value>, ApiError> {
    let outcome = state
        .skill_home
        .disable(&slug, &payload.base_hash)
        .map_err(skill_error_response)?;
    Ok(Json(json!({ "status": "ok", "outcome": outcome })))
}

pub async fn list_skill_history_handler(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let history = state
        .skill_home
        .history(&slug)
        .map_err(skill_error_response)?;
    Ok(Json(json!({ "status": "ok", "history": history })))
}

pub async fn get_skill_history_revision_handler(
    State(state): State<AppState>,
    Path((slug, revision)): Path<(String, u64)>,
) -> Result<Json<Value>, ApiError> {
    let document = state
        .skill_home
        .history_document(&slug, revision)
        .map_err(skill_error_response)?;
    Ok(Json(json!({ "status": "ok", "document": document })))
}

pub async fn list_skill_requests_handler(
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let requests = state
        .skill_home
        .list_requests()
        .map_err(skill_error_response)?;
    Ok(Json(json!({ "status": "ok", "requests": requests })))
}

pub async fn create_skill_request_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateSkillRequestPayload>,
) -> Result<Json<Value>, ApiError> {
    let request = state
        .skill_home
        .create_request(CreateSkillProposal {
            slug: payload.slug,
            title: payload.title,
            proposed_markdown: payload.proposed_markdown,
            evidence: payload.evidence,
            attestation: payload.attestation,
            base_hash: payload.base_hash,
            source: SkillProposalSource::UserRequest,
            reason: payload.reason,
        })
        .map_err(skill_error_response)?;
    Ok(Json(json!({ "status": "ok", "request": request })))
}

pub async fn get_skill_request_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let request = state
        .skill_home
        .get_request(&id)
        .map_err(skill_error_response)?;
    Ok(Json(json!({ "status": "ok", "request": request })))
}

pub async fn accept_skill_request_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let request = state
        .skill_home
        .accept_request(&id)
        .map_err(skill_error_response)?;
    Ok(Json(json!({ "status": "ok", "request": request })))
}

pub async fn reject_skill_request_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let request = state
        .skill_home
        .reject_request(&id)
        .map_err(skill_error_response)?;
    Ok(Json(json!({ "status": "ok", "request": request })))
}

fn skill_error_response(error: SkillHomeError) -> ApiError {
    let status = match error {
        SkillHomeError::HashConflict
        | SkillHomeError::RequestExists(_)
        | SkillHomeError::RequestStale
        | SkillHomeError::AlreadyExists(_) => StatusCode::CONFLICT,
        SkillHomeError::SlugInvalid(_)
        | SkillHomeError::EvidenceRequired
        | SkillHomeError::SecretRejected
        | SkillHomeError::InvalidMarkdown(_)
        | SkillHomeError::InvalidPackagePath(_) => StatusCode::UNPROCESSABLE_ENTITY,
        SkillHomeError::NotFound(_) => StatusCode::NOT_FOUND,
        SkillHomeError::Io { .. } | SkillHomeError::Serialization(_) => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    };
    let code = error.code();
    (
        status,
        Json(json!({
            "status": "error",
            "error": { "code": code, "message": error.to_string() }
        })),
    )
}

fn anyhow_skill_error_response(error: anyhow::Error) -> ApiError {
    match error.downcast::<SkillHomeError>() {
        Ok(error) => skill_error_response(error),
        Err(error) => internal_error(error.to_string()),
    }
}

fn bad_request(error: impl std::fmt::Display) -> ApiError {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({
            "status": "error",
            "error": { "code": "skill_invalid", "message": error.to_string() }
        })),
    )
}

fn internal_error(message: String) -> ApiError {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({
            "status": "error",
            "error": { "code": "skill_internal_error", "message": message }
        })),
    )
}
