use axum::{
    extract::DefaultBodyLimit,
    routing::{delete, get, patch, post},
    Router,
};
use std::net::SocketAddr;
use tokio::sync::{broadcast, watch};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::handlers::{
    add_provider_model_handler, apply_laputa_proposal_handler, cancel_autodream_run_handler,
    chat_handler, create_cron_job_handler, create_laputa_proposal_handler, create_mcp_handler,
    create_provider_handler, delete_cron_job_handler, delete_mcp_handler, delete_provider_handler,
    delete_provider_model_handler, delete_session_handler, delete_skill_handler,
    edit_laputa_proposal_handler, events_handler, generate_session_title_handler,
    get_audit_events_handler, get_audit_log_handler, get_autodream_run_handler,
    get_channels_handler, get_config_handler, get_cron_job_handler, get_laputa_changelog_handler,
    get_laputa_proposal_handler, get_laputa_section_handler, get_laputa_snapshot_handler,
    get_mcps_handler, get_provider_handler, get_provider_models_handler, get_providers_handler,
    get_self_evolution_config_handler, get_session_history_handler, get_sessions_handler,
    get_skills_handler, get_tools_handler, health_handler, heartbeat_handler,
    list_autodream_runs_handler, list_cron_jobs_handler, list_laputa_changelog_handler,
    list_laputa_proposals_handler, list_mentle_tools_handler, logs_routes,
    poll_laputa_events_handler, refresh_mcp_status_handler, reset_session_handler,
    resolve_provider_handler, rollback_laputa_changelog_handler, run_cron_job_handler,
    set_cron_job_enabled_handler, set_mcp_enabled_handler, stop_chat_handler,
    stop_cron_job_handler, stream_laputa_events_handler, todo_routes,
    transition_laputa_proposal_handler, trigger_autodream_run_handler, update_channel_handler,
    update_config_handler, update_cron_job_handler, update_mcp_handler, update_provider_handler,
    update_self_evolution_config_handler, update_session_title_handler, update_tools_handler,
    upload_file_handler, upload_skill_handler, write_laputa_section_handler,
};
use crate::state::AppState;

pub async fn run_server(
    state: AppState,
    port: u16,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> anyhow::Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    run_server_with_listener_and_signal(state, listener, async move {
        let _ = shutdown_rx.recv().await;
        tracing::info!("Server shutting down signal received");
    })
    .await
}

pub async fn run_server_with_listener(
    state: AppState,
    listener: tokio::net::TcpListener,
    mut shutdown_rx: watch::Receiver<bool>,
) -> anyhow::Result<()> {
    let listen_addr = listener.local_addr()?;
    tracing::info!("Listening on {}", listen_addr);

    run_server_with_listener_and_signal(state, listener, async move {
        let _ = shutdown_rx.wait_for(|value| *value).await;
        tracing::info!("Server shutting down signal received");
    })
    .await
}

async fn run_server_with_listener_and_signal<F>(
    state: AppState,
    listener: tokio::net::TcpListener,
    shutdown_signal: F,
) -> anyhow::Result<()>
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    let app = build_router(state);
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown_signal.await;
        })
        .await?;
    Ok(())
}

/// Build the axum router with all manager HTTP routes.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .merge(runtime_routes())
        .merge(provider_routes())
        .merge(planning_routes())
        .merge(autodream_routes())
        .merge(laputa_routes())
        .merge(audit_routes())
        .merge(todo_routes())
        .merge(misc_routes())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

fn autodream_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/autodream/runs",
            get(list_autodream_runs_handler).post(trigger_autodream_run_handler),
        )
        .route("/api/autodream/runs/:id", get(get_autodream_run_handler))
        .route(
            "/api/autodream/runs/:id/cancel",
            post(cancel_autodream_run_handler),
        )
}

fn laputa_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/laputa/proposals",
            get(list_laputa_proposals_handler).post(create_laputa_proposal_handler),
        )
        .route(
            "/api/laputa/proposals/:id",
            get(get_laputa_proposal_handler).put(edit_laputa_proposal_handler),
        )
        .route(
            "/api/laputa/proposals/:id/transition",
            post(transition_laputa_proposal_handler),
        )
        .route(
            "/api/laputa/proposals/:id/apply",
            post(apply_laputa_proposal_handler),
        )
        .route("/api/laputa/snapshot", get(get_laputa_snapshot_handler))
        .route("/api/laputa/section/:name", get(get_laputa_section_handler))
        .route(
            "/api/laputa/section/:name/write",
            post(write_laputa_section_handler),
        )
        .route("/api/laputa/changelog", get(list_laputa_changelog_handler))
        .route(
            "/api/laputa/changelog/:id",
            get(get_laputa_changelog_handler),
        )
        .route(
            "/api/laputa/changelog/:id/rollback",
            post(rollback_laputa_changelog_handler),
        )
        .route(
            "/api/laputa/events/:kind",
            get(stream_laputa_events_handler),
        )
        .route(
            "/api/laputa/events/:kind/poll",
            get(poll_laputa_events_handler),
        )
}

