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
    accept_persona_request_handler, accept_skill_request_handler, add_provider_model_handler,
    cancel_autodream_run_handler, chat_handler, create_cron_job_handler, create_mcp_handler,
    create_memory_record_handler, create_persona_request_handler, create_provider_handler,
    create_skill_request_handler, delete_actmem_capsule_handler, delete_cron_job_handler,
    delete_mcp_handler, delete_memory_record_handler, delete_provider_handler,
    delete_provider_model_handler, delete_session_handler, delete_skill_handler,
    disable_skill_handler, events_handler, featured_marketplace_skills_handler,
    generate_session_title_handler, get_actmem_capsule_handler, get_actmem_handler,
    get_audit_events_handler, get_audit_log_handler, get_autodream_live_text_handler,
    get_autodream_run_handler, get_channels_handler, get_config_handler, get_cron_job_handler,
    get_mcps_handler, get_memory_record_handler, get_memrules_handler,
    get_persona_document_handler, get_persona_history_revision_handler, get_persona_status_handler,
    get_provider_handler, get_provider_models_handler, get_providers_handler,
    get_self_evolution_config_handler, get_session_history_handler, get_sessions_handler,
    get_skill_handler, get_skill_history_revision_handler, get_skill_request_handler,
    get_skills_handler, get_tools_handler, get_workspace_handler, health_handler,
    heartbeat_handler, initialize_persona_handler, install_marketplace_skill_handler,
    list_actmem_capsules_handler, list_autodream_run_events_handler, list_autodream_runs_handler,
    list_cron_jobs_handler, list_memory_records_handler, list_persona_history_handler,
    list_persona_requests_handler, list_recall_feedback_handler, list_skill_history_handler,
    list_skill_requests_handler, logs_routes, put_actmem_handler, put_memrules_handler,
    refresh_mcp_status_handler, reject_persona_request_handler, reject_skill_request_handler,
    repair_persona_handler, reset_session_handler, resolve_provider_handler, run_cron_job_handler,
    save_persona_document_handler, search_marketplace_skills_handler, set_cron_job_enabled_handler,
    set_mcp_enabled_handler, stop_chat_handler, stop_cron_job_handler, todo_routes,
    token_stats_routes, trigger_autodream_run_handler, update_channel_handler,
    update_config_handler, update_cron_job_handler, update_mcp_handler,
    update_memory_record_handler, update_provider_handler, update_self_evolution_config_handler,
    update_session_title_handler, update_skill_handler, update_tools_handler, upload_file_handler,
    upload_skill_handler,
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
        .merge(crate::handlers::command_approval_routes())
        .merge(crate::handlers::approval_routes())
        .merge(crate::handlers::ask_user_routes())
        .merge(provider_routes())
        .merge(planning_routes())
        .merge(autodream_routes())
        .merge(laputa_routes())
        .merge(persona_routes())
        .merge(memory_routes())
        .merge(audit_routes())
        .merge(token_stats_routes())
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
            "/api/autodream/runs/:id/live-text",
            get(get_autodream_live_text_handler),
        )
        .route(
            "/api/autodream/runs/:id/events",
            get(list_autodream_run_events_handler),
        )
        .route(
            "/api/autodream/runs/:id/cancel",
            post(cancel_autodream_run_handler),
        )
}

pub(crate) fn memory_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/memory/records",
            get(list_memory_records_handler).post(create_memory_record_handler),
        )
        .route(
            "/api/memory/records/:id",
            get(get_memory_record_handler)
                .patch(update_memory_record_handler)
                .delete(delete_memory_record_handler),
        )
        .route(
            "/api/memory/actmem",
            get(get_actmem_handler).put(put_actmem_handler),
        )
        .route(
            "/api/memory/actmem/capsules",
            get(list_actmem_capsules_handler),
        )
        .route(
            "/api/memory/actmem/capsules/:name",
            get(get_actmem_capsule_handler).delete(delete_actmem_capsule_handler),
        )
        .route(
            "/api/memory/memrules",
            get(get_memrules_handler).put(put_memrules_handler),
        )
}

