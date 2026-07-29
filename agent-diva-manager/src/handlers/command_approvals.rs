use crate::state::AppState;
use agent_diva_sandbox::{ApprovalDecision, ApprovalResolveError, CommandApprovalScope};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use tokio_stream::wrappers::BroadcastStream;

#[derive(Debug, Deserialize)]
pub struct ApprovalQuery {
    channel: Option<String>,
    chat_id: Option<String>,
    session_key: Option<String>,
}

impl ApprovalQuery {
    fn scope(&self) -> Option<CommandApprovalScope> {
        Some(CommandApprovalScope {
            channel: self.channel.clone()?,
            chat_id: self.chat_id.clone()?,
            session_key: self.session_key.clone().unwrap_or_else(|| {
                format!(
                    "{}:{}",
                    self.channel.as_deref().unwrap_or_default(),
                    self.chat_id.as_deref().unwrap_or_default()
                )
            }),
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct ResolveApprovalBody {
    decision: ApprovalDecision,
}

#[derive(Debug, Serialize)]
struct ApprovalError {
    error: &'static str,
}

pub fn command_approval_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/command-approvals",
            get(list_command_approvals_handler),
        )
        .route(
            "/api/command-approvals/events",
            get(command_approval_events_handler),
        )
        .route(
            "/api/command-approvals/:approval_id",
            post(resolve_command_approval_handler),
        )
}

pub async fn list_command_approvals_handler(
    State(state): State<AppState>,
    Query(query): Query<ApprovalQuery>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "requests": state.command_approvals.pending(query.scope().as_ref()).await
    }))
}

pub async fn resolve_command_approval_handler(
    State(state): State<AppState>,
    Path(approval_id): Path<String>,
    Json(body): Json<ResolveApprovalBody>,
) -> Response {
    match state
        .command_approvals
        .resolve(&approval_id, body.decision)
        .await
    {
        Ok(response) => Json(response).into_response(),
        Err(ApprovalResolveError::NotFound) => (
            StatusCode::NOT_FOUND,
            Json(ApprovalError {
                error: "approval_not_found",
            }),
        )
            .into_response(),
        Err(ApprovalResolveError::AlreadyResolved) => (
            StatusCode::CONFLICT,
            Json(ApprovalError {
                error: "approval_already_resolved",
            }),
        )
            .into_response(),
    }
}

pub async fn command_approval_events_handler(
    State(state): State<AppState>,
    Query(query): Query<ApprovalQuery>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let scope = query.scope();
    let stream =
        BroadcastStream::new(state.command_approvals.subscribe()).filter_map(move |message| {
            let scope = scope.clone();
            async move {
                let request = message.ok()?;
                if scope.as_ref().is_some_and(|scope| scope != &request.scope) {
                    return None;
                }
                let data = serde_json::to_string(&request).ok()?;
                Some(Ok(Event::default()
                    .event("command_approval_requested")
                    .data(data)))
            }
        });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::bus::MessageBus;
    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use tokio::sync::mpsc;
    use tower::ServiceExt;

    fn state() -> AppState {
        let (api_tx, _api_rx) = mpsc::channel(1);
        AppState::new(
            api_tx,
            MessageBus::new(),
            tempfile::tempdir().unwrap().keep(),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn resolve_endpoint_distinguishes_missing_and_resolved() {
        let state = state();
        let task = tokio::spawn({
            let coordinator = state.command_approvals.clone();
            async move {
                coordinator
                    .request(
                        "echo ok".into(),
                        ".".into(),
                        "sandbox denied".into(),
                        CommandApprovalScope {
                            channel: "api".into(),
                            chat_id: "chat".into(),
                            session_key: "api:chat".into(),
                        },
                    )
                    .await
            }
        });
        tokio::task::yield_now().await;
        let id = state.command_approvals.pending(None).await[0]
            .approval_id
            .clone();
        let app = command_approval_routes().with_state(state);
        let request = |id: &str| {
            Request::builder()
                .method("POST")
                .uri(format!("/api/command-approvals/{id}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"decision":"approve_once"}"#))
                .unwrap()
        };
        assert_eq!(
            app.clone().oneshot(request(&id)).await.unwrap().status(),
            StatusCode::OK
        );
        assert_eq!(
            app.clone().oneshot(request(&id)).await.unwrap().status(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            app.oneshot(request("missing")).await.unwrap().status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            task.await.unwrap().1,
            agent_diva_sandbox::CommandApprovalStatus::Approved
        );
    }

    #[tokio::test]
    async fn pending_endpoint_filters_by_scope() {
        let state = state();
        let _task = tokio::spawn({
            let coordinator = state.command_approvals.clone();
            async move {
                coordinator
                    .request(
                        "echo ok".into(),
                        ".".into(),
                        "sandbox denied".into(),
                        CommandApprovalScope {
                            channel: "api".into(),
                            chat_id: "chat".into(),
                            session_key: "api:chat".into(),
                        },
                    )
                    .await
            }
        });
        tokio::task::yield_now().await;
        let app = command_approval_routes().with_state(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/command-approvals?channel=api&chat_id=chat")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["requests"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn invalid_decision_returns_unprocessable_entity() {
        let app = command_approval_routes().with_state(state());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/command-approvals/missing")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"decision":"always_allow"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}
