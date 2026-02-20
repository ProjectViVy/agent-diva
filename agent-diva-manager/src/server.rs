use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tokio::sync::broadcast;

use crate::handlers::{chat_handler, get_config_handler, update_config_handler, get_providers_handler, get_channels_handler, update_channel_handler};
use crate::state::AppState;

pub async fn run_server(state: AppState, port: u16, mut shutdown_rx: broadcast::Receiver<()>) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/api/chat", post(chat_handler))
        .route("/api/config", get(get_config_handler).post(update_config_handler))
        .route("/api/providers", get(get_providers_handler))
        .route("/api/channels", get(get_channels_handler).post(update_channel_handler))
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
