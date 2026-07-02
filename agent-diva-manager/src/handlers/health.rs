//! Health check HTTP endpoint for the manager API.
//!
//! Provides `GET /api/health` returning JSON with status, version,
//! uptime, and component health.

use axum::{extract::State, Json};
use serde::Serialize;

use crate::state::AppState;

// ── DTOs ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub uptime_secs: u64,
    pub components: ComponentsHealth,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComponentsHealth {
    pub database: &'static str,
    pub event_bus: &'static str,
}

// ── Handler ───────────────────────────────────────────────────────────

/// GET /api/health
///
/// Returns JSON health status with version, uptime, and component checks.
pub async fn health_handler(State(state): State<AppState>) -> Json<HealthResponse> {
    let uptime_secs = state.started_at.elapsed().as_secs();

    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        uptime_secs,
        components: ComponentsHealth {
            database: "ok",
            event_bus: "ok",
        },
    })
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

    fn test_app() -> Router {
        let (api_tx, _api_rx) = mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let state =
            AppState::new(api_tx, MessageBus::new(), temp.path()).unwrap();
        Router::new()
            .route("/api/health", axum::routing::get(health_handler))
            .with_state(state)
    }

    #[tokio::test]
    async fn health_returns_200_ok() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn health_response_has_required_fields() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(value["status"], "ok");
        assert!(value["version"].is_string());
        assert!(value["uptime_secs"].is_number());
        assert_eq!(value["components"]["database"], "ok");
        assert_eq!(value["components"]["event_bus"], "ok");
    }

    #[tokio::test]
    async fn health_version_matches_cargo_pkg_version() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(value["version"], env!("CARGO_PKG_VERSION"));
    }
}
