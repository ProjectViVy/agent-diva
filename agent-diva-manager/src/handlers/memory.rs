use agent_diva_core::{evolution::EvidenceRef, memory::MemoryEntry};
use agent_diva_laputa::{ActmemError, ActmemPatch, MemoryHomeError};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::state::AppState;

type ApiError = (StatusCode, Json<Value>);

#[derive(Debug, Deserialize)]
pub struct MemoryListQuery {
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMemoryPayload {
    pub content: String,
    #[serde(default)]
    pub evidence_refs: Vec<EvidenceRef>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMemoryPayload {
    pub content: String,
    pub base_revision: i64,
    #[serde(default)]
    pub evidence_refs: Vec<EvidenceRef>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteMemoryPayload {
    pub reason: String,
    pub base_revision: i64,
}

#[derive(Debug, Deserialize)]
pub struct PutActmemPayload {
    pub pulse: Option<String>,
    pub recap: Option<String>,
    pub work: Option<String>,
    pub base_revision: u64,
}

#[derive(Debug, Deserialize)]
pub struct PutMemRulesPayload {
    pub content: String,
}

pub async fn list_memory_records_handler(
    State(state): State<AppState>,
    Query(query): Query<MemoryListQuery>,
) -> Result<Json<Value>, ApiError> {
    state
        .memory_home
        .list_records(query.limit.unwrap_or(100))
        .await
        .map(|records| Json(json!({ "records": records })))
        .map_err(memory_error_response)
}

pub async fn create_memory_record_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateMemoryPayload>,
) -> Result<Json<Value>, ApiError> {
    state
        .memory_home
        .add_long_term(payload.content, payload.evidence_refs)
        .await
        .map(record_json)
        .map_err(memory_error_response)
}

pub async fn get_memory_record_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    match state
        .memory_home
        .get_record(&id)
        .await
        .map_err(memory_error_response)?
    {
        Some(record) => Ok(record_json(record)),
        None => Err(memory_error_response(MemoryHomeError::NotFound(id))),
    }
}

pub async fn update_memory_record_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateMemoryPayload>,
) -> Result<Json<Value>, ApiError> {
    state
        .memory_home
        .update_record(
            &id,
            payload.content,
            payload.base_revision,
            payload.evidence_refs,
        )
        .await
        .map(record_json)
        .map_err(memory_error_response)
}

pub async fn delete_memory_record_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<DeleteMemoryPayload>,
) -> Result<Json<Value>, ApiError> {
    state
        .memory_home
        .remove_record(&id, payload.reason, payload.base_revision)
        .await
        .map(|record| Json(json!({ "record": record, "deleted": true })))
        .map_err(memory_error_response)
}

pub async fn get_actmem_handler(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    state
        .memory_home
        .actmem()
        .read()
        .map(|actmem| Json(json!(actmem)))
        .map_err(actmem_error_response)
}

pub async fn put_actmem_handler(
    State(state): State<AppState>,
    Json(payload): Json<PutActmemPayload>,
) -> Result<Json<Value>, ApiError> {
    state
        .memory_home
        .actmem()
        .put(ActmemPatch {
            pulse: payload.pulse,
            recap: payload.recap,
            work: payload.work,
            base_revision: payload.base_revision,
        })
        .await
        .map(|actmem| Json(json!(actmem)))
        .map_err(actmem_error_response)
}

pub async fn list_actmem_capsules_handler(
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    state
        .memory_home
        .actmem()
        .list_capsules()
        .map(|capsules| Json(json!({ "capsules": capsules })))
        .map_err(actmem_error_response)
}

