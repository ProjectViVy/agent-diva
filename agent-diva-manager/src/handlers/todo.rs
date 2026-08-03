//! Todo HTTP endpoints for the manager API.
//!
//! Provides CRUD routes for todo items backed by `JsonlTodoStore`.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    Router,
};
use serde::{Deserialize, Serialize};

use agent_diva_core::todo::{JsonlTodoStore, TodoItem, TodoStatus, TodoStatusFilter};

use crate::state::AppState;

// ── DTOs ──────────────────────────────────────────────────────────────

/// Request body for `POST /api/todos`.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateTodoRequest {
    pub title: String,
}

/// Request body for `PATCH /api/todos/:id`.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateTodoRequest {
    pub status: String,
}

/// Response body for todo endpoints.
#[derive(Debug, Clone, Serialize)]
pub struct TodoResponse {
    pub id: String,
    pub title: String,
    pub status: String,
    pub source: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<TodoItem> for TodoResponse {
    fn from(item: TodoItem) -> Self {
        Self {
            id: item.id,
            title: item.title,
            status: item.status.as_str().to_string(),
            source: item.source,
            created_at: item.created_at.to_rfc3339(),
            updated_at: item.updated_at.to_rfc3339(),
        }
    }
}

/// Query parameters for `GET /api/todos`.
#[derive(Debug, Clone, Deserialize)]
pub struct TodoQuery {
    /// Optional status filter: `open` (pending+active), `done` (completed), `cancelled`
    pub status: Option<String>,
}

// ── Helpers ───────────────────────────────────────────────────────────

/// Resolve the todo store path from `AppState`.
fn todo_store(state: &AppState) -> JsonlTodoStore {
    let data_root = state.workspace_root.join("todos");
    JsonlTodoStore::new(&data_root).expect("todo store creation")
}

// ── Handlers ──────────────────────────────────────────────────────────

/// GET /api/todos
///
/// List all todo items, optionally filtered by status.
pub async fn query_todos_handler(
    State(state): State<AppState>,
    Query(params): Query<TodoQuery>,
) -> (StatusCode, Json<serde_json::Value>) {
    let store = todo_store(&state);

    let filter = match params.status.as_deref().map(TodoStatusFilter::parse) {
        Some(Some(filter)) => Some(filter),
        Some(None) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "status": "error",
                    "message": format!(
                        "invalid status filter: {}",
                        params.status.as_deref().unwrap_or_default()
                    ),
                })),
            );
        }
        None => None,
    };

    let items = match store.list().await {
        Ok(items) => items,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "status": "error",
                    "message": e.to_string(),
                })),
            );
        }
    };

    let filtered: Vec<TodoResponse> = if let Some(filter) = filter {
        items
            .into_iter()
            .filter(|item| filter.matches(item.status))
            .map(TodoResponse::from)
            .collect()
    } else {
        items.into_iter().map(TodoResponse::from).collect()
    };

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ok",
            "todos": filtered,
        })),
    )
}

/// POST /api/todos
///
/// Create a new todo item.
pub async fn create_todo_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateTodoRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let store = todo_store(&state);
    let item = TodoItem::new(payload.title, "api");

    match store.create(item.clone()).await {
        Ok(created) => {
            let resp = TodoResponse::from(created);
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "status": "ok",
                    "todo": resp,
                })),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "status": "error",
                "message": e.to_string(),
            })),
        ),
    }
}

/// PATCH /api/todos/:id
///
/// Update the status of a todo item.
pub async fn update_todo_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateTodoRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let store = todo_store(&state);

    let new_status = match TodoStatus::parse_update(&payload.status) {
        Some(status) => status,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "status": "error",
                    "message": format!("invalid status: {}", payload.status),
                })),
            );
        }
    };

    match store.update_status(&id, new_status).await {
        Ok(Some(updated)) => {
            let resp = TodoResponse::from(updated);
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "status": "ok",
                    "todo": resp,
                })),
            )
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "status": "error",
                "message": "todo not found",
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "status": "error",
                "message": e.to_string(),
            })),
        ),
    }
}

