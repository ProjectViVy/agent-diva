use axum::{
    routing::{delete, get, post},
    Router,
};
use std::net::SocketAddr;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::handlers::{
    add_provider_model_handler, chat_handler, create_cron_job_handler, create_mcp_handler,
    create_provider_handler, delete_cron_job_handler, delete_mcp_handler, delete_provider_handler,
    delete_provider_model_handler, delete_session_handler, delete_skill_handler, events_handler,
    get_channels_handler, get_config_handler, get_cron_job_handler, get_mcps_handler,
    get_provider_handler, get_provider_models_handler, get_providers_handler,
    get_session_history_handler, get_sessions_handler, get_skills_handler, get_swarm_cortex_doctor_handler,
    get_swarm_cortex_handler, get_tools_handler, heartbeat_handler, list_cron_jobs_handler,
    refresh_mcp_status_handler,
    post_capability_manifest_handler, reset_session_handler, resolve_provider_handler,
    run_cron_job_handler, set_cron_job_enabled_handler, set_mcp_enabled_handler,
    set_swarm_cortex_handler,
    stop_chat_handler, stop_cron_job_handler, update_channel_handler, update_config_handler,
    update_cron_job_handler, update_mcp_handler, update_provider_handler, update_tools_handler,
    upload_skill_handler,
};
use crate::state::AppState;

pub async fn run_server(
    state: AppState,
    port: u16,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> anyhow::Result<()> {
    let app = build_app(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.recv().await;
            tracing::info!("Server shutting down signal received");
        })
        .await?;

    Ok(())
}