pub async fn get_actmem_capsule_handler(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Value>, ApiError> {
    state
        .memory_home
        .actmem()
        .read_capsule(&name)
        .map(|capsule| Json(json!({ "capsule": capsule })))
        .map_err(actmem_error_response)
}

pub async fn delete_actmem_capsule_handler(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Value>, ApiError> {
    state
        .memory_home
        .actmem()
        .delete_capsule(&name)
        .await
        .map(|()| Json(json!({ "deleted": true })))
        .map_err(actmem_error_response)
}

pub async fn get_memrules_handler(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    state
        .memory_home
        .read_memrules()
        .map(|rules| Json(json!(rules)))
        .map_err(memory_error_response)
}

pub async fn put_memrules_handler(
    State(state): State<AppState>,
    Json(payload): Json<PutMemRulesPayload>,
) -> Result<Json<Value>, ApiError> {
    state
        .memory_home
        .write_memrules(&payload.content)
        .map(|rules| Json(json!(rules)))
        .map_err(memory_error_response)
}

fn record_json(record: MemoryEntry) -> Json<Value> {
    Json(json!({ "record": record }))
}

fn memory_error_response(error: MemoryHomeError) -> ApiError {
    let status = match error {
        MemoryHomeError::BmlUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        MemoryHomeError::RevisionConflict { .. } => StatusCode::CONFLICT,
        MemoryHomeError::NotFound(_) => StatusCode::NOT_FOUND,
        MemoryHomeError::KindForbidden | MemoryHomeError::Invalid(_) => {
            StatusCode::UNPROCESSABLE_ENTITY
        }
        MemoryHomeError::Io { .. } => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(json!({
            "error": { "code": error.code(), "message": error.to_string() }
        })),
    )
}

fn actmem_error_response(error: ActmemError) -> ApiError {
    let status = match error {
        ActmemError::RevisionConflict { .. } => StatusCode::CONFLICT,
        ActmemError::InvalidCapsule => StatusCode::NOT_FOUND,
        ActmemError::CapacityExceeded { .. }
        | ActmemError::UnknownWorkSection(_)
        | ActmemError::ItemIndexOutOfRange
        | ActmemError::Malformed(_) => StatusCode::UNPROCESSABLE_ENTITY,
        ActmemError::Io { .. } => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(json!({
            "error": { "code": error.code(), "message": error.to_string() }
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::memory_routes;
    use agent_diva_core::bus::AgentEventBus;
    use axum::{
        body::{to_bytes, Body},
        http::Request,
        Router,
    };
    use tokio::sync::mpsc;
    use tower::util::ServiceExt;

    fn app(temp: &tempfile::TempDir) -> (Router, AppState) {
        let (api_tx, _api_rx) = mpsc::channel(1);
        let state = AppState::new_with_runtime_memory(
            api_tx,
            AgentEventBus::new(),
            temp.path(),
            agent_diva_sandbox::CommandApprovalCoordinator::default(),
            agent_diva_core::ask_user::AskUserCoordinator::default(),
        )
        .unwrap();
        (memory_routes().with_state(state.clone()), state)
    }

    async fn request_json(
        app: Router,
        method: &str,
        uri: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut builder = Request::builder().method(method).uri(uri);
        let body = if let Some(body) = body {
            builder = builder.header("content-type", "application/json");
            Body::from(body.to_string())
        } else {
            Body::empty()
        };
        let response = app.oneshot(builder.body(body).unwrap()).await.unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn memory_routes_cover_direct_crud_without_proposals() {
        let temp = tempfile::tempdir().unwrap();
        let (app, state) = app(&temp);
        let (status, created) = request_json(
            app.clone(),
            "POST",
            "/api/memory/records",
            Some(json!({"content":"likes concise summaries"})),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let id = created["record"]["id"].as_str().unwrap();
        let revision = created["record"]["revision"].as_i64().unwrap();

        let (status, updated) = request_json(
            app.clone(),
            "PATCH",
            &format!("/api/memory/records/{id}"),
            Some(json!({"content":"likes terse summaries","base_revision":revision})),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let updated_revision = updated["record"]["revision"].as_i64().unwrap();
        assert!(updated_revision > revision);

        let (status, conflict) = request_json(
            app.clone(),
            "PATCH",
            &format!("/api/memory/records/{id}"),
            Some(json!({"content":"stale","base_revision":revision})),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(conflict["error"]["code"], "memory_revision_conflict");

        let (status, _) = request_json(
            app.clone(),
            "DELETE",
            &format!("/api/memory/records/{id}"),
            Some(json!({"reason":"obsolete","base_revision":updated_revision})),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (_, list) = request_json(app, "GET", "/api/memory/records", None).await;
        assert!(list["records"].as_array().unwrap().is_empty());
        assert!(
            std::fs::read_dir(state.workspace_root.join(".laputa/proposals"))
                .map(|entries| entries.count() == 0)
                .unwrap_or(true)
        );
    }

    #[tokio::test]
    async fn actmem_capsules_and_memrules_are_independent_from_bml() {
        let temp = tempfile::tempdir().unwrap();
        let (app, state) = app(&temp);
        std::fs::create_dir_all(state.memory_home.memory_dir()).unwrap();
        std::fs::write(state.memory_home.database_path(), b"not sqlite").unwrap();

        let (status, _) = request_json(app.clone(), "GET", "/api/memory/records", None).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        let (status, actmem) = request_json(app.clone(), "GET", "/api/memory/actmem", None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(actmem["revision"], 0);
        let (status, saved) = request_json(
            app.clone(),
            "PUT",
            "/api/memory/actmem",
            Some(json!({"pulse":"- user note","base_revision":0})),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(saved["revision"], 1);

        let (status, defaults) =
            request_json(app.clone(), "GET", "/api/memory/memrules", None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(defaults["source"], "default");
        let (status, file) = request_json(
            app,
            "PUT",
            "/api/memory/memrules",
            Some(json!({"content":"# Local Memory Rules\n"})),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(file["source"], "file");
    }
}
