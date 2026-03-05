use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::handlers::{
    chat_handler, create_cron_job_handler, delete_cron_job_handler, events_handler,
    get_channels_handler, get_config_handler, get_cron_job_handler, get_providers_handler,
    get_session_history_handler, get_sessions_handler, get_tools_handler, heartbeat_handler,
    list_cron_jobs_handler, reset_session_handler, run_cron_job_handler,
    set_cron_job_enabled_handler, stop_chat_handler, stop_cron_job_handler, test_channel_handler,
    update_channel_handler, update_config_handler, update_cron_job_handler, update_tools_handler,
};
use crate::state::AppState;

pub async fn run_server(
    state: AppState,
    port: u16,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/api/chat", post(chat_handler))
        .route("/api/chat/stop", post(stop_chat_handler))
        .route("/api/sessions", get(get_sessions_handler))
        .route("/api/sessions/:id", get(get_session_history_handler))
        .route("/api/sessions/reset", post(reset_session_handler))
        .route("/api/events", get(events_handler))
        .route(
            "/api/config",
            get(get_config_handler).post(update_config_handler),
        )
        .route("/api/providers", get(get_providers_handler))
        .route(
            "/api/channels",
            get(get_channels_handler).post(update_channel_handler),
        )
        .route(
            "/api/tools",
            get(get_tools_handler).post(update_tools_handler),
        )
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
        .route("/api/channels/:name/test", post(test_channel_handler))
        .route("/api/health", get(heartbeat_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

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
