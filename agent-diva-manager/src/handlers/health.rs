//! Health and heartbeat HTTP endpoints for the manager API.

use std::collections::BTreeMap;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;

use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub uptime_secs: u64,
    pub components: BTreeMap<&'static str, ComponentHealth>,
    pub memory: MemoryHealth,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryHealth {
    pub status: &'static str,
    pub degraded_reason: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComponentHealth {
    pub status: &'static str,
    pub ready: Option<bool>,
    pub critical: bool,
}

pub async fn health_handler(State(state): State<AppState>) -> impl IntoResponse {
    let uptime_secs = state.started_at.elapsed().as_secs();
    let workspace_ready =
        state.workspace_root.exists() && std::fs::metadata(&state.workspace_root).is_ok();
    let audit_sink_ready = state.audit_root.exists() && state.health.audit_sink_ready();
    let event_bus_ready = true;
    let cron_ready = state.health.cron_ready();
    let memory = memory_health(&state).await;

    let mut components = BTreeMap::new();
    components.insert("audit_sink", component_from_bool(audit_sink_ready, true));
    components.insert("cron", component_from_option(cron_ready, true));
    components.insert("event_bus", component_from_bool(event_bus_ready, true));
    components.insert("workspace", component_from_bool(workspace_ready, true));
    components.insert(
        "memory",
        component_from_bool(memory.status == "ready", true),
    );

    let all_critical_ready = components
        .values()
        .filter(|component| component.critical)
        .all(|component| component.ready == Some(true));
    let status = if all_critical_ready { "ok" } else { "degraded" };
    let http_status = if all_critical_ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        http_status,
        Json(HealthResponse {
            status,
            version: env!("CARGO_PKG_VERSION"),
            uptime_secs,
            components,
            memory,
        }),
    )
}

/// Memory health probes the machine-wide MemoryHome introduced by the
/// cognitive clean break. The retired workspace-rooted typed store and the
/// proposal-governance metrics are no longer production dependencies.
async fn memory_health(state: &AppState) -> MemoryHealth {
    match state.memory_home.warmup().await {
        Ok(()) => MemoryHealth {
            status: "ready",
            degraded_reason: None,
        },
        Err(_) => degraded_memory("memory_home_unavailable"),
    }
}

fn degraded_memory(reason: &'static str) -> MemoryHealth {
    MemoryHealth {
        status: "degraded",
        degraded_reason: Some(reason),
    }
}

fn component_from_bool(ready: bool, critical: bool) -> ComponentHealth {
    ComponentHealth {
        status: if ready { "ready" } else { "degraded" },
        ready: Some(ready),
        critical,
    }
}

fn component_from_option(ready: Option<bool>, critical: bool) -> ComponentHealth {
    match ready {
        Some(ready) => component_from_bool(ready, critical),
        None => ComponentHealth {
            status: "unknown",
            ready: None,
            critical,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::bus::AgentEventBus;
    use axum::body::to_bytes;
    use axum::http::Request;
    use axum::Router;
    use std::time::{Duration, Instant};
    use tokio::sync::mpsc;
    use tower::util::ServiceExt;

    fn test_app(mark_cron_ready: bool) -> Router {
        let (api_tx, _api_rx) = mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().to_path_buf();
        std::mem::forget(temp);
        let state = AppState::new_with_runtime_memory(
            api_tx,
            AgentEventBus::new(),
            workspace,
            agent_diva_sandbox::CommandApprovalCoordinator::default(),
            agent_diva_core::ask_user::AskUserCoordinator::default(),
        )
        .unwrap();
        if mark_cron_ready {
            state.health.mark_cron_ready();
        }
        Router::new()
            .route("/api/health", axum::routing::get(health_handler))
            .with_state(state)
    }

    #[tokio::test]
    async fn health_returns_503_when_runtime_readiness_is_unknown() {
        let response = test_app(false)
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn memory_health_stays_ready_when_machine_home_database_is_absent() {
        let (api_tx, _api_rx) = mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let state = AppState::new_with_runtime_memory(
            api_tx,
            AgentEventBus::new(),
            temp.path(),
            agent_diva_sandbox::CommandApprovalCoordinator::default(),
            agent_diva_core::ask_user::AskUserCoordinator::default(),
        )
        .unwrap();
        let health = memory_health(&state).await;
        assert_eq!(health.status, "ready");
        assert_eq!(health.degraded_reason, None);
    }

    #[tokio::test]
    async fn health_returns_200_when_all_critical_components_are_ready() {
        let response = test_app(true)
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
    async fn health_response_uses_readiness_contract() {
        let response = test_app(false)
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

        assert_eq!(value["status"], "degraded");
        assert!(value["version"].is_string());
        assert!(value["uptime_secs"].is_number());
        assert!(value["components"]["audit_sink"]["ready"].is_boolean());
        assert_eq!(value["components"]["cron"]["status"], "unknown");
        assert!(value["memory"]["status"].is_string());
        assert!(value.get("database").is_none());
    }

    #[tokio::test]
    #[ignore = "run via `just health-benchmark-check` outside the parallel workspace suite"]
    async fn health_benchmark_ci_gate_stays_within_budget() {
        let app = test_app(true);
        let iterations = 500usize;
        let started = Instant::now();

        for _ in 0..iterations {
            let response = app
                .clone()
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

        let elapsed = started.elapsed();
        // The full workspace gate runs many test binaries concurrently on
        // Windows. Keep a strict 10 ms/request ceiling while allowing scheduler
        // contention that does not represent health-handler degradation.
        assert!(
            elapsed < Duration::from_secs(5),
            "health benchmark CI gate exceeded budget: {:?} for {} requests",
            elapsed,
            iterations
        );
    }
}
