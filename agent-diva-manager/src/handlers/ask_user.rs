//! Conversational ask-user (clarify) HITL endpoints.
//!
//! The coordinator is process-local and in-memory, so the API is a simple
//! pull model: list pending questions, answer one, or cancel one. The GUI
//! polls the list endpoint; no SSE push channel is needed.

use agent_diva_core::ask_user::AskUserError;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Debug, Serialize)]
struct AskUserErrorBody {
    error: &'static str,
}

#[derive(Debug, Deserialize)]
pub struct AnswerAskUserBody {
    #[serde(default)]
    pub selected_index: Option<usize>,
    #[serde(default)]
    pub other_text: Option<String>,
}

pub fn ask_user_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/ask-user/questions",
            get(list_ask_user_questions_handler),
        )
        .route(
            "/api/ask-user/questions/:question_id/answer",
            post(answer_ask_user_question_handler),
        )
        .route(
            "/api/ask-user/questions/:question_id/cancel",
            post(cancel_ask_user_question_handler),
        )
}

pub async fn list_ask_user_questions_handler(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "questions": state.ask_user.pending().await
    }))
}

pub async fn answer_ask_user_question_handler(
    State(state): State<AppState>,
    Path(question_id): Path<String>,
    Json(body): Json<AnswerAskUserBody>,
) -> axum::response::Response {
    match state
        .ask_user
        .answer(&question_id, body.selected_index, body.other_text)
        .await
    {
        Ok(()) => Json(serde_json::json!({ "status": "answered" })).into_response(),
        Err(AskUserError::NotFound) => (
            axum::http::StatusCode::NOT_FOUND,
            Json(AskUserErrorBody {
                error: "question_not_found",
            }),
        )
            .into_response(),
        Err(AskUserError::InvalidChoice | AskUserError::OtherNotAllowed) => (
            axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            Json(AskUserErrorBody {
                error: "invalid_answer",
            }),
        )
            .into_response(),
        Err(AskUserError::Expired) => (
            axum::http::StatusCode::GONE,
            Json(AskUserErrorBody {
                error: "question_expired",
            }),
        )
            .into_response(),
    }
}

