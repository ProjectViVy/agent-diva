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
    get_session_history_handler, get_sessions_handler, get_skills_handler, get_tools_handler,
    heartbeat_handler, list_cron_jobs_handler, refresh_mcp_status_handler, reset_session_handler,
    resolve_provider_handler, run_cron_job_handler, set_cron_job_enabled_handler,
    set_mcp_enabled_handler, stop_chat_handler, stop_cron_job_handler, update_channel_handler,
    update_config_handler, update_cron_job_handler, update_mcp_handler, update_provider_handler,
    update_tools_handler, upload_file_handler, upload_skill_handler,
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
        .route("/api/files/upload", post(upload_file_handler))
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

    use crate::state::AppState;

    #[tokio::test]
    async fn build_app_keeps_health_and_skills_routes_without_overlap() {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let state = AppState {
            api_tx,
            bus: agent_diva_core::bus::MessageBus::new(),
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
}