fn runtime_routes() -> Router<AppState> {
    Router::new()
        .route("/api/chat", post(chat_handler))
        .route("/api/chat/stop", post(stop_chat_handler))
        .route("/api/events", get(events_handler))
        .route("/api/sessions", get(get_sessions_handler))
        .route(
            "/api/sessions/:id",
            get(get_session_history_handler)
                .delete(delete_session_handler)
                .post(delete_session_handler),
        )
        .route(
            "/api/sessions/:id/title",
            patch(update_session_title_handler),
        )
        .route(
            "/api/sessions/:id/generate-title",
            post(generate_session_title_handler),
        )
        .route("/api/sessions/reset", post(reset_session_handler))
        .route(
            "/api/config",
            get(get_config_handler).post(update_config_handler),
        )
        .route(
            "/api/config/self-evolution",
            get(get_self_evolution_config_handler).post(update_self_evolution_config_handler),
        )
        .route(
            "/api/channels",
            get(get_channels_handler).post(update_channel_handler),
        )
        .route(
            "/api/tools",
            get(get_tools_handler).post(update_tools_handler),
        )
        .route(
            "/api/tools/mentle/available",
            get(list_mentle_tools_handler),
        )
        .route(
            "/api/skills",
            get(get_skills_handler).post(upload_skill_handler),
        )
        .route("/api/skills/:name", delete(delete_skill_handler))
        .route(
            "/api/files/upload",
            post(upload_file_handler).layer(DefaultBodyLimit::max(50 * 1024 * 1024)),
        ) // 50MB limit
        .route("/api/mcps", get(get_mcps_handler).post(create_mcp_handler))
        .route(
            "/api/mcps/:name",
            axum::routing::put(update_mcp_handler).delete(delete_mcp_handler),
        )
        .route("/api/mcps/:name/enable", post(set_mcp_enabled_handler))
        .route("/api/mcps/:name/refresh", post(refresh_mcp_status_handler))
        .route(
            "/api/cron/jobs",
            get(list_cron_jobs_handler).post(create_cron_job_handler),
        )
        .route(
            "/api/cron/jobs/:id",
            get(get_cron_job_handler)
                .put(update_cron_job_handler)
                .delete(delete_cron_job_handler),
        )
        .route(
            "/api/cron/jobs/:id/enable",
            post(set_cron_job_enabled_handler),
        )
        .route("/api/cron/jobs/:id/run", post(run_cron_job_handler))
        .route("/api/cron/jobs/:id/stop", post(stop_cron_job_handler))
}

fn provider_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/providers",
            get(get_providers_handler).post(create_provider_handler),
        )
        .route("/api/providers/resolve", post(resolve_provider_handler))
        .route(
            "/api/providers/:name",
            get(get_provider_handler)
                .put(update_provider_handler)
                .delete(delete_provider_handler),
        )
        .route(
            "/api/providers/:name/models",
            get(get_provider_models_handler).post(add_provider_model_handler),
        )
        .route(
            "/api/providers/:name/models/:model_id",
            delete(delete_provider_model_handler),
        )
}

fn planning_routes() -> Router<AppState> {
    use crate::handlers::planning::{
        approve_active_plan_handler, create_plan_handler, delete_plan_handler,
        delete_plan_todo_handler, get_plan_handler, list_plans_handler, restore_plan_todo_handler,
        update_plan_handler,
    };
    Router::new()
        .route(
            "/api/plans",
            get(list_plans_handler).post(create_plan_handler),
        )
        .route(
            "/api/plans/active/approve-execute",
            post(approve_active_plan_handler),
        )
        .route(
            "/api/plans/:plan_id",
            get(get_plan_handler)
                .put(update_plan_handler)
                .delete(delete_plan_handler),
        )
        .route(
            "/api/plans/:plan_id/todos/:todo_id/delete",
            post(delete_plan_todo_handler),
        )
        .route(
            "/api/plans/:plan_id/todos/:todo_id/restore",
            post(restore_plan_todo_handler),
        )
}

fn audit_routes() -> Router<AppState> {
    Router::new()
        .route("/api/audit/log", get(get_audit_log_handler))
        .route("/api/audit/events", get(get_audit_events_handler))
        .merge(logs_routes())
}

fn misc_routes() -> Router<AppState> {
    Router::new()
        .route("/api/health", get(health_handler))
        .route("/api/heartbeat", get(heartbeat_handler))
}

#[cfg(test)]
mod tests {
    use super::build_router;
    use agent_diva_core::evolution::{
        EvidenceRef, EvidenceSource, EvolutionProposal, LaputaSectionName, ProposalState,
        ProposalType, RiskLevel,
    };
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use chrono::{DateTime, Utc};
    use tower::util::ServiceExt;

