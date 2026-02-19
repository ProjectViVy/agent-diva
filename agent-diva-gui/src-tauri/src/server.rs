use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tauri::{AppHandle, Emitter};
use tower_http::cors::CorsLayer;

#[derive(Clone)]
struct AppState {
    app_handle: AppHandle,
}

#[derive(Deserialize)]
pub struct HookMessage {
    pub content: String,
    pub source: Option<String>,
}

#[derive(Serialize)]
pub struct HookResponse {
    pub status: String,
}

async fn handle_message(
    State(state): State<AppState>,
    Json(payload): Json<HookMessage>,
) -> (StatusCode, Json<HookResponse>) {
    println!("Received hook message: {}", payload.content);
    
    // Broadcast the message to all windows or a specific one
    if let Err(e) = state.app_handle.emit("external-message", payload.content) {
        eprintln!("Failed to emit event: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(HookResponse { status: "error".to_string() }));
    }

    (StatusCode::OK, Json(HookResponse { status: "received".to_string() }))
}

async fn handle_status() -> (StatusCode, Json<HookResponse>) {
    (StatusCode::OK, Json(HookResponse { status: "running".to_string() }))
}

pub async fn start_server(app_handle: AppHandle, port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state = AppState { app_handle };

    let app = Router::new()
        .route("/api/hook/message", post(handle_message))
        .route("/api/status", get(handle_status))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("HTTP Hook Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}