fn build_app(state: AppState) -> Router {
    Router::new()
        .merge(runtime_routes())
        .merge(provider_routes())
        .merge(misc_routes())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

fn runtime_routes() -> Router<AppState> {
    Router::new()
        .route("/api/chat", post(chat_handler))
        .route("/api/chat/stop", post(stop_chat_handler))
        .route(
            "/api/swarm/cortex",
            get(get_swarm_cortex_handler).post(set_swarm_cortex_handler),
        )
        .route(
            "/api/diagnostics/swarm-doctor",
            get(get_swarm_cortex_doctor_handler),
        )
        .route(
            "/api/capabilities/manifest",
            post(post_capability_manifest_handler),
        )
        .route("/api/events", get(events_handler))
        .route("/api/sessions", get(get_sessions_handler))
        .route(
            "/api/sessions/:id",
            get(get_session_history_handler)
                .delete(delete_session_handler)
                .post(delete_session_handler),
        )
        .route("/api/sessions/reset", post(reset_session_handler))
        .route(
            "/api/config",
            get(get_config_handler).post(update_config_handler),
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
        .route("/api/skills/:name", delete(delete_skill_handler))
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

fn misc_routes() -> Router<AppState> {
    Router::new().route("/api/health", get(heartbeat_handler))
}

#[cfg(test)]
mod tests {
    use super::build_app;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::util::ServiceExt;

    use crate::state::{AppState, ManagerCommand};
    use agent_diva_agent::capability::{parse_and_validate_manifest_from_str, PlaceholderCapabilityRegistry};
    use agent_diva_core::config::schema::Config;
    use agent_diva_agent::swarm_doctor::swarm_cortex_doctor_for_gateway;
    use agent_diva_swarm::CortexState;
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn build_app_keeps_health_and_skills_routes_without_overlap() {
        let (api_tx, mut api_rx) = tokio::sync::mpsc::channel(1);
        let capability_registry = Arc::new(PlaceholderCapabilityRegistry::new());
        let capability_registry_task = capability_registry.clone();
        let capability_manifest_bootstrap_error = Arc::new(Mutex::new(None));
        let bootstrap_err_task = capability_manifest_bootstrap_error.clone();
        // get_skills_handler sends GetSkills on api_tx and awaits a oneshot reply; without a
        // consumer the request would hang forever.
        tokio::spawn(async move {
            while let Some(cmd) = api_rx.recv().await {
                match cmd {
                    ManagerCommand::GetSkills(tx) => {
                        let _ = tx.send(Ok(vec![]));
                    }
                    ManagerCommand::GetCortex(tx) => {
                        let _ = tx.send(Ok(CortexState::default()));
                    }
                    ManagerCommand::SetCortex(_, tx) => {
                        let _ = tx.send(Ok(()));
                    }
                    ManagerCommand::GetSwarmCortexDoctor(tx) => {
                        let cfg = Config::default();
                        let note = bootstrap_err_task
                            .lock()
                            .expect("test mutex")
                            .clone();
                        let _ = tx.send(Ok(swarm_cortex_doctor_for_gateway(
                            &cfg,
                            capability_registry_task.as_ref(),
                            note.as_deref(),
                        )));
                    }
                    _ => {}
                }
            }
        });
        let tmp = tempfile::tempdir().unwrap();
        let state = AppState {
            api_tx,
            bus: agent_diva_core::bus::MessageBus::new(),
            capability_registry,
            gateway_workspace: tmp.path().to_path_buf(),
            capability_manifest_bootstrap_error,
        };

        let app = build_app(state.clone());

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
        assert_eq!(health_response.status(), StatusCode::OK);

        let skills_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/skills")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(skills_response.status(), StatusCode::OK);

        let cortex_response = app
            .oneshot(
                Request::builder()
                    .uri("/api/swarm/cortex")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(cortex_response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn get_swarm_doctor_reflects_app_state_registry() {
        let tmp = tempfile::tempdir().unwrap();
        let reg = Arc::new(PlaceholderCapabilityRegistry::new());
        let m = parse_and_validate_manifest_from_str(
            r#"{"schema_version":"0","capabilities":[{"id":"route-cap","name":"R"}]}"#,
        )
        .unwrap();
        reg.register(m).unwrap();
        let reg_task = reg.clone();
        let bootstrap_err = Arc::new(Mutex::new(None));
        let bootstrap_task = bootstrap_err.clone();
        let (api_tx, mut api_rx) = tokio::sync::mpsc::channel(16);
        tokio::spawn(async move {
            while let Some(cmd) = api_rx.recv().await {
                if let ManagerCommand::GetSwarmCortexDoctor(tx) = cmd {
                    let cfg = Config::default();
                    let note = bootstrap_task.lock().expect("test mutex").clone();
                    let _ = tx.send(Ok(swarm_cortex_doctor_for_gateway(
                        &cfg,
                        reg_task.as_ref(),
                        note.as_deref(),
                    )));
                }
            }
        });
        let state = AppState {
            api_tx,
            bus: agent_diva_core::bus::MessageBus::new(),
            capability_registry: reg,
            gateway_workspace: tmp.path().to_path_buf(),
            capability_manifest_bootstrap_error: bootstrap_err,
        };
        let app = build_app(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/diagnostics/swarm-doctor")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["status"], "ok");
        assert_eq!(v["swarm_cortex"]["capabilities"]["count"], 1);
        assert_eq!(v["swarm_cortex"]["capabilities"]["source"], "process_registry");
    }

    #[tokio::test]
    async fn post_capability_manifest_persists_and_updates_registry() {
        let tmp = tempfile::tempdir().unwrap();
        let reg = Arc::new(PlaceholderCapabilityRegistry::new());
        let (api_tx, mut api_rx) = tokio::sync::mpsc::channel(8);
        tokio::spawn(async move {
            while let Some(_cmd) = api_rx.recv().await {}
        });
        let state = AppState {
            api_tx,
            bus: agent_diva_core::bus::MessageBus::new(),
            capability_registry: reg.clone(),
            gateway_workspace: tmp.path().to_path_buf(),
            capability_manifest_bootstrap_error: Arc::new(Mutex::new(None)),
        };
        let app = build_app(state);
        let body_json =
            r#"{"schema_version":"0","capabilities":[{"id":"http-int","name":"Integration"}]}"#;
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/capabilities/manifest")
                    .header("content-type", "application/json")
                    .body(Body::from(body_json))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["status"], "ok");
        assert_eq!(v["summary"]["count"], 1);
        let manifest_path = tmp
            .path()
            .join(".agent-diva")
            .join("capability-manifest.json");
        assert!(manifest_path.exists());
        assert_eq!(reg.summary().count, 1);
    }
}