fn laputa_routes() -> Router<AppState> {
    Router::new().route(
        "/api/laputa/recall-feedback",
        get(list_recall_feedback_handler),
    )
}

fn persona_routes() -> Router<AppState> {
    Router::new()
        .route("/api/persona/status", get(get_persona_status_handler))
        .route("/api/persona/initialize", post(initialize_persona_handler))
        .route("/api/persona/repair", post(repair_persona_handler))
        .route(
            "/api/persona/docs/:kind",
            get(get_persona_document_handler).put(save_persona_document_handler),
        )
        .route(
            "/api/persona/docs/:kind/history",
            get(list_persona_history_handler),
        )
        .route(
            "/api/persona/docs/:kind/history/:revision",
            get(get_persona_history_revision_handler),
        )
        .route(
            "/api/persona/requests",
            get(list_persona_requests_handler).post(create_persona_request_handler),
        )
        .route(
            "/api/persona/requests/:id/accept",
            post(accept_persona_request_handler),
        )
        .route(
            "/api/persona/requests/:id/reject",
            post(reject_persona_request_handler),
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
        .route("/api/workspace", get(get_workspace_handler))
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
            "/api/skills",
            get(get_skills_handler).post(upload_skill_handler),
        )
        .route(
            "/api/skills/marketplace/search",
            get(search_marketplace_skills_handler),
        )
        .route(
            "/api/skills/marketplace/install",
            post(install_marketplace_skill_handler),
        )
        .route(
            "/api/skills/marketplace/featured",
            get(featured_marketplace_skills_handler),
        )
        .route(
            "/api/skills/:slug",
            get(get_skill_handler)
                .put(update_skill_handler)
                .delete(delete_skill_handler),
        )
        .route("/api/skills/:slug/disable", post(disable_skill_handler))
        .route("/api/skills/:slug/history", get(list_skill_history_handler))
        .route(
            "/api/skills/:slug/history/:revision",
            get(get_skill_history_revision_handler),
        )
        .route(
            "/api/evolution/requests",
            get(list_skill_requests_handler).post(create_skill_request_handler),
        )
        .route(
            "/api/evolution/requests/:id",
            get(get_skill_request_handler),
        )
        .route(
            "/api/evolution/requests/:id/accept",
            post(accept_skill_request_handler),
        )
        .route(
            "/api/evolution/requests/:id/reject",
            post(reject_skill_request_handler),
        )
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
        active_plan_execution_handler, append_plan_report_revision_handler,
        approve_plan_report_handler, create_plan_report_handler, list_execution_todos_handler,
        list_plan_reports_handler, update_execution_todo_handler,
    };
    Router::new()
        .route(
            "/api/plan-reports",
            get(list_plan_reports_handler).post(create_plan_report_handler),
        )
        .route(
            "/api/plan-reports/:report_id/revisions",
            post(append_plan_report_revision_handler),
        )
        .route(
            "/api/plan-reports/:report_id/approve",
            post(approve_plan_report_handler),
        )
        .route(
            "/api/plan-executions/active",
            get(active_plan_execution_handler),
        )
        .route(
            "/api/plan-executions/:execution_id/todos",
            get(list_execution_todos_handler),
        )
        .route(
            "/api/plan-executions/:execution_id/todos/:todo_id",
            patch(update_execution_todo_handler),
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
    use agent_diva_core::evolution::{AutoDreamFailureCode, AutoDreamRunState};
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use std::io::Write;
    use tower::util::ServiceExt;

    use crate::state::{AppState, ManagerCommand};

    const G0_RUNTIME_CONTRACT: &str = include_str!("../tests/fixtures/g0_runtime_contract.json");

    async fn json_response(
        app: axum::Router,
        request: Request<Body>,
    ) -> (StatusCode, serde_json::Value) {
        let response = app.oneshot(request).await.unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
        (status, value)
    }

    fn skill_zip(slug: &str) -> Vec<u8> {
        let mut cursor = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut cursor);
            writer
                .start_file("SKILL.md", zip::write::FileOptions::default())
                .unwrap();
            writer
                .write_all(
                    format!("---\nname: {slug}\ndescription: uploaded\n---\nbody\n").as_bytes(),
                )
                .unwrap();
            writer.finish().unwrap();
        }
        cursor.into_inner()
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
    async fn skill_evolution_http_routes_enforce_zip_cas_history_and_review() {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let state =
            AppState::new(api_tx, agent_diva_core::bus::MessageBus::new(), temp.path()).unwrap();
        let app = build_router(state.clone());

        let zip = skill_zip("zip-skill");
        let boundary = "skill-upload-boundary";
        let mut multipart = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"zip-skill.zip\"\r\nContent-Type: application/zip\r\n\r\n"
        )
        .into_bytes();
        multipart.extend_from_slice(&zip);
        multipart.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        let (status, uploaded) = json_response(
            app.clone(),
            Request::builder()
                .method("POST")
                .uri("/api/skills")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(multipart))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(uploaded["skill"]["slug"], "zip-skill");

        let (status, duplicate) = json_response(
            app.clone(),
            Request::builder()
                .method("PUT")
                .uri("/api/skills/zip-skill")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"markdown":"---
name: zip-skill
description: changed
---
body
","base_hash":"wrong"}"#,
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(duplicate["error"]["code"], "skill_hash_conflict");

        let markdown =
            "---\nname: reviewed-skill\ndescription: reviewed\nalways: true\n---\nreviewed body\n";
        let request_body = serde_json::json!({
            "slug": "reviewed-skill",
            "title": "Reviewed Skill",
            "proposed_markdown": markdown,
            "evidence": [],
            "attestation": "I authored and reviewed this Skill",
            "base_hash": "0",
            "reason": "user-authored reusable routine"
        });
        let (status, created) = json_response(
            app.clone(),
            Request::builder()
                .method("POST")
                .uri("/api/evolution/requests")
                .header("content-type", "application/json")
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let request_id = created["request"]["id"].as_str().unwrap();

        let (status, conflict) = json_response(
            app.clone(),
            Request::builder()
                .method("POST")
                .uri("/api/evolution/requests")
                .header("content-type", "application/json")
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(conflict["error"]["code"], "skill_request_exists");

        let (status, accepted) = json_response(
            app.clone(),
            Request::builder()
                .method("POST")
                .uri(format!("/api/evolution/requests/{request_id}/accept"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(accepted["request"]["status"], "accepted");

        let (status, detail) = json_response(
            app.clone(),
            Request::builder()
                .uri("/api/skills/reviewed-skill")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(detail["skill"]["source"], "home");
        let hash = detail["skill"]["content_hash"].as_str().unwrap();

        let (status, history) = json_response(
            app.clone(),
            Request::builder()
                .uri("/api/skills/reviewed-skill/history")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(history["history"].as_array().unwrap().len(), 1);

        let (status, disabled) = json_response(
            app.clone(),
            Request::builder()
                .method("POST")
                .uri("/api/skills/reviewed-skill/disable")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "base_hash": hash }).to_string(),
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(disabled["outcome"]["document"]["enabled"], false);
        let disabled_hash = disabled["outcome"]["document"]["content_hash"]
            .as_str()
            .unwrap();

        let (status, _) = json_response(
            app,
            Request::builder()
                .method("DELETE")
                .uri("/api/skills/reviewed-skill")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "base_hash": disabled_hash }).to_string(),
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn persona_api_initializes_reads_and_enforces_cas() {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let state =
            AppState::new(api_tx, agent_diva_core::bus::MessageBus::new(), temp.path()).unwrap();
        let app = build_router(state);

        let status = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/persona/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(status.status(), StatusCode::OK);
        let status_body = to_bytes(status.into_body(), usize::MAX).await.unwrap();
        let status_json: serde_json::Value = serde_json::from_slice(&status_body).unwrap();
        assert_eq!(status_json["status"], "ok");
        assert_eq!(status_json["persona"]["status"], "uninitialized");

        let initialize = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/persona/initialize")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "identity": "Identity",
                            "relationship": "Relationship",
                            "redline": "Redline",
                            "user": "Concise answers",
                            "world": "WORLD remains user-owned"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(initialize.status(), StatusCode::OK);
        let initialize_body = to_bytes(initialize.into_body(), usize::MAX).await.unwrap();
        let initialize_json: serde_json::Value = serde_json::from_slice(&initialize_body).unwrap();
        assert_eq!(initialize_json["status"], "ok");
        assert_eq!(initialize_json["persona"]["status"], "ready");

        let document = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/persona/docs/identity")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(document.status(), StatusCode::OK);
        let document_body = to_bytes(document.into_body(), usize::MAX).await.unwrap();
        let document_json: serde_json::Value = serde_json::from_slice(&document_body).unwrap();
        assert_eq!(document_json["document"]["revision"], 1);

        let conflict = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/persona/docs/identity")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "content": "Changed",
                            "base_revision": 0,
                            "reason": "stale client"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(conflict.status(), StatusCode::CONFLICT);
        let conflict_body = to_bytes(conflict.into_body(), usize::MAX).await.unwrap();
        assert!(String::from_utf8_lossy(&conflict_body).contains("persona_revision_conflict"));
    }

    #[tokio::test]
    async fn g0_runtime_route_contract_matches_fixture() {
        let fixture: serde_json::Value = serde_json::from_str(G0_RUNTIME_CONTRACT).unwrap();
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let state =
            AppState::new(api_tx, agent_diva_core::bus::MessageBus::new(), temp.path()).unwrap();
        let app = build_router(state.clone());

        for route in fixture["routes"].as_array().unwrap() {
            let request = Request::builder()
                .method(route["method"].as_str().unwrap())
                .uri(route["path"].as_str().unwrap())
                .body(Body::empty())
                .unwrap();
            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(
                response.status().as_u16(),
                route["status_without_runtime"].as_u64().unwrap() as u16,
                "{} {}",
                route["method"],
                route["path"]
            );
        }
    }

    #[tokio::test]
    async fn build_router_keeps_legacy_laputa_governance_routes_removed() {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let state =
            AppState::new(api_tx, agent_diva_core::bus::MessageBus::new(), temp.path()).unwrap();

        let app = build_router(state.clone());

        for uri in [
            "/api/laputa/snapshot",
            "/api/laputa/persona-workspace",
            "/api/laputa/section/memory_md",
            "/api/laputa/section/memory_md/write",
            "/api/laputa/cognitive/memrules",
            "/api/laputa/proposals",
            "/api/laputa/proposals/p1/apply",
            "/api/laputa/proposals/p1/decision",
            "/api/laputa/changelog",
            "/api/laputa/changelog/c1/rollback",
            "/api/laputa/events/proposals",
            "/api/laputa/events/proposals/poll",
        ] {
            let response = app
                .clone()
                .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{uri}");
        }
    }

    #[tokio::test]
    async fn evolution_workspace_exposes_payload_free_recall_feedback() {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let state =
            AppState::new(api_tx, agent_diva_core::bus::MessageBus::new(), temp.path()).unwrap();
        let response = build_router(state)
            .oneshot(
                Request::builder()
                    .uri("/api/laputa/recall-feedback?limit=10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["status"], "ok");
        assert_eq!(value["feedback"], serde_json::json!([]));
    }

    #[tokio::test]
    async fn autodream_manual_run_executes_worker_and_returns_terminal_failure() {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let state =
            AppState::new(api_tx, agent_diva_core::bus::MessageBus::new(), temp.path()).unwrap();

        let app = build_router(state.clone());

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
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["run"]["state"], "pending");
        assert_eq!(body["run"]["orchestration"]["phase"], "queued");
        let run_id = body["run"]["id"].as_str().unwrap();
        let terminal = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let status = state.autodream.get_run_status(run_id).unwrap();
                if matches!(
                    status.run.state,
                    AutoDreamRunState::Completed
                        | AutoDreamRunState::Failed
                        | AutoDreamRunState::Cancelled
                ) {
                    break status;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(terminal.run.state, AutoDreamRunState::Failed);
        assert_eq!(
            terminal.run.failure_code,
            Some(AutoDreamFailureCode::InputUnavailable)
        );
        assert!(terminal
            .run
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("all mandatory inputs omitted"));
    }

    #[tokio::test]
    async fn autodream_manual_run_organizes_actmem_without_proposals() {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let mut sessions = agent_diva_core::session::SessionManager::new(temp.path());
        let session = sessions.get_or_create("chat:e0");
        session.add_message("user", "verified local E0 session evidence");
        let saved = session.clone();
        sessions.save(&saved).unwrap();
        let mut state =
            AppState::new(api_tx, agent_diva_core::bus::MessageBus::new(), temp.path()).unwrap();
        // Keep the S3/S4 vertical deterministic: no provider-backed skill engine.
        state.autodream = state.autodream.clone().with_skill_reflection_engine(None);
        let app = build_router(state.clone());

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
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["run"]["state"], "pending");
        assert_eq!(body["run"]["orchestration"]["phase"], "queued");
        let run_id = body["run"]["id"].as_str().unwrap();
        let terminal = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let status = state.autodream.get_run_status(run_id).unwrap();
                if matches!(
                    status.run.state,
                    AutoDreamRunState::Completed
                        | AutoDreamRunState::Failed
                        | AutoDreamRunState::Cancelled
                ) {
                    break status;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(terminal.run.state, AutoDreamRunState::Completed);
        assert!(terminal.run.proposal_ids.is_empty());
        assert!(
            std::fs::read_dir(state.workspace_root.join(".laputa/proposals"))
                .map(|entries| entries.count() == 0)
                .unwrap_or(true)
        );
        let document = state.memory_home.actmem().read().unwrap();
        assert!(document.work.contains("### Goal"));
        assert!(document.work.contains("### Pointers"));
        assert!(document.work.contains("MEMRULES:Default"));
    }

    #[tokio::test]
    async fn vertical_autodream_keeps_bml_and_governance_untouched_in_s3() {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let journal = agent_diva_core::experience::ExperienceJournal::open(temp.path());
        journal
            .append(&journal.tool_evidence(
                "gui:e7",
                "trace-e7",
                "tool-e7",
                "exec",
                agent_diva_core::experience::OutcomeKind::Succeeded,
            ))
            .unwrap();

        let mut state = AppState::new_with_runtime_memory(
            api_tx,
            agent_diva_core::bus::MessageBus::new(),
            temp.path(),
            agent_diva_sandbox::CommandApprovalCoordinator::default(),
            agent_diva_core::ask_user::AskUserCoordinator::default(),
        )
        .unwrap();
        // Keep the S3/S4 vertical deterministic: no provider-backed skill engine.
        state.autodream = state.autodream.clone().with_skill_reflection_engine(None);
        let app = build_router(state.clone());

        let triggered = app
            .clone()
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
        assert_eq!(triggered.status(), StatusCode::OK);
        let triggered: serde_json::Value =
            serde_json::from_slice(&to_bytes(triggered.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let run_id = triggered["run"]["id"].as_str().unwrap();
        let terminal = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let status = state.autodream.get_run_status(run_id).unwrap();
                if matches!(
                    status.run.state,
                    AutoDreamRunState::Completed
                        | AutoDreamRunState::Failed
                        | AutoDreamRunState::Cancelled
                ) {
                    break status;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(terminal.run.state, AutoDreamRunState::Completed);
        assert!(terminal.run.proposal_ids.is_empty());
        assert!(
            std::fs::read_dir(state.workspace_root.join(".laputa/proposals"))
                .map(|entries| entries.count() == 0)
                .unwrap_or(true)
        );
        assert!(!state.memory_home.database_path().exists());
        let document = state.memory_home.actmem().read().unwrap();
        assert!(document.work.contains("evidence:"));
    }
}