    use crate::state::{AppState, ManagerCommand};

    fn ts(seconds: u32) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(&format!("2026-06-14T00:03:{seconds:02}Z"))
            .unwrap()
            .with_timezone(&Utc)
    }

    fn laputa_proposal(id: &str, patch: &str) -> EvolutionProposal {
        EvolutionProposal {
            id: id.to_string(),
            created_at: ts(2),
            updated_at: ts(2),
            created_by: "autodream".to_string(),
            proposal_type: ProposalType::MemoryPatch,
            target_section: LaputaSectionName::MemoryMd,
            evidence_refs: vec![EvidenceRef {
                id: "ev-1".to_string(),
                source: EvidenceSource::Session,
                uri: "session://ev-1".to_string(),
                excerpt: Some("bounded evidence".to_string()),
                hash: Some("hash-ev-1".to_string()),
                created_at: ts(1),
            }],
            proposed_patch: patch.to_string(),
            risk_level: RiskLevel::Medium,
            state: ProposalState::PendingReview,
            source_run_id: Some("run-1".to_string()),
        }
    }

    #[tokio::test]
    async fn build_router_keeps_health_and_skills_routes_without_overlap() {
        let (api_tx, mut api_rx) = tokio::sync::mpsc::channel(1);
        // get_skills_handler sends GetSkills on api_tx and awaits a oneshot reply; without a
        // consumer the request would hang forever.
        tokio::spawn(async move {
            while let Some(cmd) = api_rx.recv().await {
                if let ManagerCommand::GetSkills(tx) = cmd {
                    let _ = tx.send(Ok(vec![]));
                }
            }
        });
        let temp = tempfile::tempdir().unwrap();
        let state =
            AppState::new(api_tx, agent_diva_core::bus::MessageBus::new(), temp.path()).unwrap();

        let app = build_router(state.clone());

        let health_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(health_response.status(), StatusCode::SERVICE_UNAVAILABLE);

        let skills_response = app
            .oneshot(
                Request::builder()
                    .uri("/api/skills")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(skills_response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn build_router_exposes_laputa_routes() {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let state =
            AppState::new(api_tx, agent_diva_core::bus::MessageBus::new(), temp.path()).unwrap();

        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/laputa/snapshot")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn laputa_apply_preserves_typed_schema_incompatible_error_code() {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let state =
            AppState::new(api_tx, agent_diva_core::bus::MessageBus::new(), temp.path()).unwrap();
        state
            .laputa
            .create_proposal(laputa_proposal("proposal-1", "not-json"))
            .unwrap();
        state
            .laputa
            .transition_proposal("proposal-1", ProposalState::Approved, ts(3))
            .unwrap();

        let app = build_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/laputa/proposals/proposal-1/apply")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"actor":"reviewer"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["status"], "error");
        assert_eq!(value["code"], "schema_incompatible");
    }

    #[tokio::test]
    async fn build_router_exposes_autodream_manual_run_route() {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let state =
            AppState::new(api_tx, agent_diva_core::bus::MessageBus::new(), temp.path()).unwrap();

        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/autodream/runs")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"trigger":"manual"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn write_laputa_section_success() {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let state =
            AppState::new(api_tx, agent_diva_core::bus::MessageBus::new(), temp.path()).unwrap();

        let app = build_router(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/laputa/section/memory_md/write")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"content":"{\"note\":\"hello\"}"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["status"], "ok");
        assert!(
            value["changelog_id"]
                .as_str()
                .unwrap()
                .starts_with("changelog-"),
            "expected changelog id, got {}",
            value["changelog_id"]
        );
        assert!(value["applied_at"].as_str().is_some());

        let section = state
            .laputa
            .read_section(LaputaSectionName::MemoryMd)
            .unwrap();
        assert_eq!(section.content, serde_json::json!({"note": "hello"}));
    }

    #[tokio::test]
    async fn write_laputa_section_unknown() {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let state =
            AppState::new(api_tx, agent_diva_core::bus::MessageBus::new(), temp.path()).unwrap();

        let app = build_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/laputa/section/not_a_section/write")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"content":"{}"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["status"], "error");
        assert_eq!(value["code"], "unknown_section");
    }

    #[tokio::test]
    async fn write_laputa_section_malformed_json() {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let state =
            AppState::new(api_tx, agent_diva_core::bus::MessageBus::new(), temp.path()).unwrap();

        let app = build_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/laputa/section/memory_md/write")
                    .header("content-type", "application/json")
                    .body(Body::from("not-json"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn write_laputa_section_schema_incompatible() {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let state =
            AppState::new(api_tx, agent_diva_core::bus::MessageBus::new(), temp.path()).unwrap();

        let app = build_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/laputa/section/memory_md/write")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"content":"plain text"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["status"], "error");
        assert_eq!(value["code"], "schema_incompatible");
    }
}
