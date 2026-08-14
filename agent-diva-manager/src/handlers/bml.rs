use agent_diva_core::memory::{
    MemoryCrudContext, MemoryCrudOutcome, MemoryProvider, MemoryRemoveRequest,
};
use agent_diva_core::workspace_identity::canonical_workspace_id;
use agent_diva_laputa::{LaputaError, MemoryListFilter, TypedLaputaMemoryProvider};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::state::AppState;

type JsonResult = Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)>;

#[derive(Debug, Deserialize)]
pub struct MemoryRemovePayload {
    pub reason: String,
}

pub async fn list_bml_memories_handler(
    State(state): State<AppState>,
    Query(filter): Query<MemoryListFilter>,
) -> JsonResult {
    let memories = state
        .laputa
        .list_memories(filter)
        .await
        .map_err(laputa_error_response)?;
    ok(serde_json::json!({ "status": "ok", "memories": memories }))
}

pub async fn get_bml_memory_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> JsonResult {
    let memory = state
        .laputa
        .get_memory(&id)
        .await
        .map_err(laputa_error_response)?;
    match memory {
        Some(memory) => ok(serde_json::json!({ "status": "ok", "memory": memory })),
        None => Err(error_response(
            StatusCode::NOT_FOUND,
            "memory_not_found",
            format!("no BML memory record with id {id}"),
        )),
    }
}

pub async fn remove_bml_memory_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<MemoryRemovePayload>,
) -> JsonResult {
    let reason = payload.reason.trim();
    if reason.is_empty() {
        return Err(error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "reason_required",
            "memory removal requires a reason",
        ));
    }
    let context = MemoryCrudContext {
        workspace_root: state.workspace_root.clone(),
    };
    let provider = TypedLaputaMemoryProvider::open(
        &state.workspace_root,
        canonical_workspace_id(&state.workspace_root),
    )
    .await
    .map_err(|error| laputa_error_response(LaputaError::MemoryStore(error)))?;
    let outcome = provider
        .memory_remove(
            &context,
            MemoryRemoveRequest {
                record_id: id,
                reason: reason.to_string(),
                base_revision: 0,
            },
        )
        .await
        .map_err(|error| {
            error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "memory_remove_failed",
                error.to_string(),
            )
        })?;
    match outcome {
        MemoryCrudOutcome::ProposalCreated { proposal_id } => {
            ok(serde_json::json!({ "status": "ok", "proposal_id": proposal_id }))
        }
        other => Err(error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "memory_remove_failed",
            format!("{other:?}"),
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
    let status = match &error {
        LaputaError::MemoryStore(_) => StatusCode::INTERNAL_SERVER_ERROR,
        _ => StatusCode::UNPROCESSABLE_ENTITY,
    };
    error_response(status, error.code(), error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::bml_routes;
    use agent_diva_core::bus::MessageBus;
    use agent_diva_core::config::schema::MemoryAuthorityMode;
    use agent_diva_core::memory::{MemoryAddRequest, MemoryCrudOutcome, MemoryProvider};
    use agent_diva_laputa::{TypedLaputaMemoryProvider, TypedMemoryStore};
    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use axum::Router;
    use tokio::sync::mpsc;
    use tower::util::ServiceExt;

    fn test_app(temp: &tempfile::TempDir) -> Router {
        let (api_tx, _api_rx) = mpsc::channel(1);
        let state = AppState::new_with_runtime_memory(
            api_tx,
            MessageBus::new(),
            temp.path().to_path_buf(),
            agent_diva_sandbox::CommandApprovalCoordinator::default(),
            agent_diva_core::ask_user::AskUserCoordinator::default(),
            MemoryAuthorityMode::Typed,
        )
        .unwrap();
        bml_routes().with_state(state)
    }

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    async fn seed_memory(temp: &tempfile::TempDir, content: &str) -> String {
        TypedMemoryStore::open_canonical(temp.path()).await.unwrap();
        let provider = TypedLaputaMemoryProvider::open(
            temp.path(),
            agent_diva_core::workspace_identity::canonical_workspace_id(temp.path()),
        )
        .await
        .unwrap();
        let outcome = provider
            .memory_add(
                &MemoryCrudContext {
                    workspace_root: temp.path().to_path_buf(),
                },
                MemoryAddRequest {
                    content: content.to_string(),
                    evidence_refs: vec![],
                },
            )
            .await
            .unwrap();
        drop(provider);
        match outcome {
            MemoryCrudOutcome::Applied { entry, .. } => entry.expect("entry").id,
            other => panic!("expected Applied, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn bml_list_and_detail_return_seeded_memory() {
        let temp = tempfile::tempdir().unwrap();
        let id = seed_memory(&temp, "kestrel patrols the ridge at dawn").await;
        let app = test_app(&temp);

        let list = body_json(
            app.clone()
                .oneshot(
                    Request::builder()
                        .uri("/api/bml/memories")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(list["status"], "ok");
        assert_eq!(list["memories"].as_array().unwrap().len(), 1);
        assert_eq!(list["memories"][0]["record"]["id"], id);

        let detail = body_json(
            app.oneshot(
                Request::builder()
                    .uri(format!("/api/bml/memories/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap(),
        )
        .await;
        assert_eq!(detail["status"], "ok");
        assert_eq!(
            detail["memory"]["record"]["content"],
            "kestrel patrols the ridge at dawn"
        );
    }

    #[tokio::test]
    async fn bml_remove_creates_proposal_without_deleting_the_record() {
        let temp = tempfile::tempdir().unwrap();
        let id = seed_memory(&temp, "obsolete route via marshland pass").await;
        let app = test_app(&temp);

        let remove = body_json(
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(format!("/api/bml/memories/{id}/remove"))
                        .header("content-type", "application/json")
                        .body(Body::from(r#"{"reason":"route retired"}"#))
                        .unwrap(),
                )
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(remove["status"], "ok", "unexpected body {remove}");
        let proposal_id = remove["proposal_id"]
            .as_str()
            .expect("proposal_id")
            .to_string();

        let detail = body_json(
            app.oneshot(
                Request::builder()
                    .uri(format!("/api/bml/memories/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap(),
        )
        .await;
        assert_eq!(
            detail["status"], "ok",
            "record must remain visible until the proposal is applied"
        );
        assert!(!proposal_id.is_empty());
    }

    #[tokio::test]
    async fn bml_remove_requires_a_reason() {
        let temp = tempfile::tempdir().unwrap();
        let id = seed_memory(&temp, "keep this one").await;
        let app = test_app(&temp);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bml/memories/{id}/remove"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"reason":"  "}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}
