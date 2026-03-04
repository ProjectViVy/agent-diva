use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::handlers::{
    chat_handler, events_handler, get_channels_handler, get_config_handler, get_providers_handler,
    get_tools_handler, heartbeat_handler, stop_chat_handler, reset_session_handler, test_channel_handler,
    update_channel_handler, update_config_handler, update_tools_handler, get_sessions_handler, get_session_history_handler,
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