// ── Router ────────────────────────────────────────────────────────────

/// Build the router for todo routes.
pub fn todo_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/todos",
            axum::routing::get(query_todos_handler).post(create_todo_handler),
        )
        .route("/api/todos/:id", axum::routing::patch(update_todo_handler))
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::bus::MessageBus;
    use axum::body::to_bytes;
    use axum::http::{Request, StatusCode};
    use axum::Router;
    use tokio::sync::mpsc;
    use tower::util::ServiceExt;

    fn test_app_with_dir(dir: &std::path::Path) -> Router {
        let (api_tx, _api_rx) = mpsc::channel(1);
        let state = AppState::new(api_tx, MessageBus::new(), dir).unwrap();
        todo_routes().with_state(state)
    }

    #[tokio::test]
    async fn create_todo_returns_200_with_id() {
        let temp = tempfile::tempdir().unwrap();
        let app = test_app_with_dir(temp.path());

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/todos")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(r#"{"title":"Test todo"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["status"], "ok");
        assert!(!value["todo"]["id"].as_str().unwrap().is_empty());
        assert_eq!(value["todo"]["title"], "Test todo");
        assert_eq!(value["todo"]["status"], "pending");
    }

    #[tokio::test]
    async fn list_todos_contains_created() {
        let temp = tempfile::tempdir().unwrap();
        let app = test_app_with_dir(temp.path());

        // Create a todo
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/todos")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(r#"{"title":"List me"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // List todos
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/todos")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["status"], "ok");
        let todos = value["todos"].as_array().unwrap();
        assert_eq!(todos.len(), 1);
        assert_eq!(todos[0]["title"], "List me");
    }

    #[tokio::test]
    async fn filter_todos_by_status() {
        let temp = tempfile::tempdir().unwrap();
        let app = test_app_with_dir(temp.path());

        // Create a todo
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/todos")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(r#"{"title":"Filter me"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Filter by open status (should match pending)
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/todos?status=open")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let todos = value["todos"].as_array().unwrap();
        assert_eq!(todos.len(), 1);

        // Filter by done status (should not match pending)
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/todos?status=done")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let todos = value["todos"].as_array().unwrap();
        assert_eq!(todos.len(), 0);
    }

    #[tokio::test]
    async fn invalid_status_filter_returns_400() {
        let temp = tempfile::tempdir().unwrap();
        let app = test_app_with_dir(temp.path());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/todos?status=garbage")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["status"], "error");
        assert!(value["message"]
            .as_str()
            .unwrap()
            .contains("invalid status filter"));
    }

    #[tokio::test]
    async fn patch_todo_status_updates() {
        let temp = tempfile::tempdir().unwrap();
        let app = test_app_with_dir(temp.path());

        // Create a todo
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/todos")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(r#"{"title":"Patch me"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let id = value["todo"]["id"].as_str().unwrap().to_string();

        // Patch status to completed
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/todos/{}", id))
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(r#"{"status":"completed"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["status"], "ok");
        assert_eq!(value["todo"]["status"], "completed");

        // Verify via list
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/todos")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let todos = value["todos"].as_array().unwrap();
        assert_eq!(todos[0]["status"], "completed");
    }

    #[tokio::test]
    async fn patch_todo_not_found_returns_404() {
        let temp = tempfile::tempdir().unwrap();
        let app = test_app_with_dir(temp.path());

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/todos/nonexistent-id")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(r#"{"status":"completed"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["status"], "error");
        assert_eq!(value["message"], "todo not found");
    }

    #[tokio::test]
    async fn patch_todo_invalid_status_returns_400() {
        let temp = tempfile::tempdir().unwrap();
        let app = test_app_with_dir(temp.path());

        // Create a todo
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/todos")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(r#"{"title":"Bad status"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let id = value["todo"]["id"].as_str().unwrap().to_string();

        // Patch with invalid status
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/todos/{}", id))
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(r#"{"status":"invalid"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["status"], "error");
        assert!(value["message"]
            .as_str()
            .unwrap()
            .contains("invalid status"));
    }
}