pub async fn cancel_ask_user_question_handler(
    State(state): State<AppState>,
    Path(question_id): Path<String>,
) -> axum::response::Response {
    match state.ask_user.cancel(&question_id).await {
        Ok(()) => Json(serde_json::json!({ "status": "cancelled" })).into_response(),
        Err(AskUserError::NotFound) => (
            axum::http::StatusCode::NOT_FOUND,
            Json(AskUserErrorBody {
                error: "question_not_found",
            }),
        )
            .into_response(),
        Err(AskUserError::InvalidChoice | AskUserError::OtherNotAllowed) => (
            axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            Json(AskUserErrorBody {
                error: "invalid_answer",
            }),
        )
            .into_response(),
        Err(AskUserError::Expired) => (
            axum::http::StatusCode::GONE,
            Json(AskUserErrorBody {
                error: "question_expired",
            }),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use agent_diva_core::ask_user::AskUserCoordinator;
    use agent_diva_core::bus::MessageBus;
    use axum::{
        body::to_bytes,
        http::{Request, StatusCode},
    };
    use std::time::Duration;
    use tower::ServiceExt;

    fn test_state() -> AppState {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(10);
        let bus = MessageBus::new();
        let temp_dir = tempfile::tempdir().unwrap();
        AppState::new(api_tx, bus, temp_dir.path()).unwrap()
    }

    async fn pending_question_id(coordinator: &AskUserCoordinator) -> String {
        for _ in 0..100 {
            if let Some(question) = coordinator.pending().await.into_iter().next() {
                return question.question_id;
            }
            tokio::task::yield_now().await;
        }
        panic!("no pending question registered");
    }

    #[tokio::test]
    async fn list_returns_pending_questions() {
        let state = test_state();
        let coordinator = state.ask_user.clone();
        let ask = tokio::spawn(async move {
            coordinator
                .request("Which backend?", vec!["A".into(), "B".into()], false, None)
                .await
        });
        let question_id = pending_question_id(&state.ask_user).await;

        let app = ask_user_routes().with_state(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/ask-user/questions")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let questions = value["questions"].as_array().unwrap();
        assert_eq!(questions.len(), 1);
        assert_eq!(questions[0]["question"], "Which backend?");
        assert_eq!(questions[0]["question_id"], question_id);

        state.ask_user.cancel(&question_id).await.unwrap();
        let _ = ask.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn answer_returns_answered_and_unblocks() {
        let state = test_state();
        let coordinator = state.ask_user.clone();
        let ask = tokio::spawn(async move {
            coordinator
                .request("Pick one?", vec!["A".into(), "B".into()], false, None)
                .await
        });
        let question_id = pending_question_id(&state.ask_user).await;

        let app = ask_user_routes().with_state(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/ask-user/questions/{question_id}/answer"))
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::json!({ "selected_index": 1 }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert!(String::from_utf8(body.to_vec())
            .unwrap()
            .contains("\"answered\""));

        let response = ask.await.unwrap().unwrap();
        assert_eq!(response.selected.as_deref(), Some("B"));
        assert_eq!(response.selected_index, Some(1));
    }

    #[tokio::test]
    async fn answer_other_text_requires_allow_other() {
        let state = test_state();
        let coordinator = state.ask_user.clone();
        let ask =
            tokio::spawn(
                async move { coordinator.request("Free text?", vec![], true, None).await },
            );
        let question_id = pending_question_id(&state.ask_user).await;

        let app = ask_user_routes().with_state(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/ask-user/questions/{question_id}/answer"))
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::json!({ "other_text": "custom" }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let response = ask.await.unwrap().unwrap();
        assert_eq!(response.other_text.as_deref(), Some("custom"));
    }

    #[tokio::test]
    async fn answer_unknown_question_is_not_found() {
        let state = test_state();
        let app = ask_user_routes().with_state(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/ask-user/questions/missing/answer")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::json!({ "selected_index": 0 }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn invalid_choice_is_unprocessable() {
        let state = test_state();
        let coordinator = state.ask_user.clone();
        let ask = tokio::spawn(async move {
            coordinator
                .request("Pick?", vec!["A".into()], false, None)
                .await
        });
        let question_id = pending_question_id(&state.ask_user).await;

        let app = ask_user_routes().with_state(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/ask-user/questions/{question_id}/answer"))
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::json!({ "selected_index": 9 }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

        state.ask_user.cancel(&question_id).await.unwrap();
        let _ = ask.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn cancel_returns_cancelled() {
        let state = test_state();
        let coordinator = state.ask_user.clone();
        let ask =
            tokio::spawn(async move { coordinator.request("Proceed?", vec![], false, None).await });
        let question_id = pending_question_id(&state.ask_user).await;

        let app = ask_user_routes().with_state(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/ask-user/questions/{question_id}/cancel"))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert!(String::from_utf8(body.to_vec())
            .unwrap()
            .contains("\"cancelled\""));

        let response = ask.await.unwrap().unwrap();
        assert_eq!(
            response.status,
            agent_diva_core::ask_user::AskUserStatus::Cancelled
        );
    }

    #[tokio::test]
    async fn expired_question_is_gone() {
        let state = test_state();
        let coordinator = AskUserCoordinator::new(Duration::from_millis(50));
        let ask = tokio::spawn({
            let coordinator = coordinator.clone();
            async move { coordinator.request("Hurry?", vec![], false, None).await }
        });
        let question_id = pending_question_id(&coordinator).await;
        let _ = ask.await.unwrap().unwrap_err();

        // The expired question is no longer pending, so answering it is 404.
        let app = ask_user_routes().with_state(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/ask-user/questions/{question_id}/answer"))
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::json!({ "selected_index": 0 }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
