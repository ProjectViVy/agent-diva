use crate::app_state::AgentState;
use crate::gateway_status::GatewayStatus;
use crate::process_utils;
use crate::shutdown_manager::ShutdownManager;
use agent_diva_cli::cli_runtime::{collect_status_report, CliRuntime, StatusReport};
use agent_diva_core::config::{Config, ConfigLoader};
use agent_diva_neuron::{LlmNeuron, NeuronNode, NeuronRequest};
use agent_diva_providers::{
    CustomProviderUpsert, LiteLLMClient, Message, ProviderAccess, ProviderCatalogService,
    ProviderModelCatalogView as SharedProviderModelCatalog, ProviderView as SharedProviderView,
};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use eventsource_stream::Eventsource;
use futures::{SinkExt, StreamExt};
use http::header::{HeaderValue, AUTHORIZATION};
use native_tls::TlsConnector;
use serde::{Deserialize, Serialize};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder, Window};
use tauri_plugin_store::StoreExt;
use tokio::process::Command as TokioCommand;
use tokio::sync::Mutex as AsyncMutex;
use tokio::time::timeout;
use tokio_tungstenite::connect_async_tls_with_config;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;
use tokio_tungstenite::{Connector, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info, warn};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(windows)]
fn configure_background_command(command: &mut TokioCommand) {
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn configure_background_command(_command: &mut TokioCommand) {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSpec {
    pub name: String,
    pub display_name: String,
    pub api_type: String,
    pub source: String,
    pub configured: bool,
    pub ready: bool,
    pub default_api_base: String,
    pub default_model: Option<String>,
    pub models: Vec<String>,
    pub custom_models: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderModelCatalog {
    pub provider: String,
    pub source: String,
    pub runtime_supported: bool,
    pub api_base: Option<String>,
    pub models: Vec<String>,
    pub custom_models: Vec<String>,
    pub warnings: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderModelTestResult {
    pub ok: bool,
    pub message: String,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomProviderPayload {
    pub id: String,
    pub display_name: String,
    pub api_type: String,
    pub api_key: String,
    pub api_base: Option<String>,
    pub default_model: Option<String>,
    pub models: Vec<String>,
    pub extra_headers: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfigSnapshot {
    pub provider: Option<String>,
    pub api_base: Option<String>,
    pub model: String,
    pub has_api_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDto {
    pub name: String,
    pub description: String,
    pub source: String,
    pub available: bool,
    pub active: bool,
    pub path: String,
    pub can_delete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAttachmentDto {
    pub file_id: String,
    pub filename: String,
    pub size: u64,
    pub mime_type: Option<String>,
    pub channel: String,
    pub message_id: Option<String>,
    pub uploaded_by: Option<String>,
    pub stored_at: String,
    pub ref_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConnectionStatusDto {
    pub state: String,
    pub connected: bool,
    pub applied: bool,
    pub tool_count: usize,
    pub error: Option<String>,
    pub checked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerDto {
    pub name: String,
    pub enabled: bool,
    pub transport: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: std::collections::HashMap<String, String>,
    pub url: String,
    pub tool_timeout: u64,
    pub status: McpConnectionStatusDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerPayload {
    pub name: String,
    pub enabled: bool,
    pub command: String,
    pub args: Vec<String>,
    pub env: std::collections::HashMap<String, String>,
    pub url: String,
    pub tool_timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WipeSummary {
    pub removed_paths: Vec<String>,
}

// Manager API bridge commands. These proxy companion/runtime HTTP APIs without
// depending on manager internals from the GUI host process.
#[tauri::command]
pub async fn get_providers(state: State<'_, AgentState>) -> Result<Vec<ProviderSpec>, String> {
    let views = state.get_provider_views().await?;
    let mut providers = Vec::with_capacity(views.len());
    for view in views {
        let models = state
            .get_provider_model_catalog(&view.id, false)
            .await
            .map(provider_models_from_catalog)
            .unwrap_or_default();
        providers.push(provider_spec_from_view(view, models.0, models.1));
    }

    Ok(providers)
}

#[tauri::command]
pub async fn create_custom_provider(
    payload: CustomProviderPayload,
    state: State<'_, AgentState>,
) -> Result<ProviderSpec, String> {
    let provider_id = payload.id.trim().to_string();
    let view = state
        .create_custom_provider(&CustomProviderUpsert {
            id: payload.id,
            display_name: payload.display_name,
            api_type: payload.api_type,
            api_key: payload.api_key,
            api_base: payload.api_base,
            default_model: payload.default_model,
            models: payload.models,
            extra_headers: payload.extra_headers,
        })
        .await
        .and_then(|provider| {
            provider.ok_or_else(|| format!("provider '{provider_id}' not found after save"))
        })?;
    let models = state
        .get_provider_model_catalog(&view.id, false)
        .await
        .map(provider_models_from_catalog)
        .unwrap_or_default();

    Ok(provider_spec_from_view(view, models.0, models.1))
}

#[tauri::command]
pub async fn delete_custom_provider(
    provider: String,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    state.delete_custom_provider(provider.trim()).await
}

#[tauri::command]
pub async fn add_provider_model(
    provider: String,
    model: String,
    state: State<'_, AgentState>,
) -> Result<ProviderModelCatalog, String> {
    let provider_id = provider.trim().to_string();
    let model_id = model.trim().to_string();
    state.add_provider_model(&provider_id, &model_id).await?;
    let updated = state
        .get_provider_model_catalog(&provider_id, false)
        .await?;
    Ok(provider_model_catalog_dto(updated))
}

#[tauri::command]
pub async fn remove_provider_model(
    provider: String,
    model: String,
    state: State<'_, AgentState>,
) -> Result<ProviderModelCatalog, String> {
    let provider_id = provider.trim().to_string();
    let model_id = model.trim().to_string();
    state.remove_provider_model(&provider_id, &model_id).await?;
    let updated = state
        .get_provider_model_catalog(&provider_id, false)
        .await?;
    Ok(provider_model_catalog_dto(updated))
}

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[derive(Deserialize, Serialize, Clone)]
struct ToolStartEvent {
    name: String,
    #[serde(alias = "args")]
    args_preview: String,
    #[serde(alias = "id")]
    call_id: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
struct ToolFinishEvent {
    name: String,
    result: String,
    #[serde(alias = "error")]
    is_error: Option<bool>,
    #[serde(alias = "id")]
    call_id: Option<String>,
}

#[derive(Deserialize)]
struct ToolDeltaEvent {
    delta: String,
}

#[derive(Deserialize)]
struct BackgroundFinalEvent {
    content: String,
}

#[derive(Serialize, Clone)]
struct StreamTextPayload {
    request_id: String,
    data: String,
}

#[derive(Serialize, Clone)]
struct StreamToolStartPayload {
    request_id: String,
    name: String,
    args_preview: String,
    call_id: Option<String>,
}

#[derive(Serialize, Clone)]
struct StreamToolFinishPayload {
    request_id: String,
    name: String,
    result: String,
    is_error: Option<bool>,
    call_id: Option<String>,
}

#[tauri::command]
pub async fn send_message(
    message: String,
    channel: Option<String>,
    #[allow(non_snake_case)] chatId: Option<String>,
    attachments: Option<Vec<String>>,
    #[allow(non_snake_case)] streamRequestId: String,
    window: Window,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    // Tauri v2 uses camelCase from frontend, convert to snake_case internally
    let chat_id = chatId;
    let stream_request_id = streamRequestId;
    info!("Sending message to API: {}", message);
    info!("Attachments: {:?}", attachments);

    let client = &state.client;
    let url = format!("{}/chat", state.api_base_url());

    let response = client
        .post(&url)
        .json(&serde_json::json!({
            "message": message,
            "channel": channel,
            "chat_id": chat_id,
            "attachments": attachments
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to connect to agent server: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let mut stream = response.bytes_stream().eventsource();

    while let Some(event) = stream.next().await {
        match event {
            Ok(event) => {
                match event.event.as_str() {
                    "delta" => {
                        let _ = window.emit(
                            "agent-response-delta",
                            StreamTextPayload {
                                request_id: stream_request_id.clone(),
                                data: event.data,
                            },
                        );
                    }
                    "reasoning_delta" => {
                        let _ = window.emit(
                            "agent-reasoning-delta",
                            StreamTextPayload {
                                request_id: stream_request_id.clone(),
                                data: event.data,
                            },
                        );
                    }
                    "tool_delta" => {
                        if let Ok(data) = serde_json::from_str::<ToolDeltaEvent>(&event.data) {
                            let _ = window.emit(
                                "agent-tool-delta",
                                StreamTextPayload {
                                    request_id: stream_request_id.clone(),
                                    data: data.delta,
                                },
                            );
                        }
                    }
                    "final" => {
                        let _ = window.emit(
                            "agent-response-complete",
                            StreamTextPayload {
                                request_id: stream_request_id.clone(),
                                data: event.data,
                            },
                        );
                    }
                    "tool_start" => {
                        if let Ok(data) = serde_json::from_str::<ToolStartEvent>(&event.data) {
                            let _ = window.emit(
                                "agent-tool-start",
                                StreamToolStartPayload {
                                    request_id: stream_request_id.clone(),
                                    name: data.name,
                                    args_preview: data.args_preview,
                                    call_id: data.call_id,
                                },
                            );
                        } else {
                            // Fallback if parsing fails
                            let _ = window.emit(
                                "agent-tool-start",
                                StreamToolStartPayload {
                                    request_id: stream_request_id.clone(),
                                    name: "unknown".to_string(),
                                    args_preview: event.data,
                                    call_id: None,
                                },
                            );
                        }
                    }
                    "tool_finish" => {
                        if let Ok(data) = serde_json::from_str::<ToolFinishEvent>(&event.data) {
                            let _ = window.emit(
                                "agent-tool-end",
                                StreamToolFinishPayload {
                                    request_id: stream_request_id.clone(),
                                    name: data.name,
                                    result: data.result,
                                    is_error: data.is_error,
                                    call_id: data.call_id,
                                },
                            );
                        } else {
                            let _ = window.emit(
                                "agent-tool-end",
                                StreamToolFinishPayload {
                                    request_id: stream_request_id.clone(),
                                    name: "unknown".to_string(),
                                    result: event.data,
                                    is_error: Some(false),
                                    call_id: None,
                                },
                            );
                        }
                    }
                    "error" => {
                        let _ = window.emit(
                            "agent-error",
                            StreamTextPayload {
                                request_id: stream_request_id.clone(),
                                data: event.data,
                            },
                        );
                    }
                    _ => {}
                }
            }
            Err(e) => {
                error!("Stream error: {}", e);
                let _ = window.emit(
                    "agent-error",
                    StreamTextPayload {
                        request_id: stream_request_id.clone(),
                        data: e.to_string(),
                    },
                );
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn stop_generation(
    channel: Option<String>,
    chat_id: Option<String>,
    state: State<'_, AgentState>,
) -> Result<bool, String> {
    let url = format!("{}/chat/stop", state.api_base_url());
    let payload = serde_json::json!({
        "channel": channel,
        "chat_id": chat_id
    });

    let response = state
        .client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to request stop: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid stop response: {}", e))?;

    let status_ok = value.get("status").and_then(|v| v.as_str()) == Some("ok");
    if !status_ok {
        let message = value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        return Err(format!("Stop request rejected: {}", message));
    }

    Ok(value
        .get("stopped")
        .and_then(|v| v.as_bool())
        .unwrap_or(true))
}

#[tauri::command]
pub async fn reset_session(
    channel: Option<String>,
    chat_id: Option<String>,
    state: State<'_, AgentState>,
) -> Result<bool, String> {
    let url = format!("{}/sessions/reset", state.api_base_url());
    let payload = serde_json::json!({
        "channel": channel,
        "chat_id": chat_id
    });

    let response = state
        .client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to request session reset: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid reset response: {}", e))?;

    let status_ok = value.get("status").and_then(|v| v.as_str()) == Some("ok");
    if !status_ok {
        let message = value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        return Err(format!("Reset request rejected: {}", message));
    }

    Ok(value.get("reset").and_then(|v| v.as_bool()).unwrap_or(true))
}

#[tauri::command]
pub async fn delete_session(chat_id: String, state: State<'_, AgentState>) -> Result<bool, String> {
    let id_encoded = urlencoding::encode(&chat_id);
    // Use POST /sessions/:id (same path as DELETE) - more reliable in some environments
    let url = format!("{}/sessions/{}", state.api_base_url(), id_encoded);

    let response = state
        .client
        .post(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to delete session: {}", e))?;

    if !response.status().is_success() {
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(false);
        }
        return Err(format!("Server returned error: {}", response.status()));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid delete session response: {}", e))?;

    let status_ok = value.get("status").and_then(|v| v.as_str()) == Some("ok");
    if !status_ok {
        let message = value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        return Err(format!("Delete session request rejected: {}", message));
    }

    Ok(value
        .get("deleted")
        .and_then(|v| v.as_bool())
        .unwrap_or(true))
}

#[tauri::command]
pub async fn get_sessions(state: State<'_, AgentState>) -> Result<serde_json::Value, String> {
    let url = format!("{}/sessions", state.api_base_url());

    let response = state
        .client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch sessions: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid get sessions response: {}", e))?;

    let status_ok = value.get("status").and_then(|v| v.as_str()) == Some("ok");
    if !status_ok {
        let message = value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        return Err(format!("Get sessions request rejected: {}", message));
    }

    Ok(value
        .get("sessions")
        .cloned()
        .unwrap_or(serde_json::Value::Array(vec![])))
}

#[tauri::command]
pub async fn get_session_history(
    chat_id: String,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, String> {
    // URL encode the chat_id in case it contains special characters like ':'
    let id_encoded = urlencoding::encode(&chat_id);
    let url = format!("{}/sessions/{}", state.api_base_url(), id_encoded);

    let response = state
        .client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch session history: {}", e))?;

    if !response.status().is_success() {
        // A 404 or other failure could mean no history
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(serde_json::Value::Null);
        }
        return Err(format!("Server returned error: {}", response.status()));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid get session history response: {}", e))?;

    let status_ok = value.get("status").and_then(|v| v.as_str()) == Some("ok");

    if !status_ok {
        let message = value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        return Err(format!("Get session history request rejected: {}", message));
    }

    Ok(value
        .get("session")
        .cloned()
        .unwrap_or(serde_json::Value::Null))
}

#[tauri::command]
pub async fn get_cron_jobs(state: State<'_, AgentState>) -> Result<serde_json::Value, String> {
    let url = format!("{}/cron/jobs", state.api_base_url());
    let response = state
        .client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch cron jobs: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid get cron jobs response: {}", e))?;

    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }

    Ok(value
        .get("jobs")
        .cloned()
        .unwrap_or(serde_json::Value::Array(vec![])))
}

#[tauri::command]
pub async fn get_cron_job(
    job_id: String,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, String> {
    let url = format!(
        "{}/cron/jobs/{}",
        state.api_base_url(),
        urlencoding::encode(&job_id)
    );
    let response = state
        .client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch cron job: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid get cron job response: {}", e))?;

    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }

    Ok(value.get("job").cloned().unwrap_or(serde_json::Value::Null))
}

#[tauri::command]
pub async fn create_cron_job(
    payload: serde_json::Value,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, String> {
    let url = format!("{}/cron/jobs", state.api_base_url());
    let response = state
        .client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to create cron job: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid create cron job response: {}", e))?;

    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }

    Ok(value.get("job").cloned().unwrap_or(serde_json::Value::Null))
}

#[tauri::command]
pub async fn update_cron_job(
    job_id: String,
    payload: serde_json::Value,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, String> {
    let url = format!(
        "{}/cron/jobs/{}",
        state.api_base_url(),
        urlencoding::encode(&job_id)
    );
    let response = state
        .client
        .put(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to update cron job: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid update cron job response: {}", e))?;

    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }

    Ok(value.get("job").cloned().unwrap_or(serde_json::Value::Null))
}

#[tauri::command]
pub async fn set_cron_job_enabled(
    job_id: String,
    enabled: bool,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, String> {
    let url = format!(
        "{}/cron/jobs/{}/enable",
        state.api_base_url(),
        urlencoding::encode(&job_id)
    );
    let response = state
        .client
        .post(&url)
        .json(&serde_json::json!({ "enabled": enabled }))
        .send()
        .await
        .map_err(|e| format!("Failed to update cron job status: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid cron job status response: {}", e))?;

    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }

    Ok(value.get("job").cloned().unwrap_or(serde_json::Value::Null))
}

#[tauri::command]
pub async fn run_cron_job(
    job_id: String,
    force: bool,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, String> {
    let url = format!(
        "{}/cron/jobs/{}/run",
        state.api_base_url(),
        urlencoding::encode(&job_id)
    );
    let response = state
        .client
        .post(&url)
        .json(&serde_json::json!({ "force": force }))
        .send()
        .await
        .map_err(|e| format!("Failed to run cron job: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid run cron job response: {}", e))?;

    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }

    Ok(value.get("job").cloned().unwrap_or(serde_json::Value::Null))
}

#[tauri::command]
pub async fn stop_cron_job_run(
    job_id: String,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, String> {
    let url = format!(
        "{}/cron/jobs/{}/stop",
        state.api_base_url(),
        urlencoding::encode(&job_id)
    );
    let response = state
        .client
        .post(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to stop cron job: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid stop cron job response: {}", e))?;

    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }

    Ok(value.get("run").cloned().unwrap_or(serde_json::Value::Null))
}

#[tauri::command]
pub async fn delete_cron_job(job_id: String, state: State<'_, AgentState>) -> Result<(), String> {
    let url = format!(
        "{}/cron/jobs/{}",
        state.api_base_url(),
        urlencoding::encode(&job_id)
    );
    let response = state
        .client
        .delete(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to delete cron job: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid delete cron job response: {}", e))?;

    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }

    Ok(())
}

#[tauri::command]
pub async fn start_background_stream(
    window: Window,
    state: State<'_, AgentState>,
    shutdown_manager: State<'_, ShutdownManager>,
) -> Result<(), String> {
    let client = state.client.clone();
    let cancel_token = shutdown_manager.cancel_token();
    let url = format!(
        "{}/events?channel=api&chat_prefix=cron:",
        state.api_base_url()
    );

    tauri::async_runtime::spawn(async move {
        loop {
            let response = tokio::select! {
                _ = cancel_token.cancelled() => {
                    info!("Background stream cancelled before next connection attempt");
                    break;
                }
                response = client.get(&url).send() => response,
            };

            let response = match response {
                Ok(resp) => resp,
                Err(e) => {
                    error!("Failed to connect background stream: {}", e);
                    tokio::select! {
                        _ = cancel_token.cancelled() => {
                            info!("Background stream cancelled during reconnect backoff");
                            break;
                        }
                        _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {}
                    }
                    continue;
                }
            };

            if !response.status().is_success() {
                error!("Background stream server error: {}", response.status());
                tokio::select! {
                    _ = cancel_token.cancelled() => {
                        info!("Background stream cancelled after server error");
                        break;
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {}
                }
                continue;
            }

            let mut stream = response.bytes_stream().eventsource();
            loop {
                let event = tokio::select! {
                    _ = cancel_token.cancelled() => {
                        info!("Background stream cancelled while reading events");
                        return;
                    }
                    event = stream.next() => event,
                };

                let Some(event) = event else {
                    break;
                };

                match event {
                    Ok(event) => match event.event.as_str() {
                        "final" => {
                            if let Ok(payload) =
                                serde_json::from_str::<BackgroundFinalEvent>(&event.data)
                            {
                                let _ = window.emit("agent-background-response", payload.content);
                            }
                        }
                        "error" => {
                            let _ = window.emit("agent-error", event.data);
                        }
                        _ => {}
                    },
                    Err(e) => {
                        error!("Background stream error: {}", e);
                        break;
                    }
                }
            }

            tokio::select! {
                _ = cancel_token.cancelled() => {
                    info!("Background stream cancelled before retry");
                    break;
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {}
            }
        }

        info!("Background stream task exited");
    });

    Ok(())
}

#[tauri::command]
pub async fn check_health(state: State<'_, AgentState>) -> Result<bool, String> {
    let url = format!("{}/health", state.api_base_url());

    let client = &state.client;
    let response = client
        .get(&url)
        .timeout(std::time::Duration::from_millis(1000))
        .send()
        .await
        .map_err(|e| format!("Health check failed: {}", e))?;

    Ok(response.status().is_success())
}

#[tauri::command]
pub async fn update_config(
    api_base: Option<String>,
    api_key: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    info!(
        "Updating config via API: model={:?}, base={:?}",
        model, api_base
    );
    state.reconfigure(api_base, api_key, provider, model).await
}

#[tauri::command]
pub async fn get_tools_config(state: State<'_, AgentState>) -> Result<serde_json::Value, String> {
    state.get_tools_config().await
}

#[tauri::command]
pub async fn list_mentle_tools(state: State<'_, AgentState>) -> Result<serde_json::Value, String> {
    state.list_mentle_tools().await
}

#[tauri::command]
pub async fn get_provider_models(
    provider: String,
    api_base: Option<String>,
    api_key: Option<String>,
    state: State<'_, AgentState>,
) -> Result<ProviderModelCatalog, String> {
    if api_base.is_none() && api_key.is_none() {
        let catalog = state
            .get_provider_model_catalog(provider.trim(), true)
            .await?;
        return Ok(provider_model_catalog_dto(catalog));
    }

    let loader = config_loader();
    let config = loader.load().unwrap_or_default();
    let mut access = ProviderCatalogService::new()
        .get_provider_access(&config, provider.trim())
        .unwrap_or_else(|| ProviderAccess::from_config(None));
    if let Some(api_base) = api_base
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
    {
        access.api_base = Some(api_base);
    }
    if let Some(api_key) = api_key
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        access.api_key = Some(api_key);
    }
    let catalog = ProviderCatalogService::new()
        .list_provider_models(&config, provider.trim(), true, Some(access))
        .await?;

    Ok(provider_model_catalog_dto(catalog))
}

#[tauri::command]
pub async fn test_provider_model(
    provider: String,
    model: String,
    api_base: Option<String>,
    api_key: Option<String>,
) -> Result<ProviderModelTestResult, String> {
    // Keep this local so the GUI can probe ad-hoc credentials without
    // introducing new manager API surface or mutating manager-managed config.
    let provider = provider.trim().to_string();
    let model = model.trim().to_string();
    if provider.is_empty() {
        return Err("provider must not be empty".to_string());
    }
    if model.is_empty() {
        return Err("model must not be empty".to_string());
    }

    let loader = config_loader();
    let config = loader.load().unwrap_or_default();
    let access = provider_access_for_test(&config, &provider, api_base, api_key);

    let client = LiteLLMClient::new(
        access.api_key,
        access.api_base,
        model.clone(),
        (!access.extra_headers.is_empty()).then(|| {
            access
                .extra_headers
                .into_iter()
                .collect::<std::collections::HashMap<_, _>>()
        }),
        Some(provider.clone()),
        config.agents.defaults.reasoning_effort.clone(),
    );
    let neuron = LlmNeuron::with_id(
        Arc::new(client),
        format!("provider-test:{provider}:{model}"),
    );
    let request = NeuronRequest::new(
        vec![Message::user(
            "Reply with a short connectivity confirmation for this model test.",
        )],
        16,
        0.0,
    )
    .with_model(model);

    let started = Instant::now();
    match neuron.run_once(request).await {
        Ok(_) => Ok(ProviderModelTestResult {
            ok: true,
            message: "Connection test succeeded.".to_string(),
            latency_ms: started.elapsed().as_millis() as u64,
        }),
        Err(error) => Ok(ProviderModelTestResult {
            ok: false,
            message: format!("Connection test failed: {error}"),
            latency_ms: started.elapsed().as_millis() as u64,
        }),
    }
}

#[tauri::command]
pub async fn get_skills(state: State<'_, AgentState>) -> Result<Vec<SkillDto>, String> {
    let value = state.get_skills().await?;
    serde_json::from_value(value).map_err(|e| format!("Invalid skills payload: {}", e))
}

#[tauri::command]
pub async fn get_mcps(state: State<'_, AgentState>) -> Result<Vec<McpServerDto>, String> {
    let value = state.get_mcps().await?;
    serde_json::from_value(value).map_err(|e| format!("Invalid MCP payload: {}", e))
}

#[tauri::command]
pub async fn create_mcp(
    payload: McpServerPayload,
    state: State<'_, AgentState>,
) -> Result<McpServerDto, String> {
    let url = format!("{}/mcps", state.api_base_url());
    let response = state
        .client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to create MCP: {}", e))?;
    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid create MCP response: {}", e))?;
    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }
    serde_json::from_value(value.get("mcp").cloned().unwrap_or(serde_json::Value::Null))
        .map_err(|e| format!("Invalid created MCP payload: {}", e))
}

#[tauri::command]
pub async fn update_mcp(
    name: String,
    payload: McpServerPayload,
    state: State<'_, AgentState>,
) -> Result<McpServerDto, String> {
    let url = format!(
        "{}/mcps/{}",
        state.api_base_url(),
        urlencoding::encode(&name)
    );
    let response = state
        .client
        .put(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to update MCP: {}", e))?;
    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid update MCP response: {}", e))?;
    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }
    serde_json::from_value(value.get("mcp").cloned().unwrap_or(serde_json::Value::Null))
        .map_err(|e| format!("Invalid updated MCP payload: {}", e))
}

#[tauri::command]
pub async fn delete_mcp(name: String, state: State<'_, AgentState>) -> Result<(), String> {
    let url = format!(
        "{}/mcps/{}",
        state.api_base_url(),
        urlencoding::encode(&name)
    );
    let response = state
        .client
        .delete(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to delete MCP: {}", e))?;
    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid delete MCP response: {}", e))?;
    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn set_mcp_enabled(
    name: String,
    enabled: bool,
    state: State<'_, AgentState>,
) -> Result<McpServerDto, String> {
    let url = format!(
        "{}/mcps/{}/enable",
        state.api_base_url(),
        urlencoding::encode(&name)
    );
    let response = state
        .client
        .post(&url)
        .json(&serde_json::json!({ "enabled": enabled }))
        .send()
        .await
        .map_err(|e| format!("Failed to toggle MCP: {}", e))?;
    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid toggle MCP response: {}", e))?;
    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }
    serde_json::from_value(value.get("mcp").cloned().unwrap_or(serde_json::Value::Null))
        .map_err(|e| format!("Invalid toggled MCP payload: {}", e))
}

#[tauri::command]
pub async fn refresh_mcp_status(
    name: String,
    state: State<'_, AgentState>,
) -> Result<McpServerDto, String> {
    let url = format!(
        "{}/mcps/{}/refresh",
        state.api_base_url(),
        urlencoding::encode(&name)
    );
    let response = state
        .client
        .post(&url)
        .json(&serde_json::json!({ "reapply": true }))
        .send()
        .await
        .map_err(|e| format!("Failed to refresh MCP: {}", e))?;
    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid refresh MCP response: {}", e))?;
    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }
    serde_json::from_value(value.get("mcp").cloned().unwrap_or(serde_json::Value::Null))
        .map_err(|e| format!("Invalid refreshed MCP payload: {}", e))
}

#[tauri::command]
pub async fn upload_skill(
    file_name: String,
    bytes: Vec<u8>,
    state: State<'_, AgentState>,
) -> Result<SkillDto, String> {
    let url = format!("{}/skills", state.api_base_url());
    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(file_name)
        .mime_str("application/zip")
        .map_err(|e| format!("Failed to build upload part: {}", e))?;
    let form = reqwest::multipart::Form::new().part("file", part);
    let response = state
        .client
        .post(&url)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Failed to upload skill: {}", e))?;

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid upload response: {}", e))?;
    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }
    serde_json::from_value(
        value
            .get("skill")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    )
    .map_err(|e| format!("Invalid uploaded skill payload: {}", e))
}

#[tauri::command]
pub async fn upload_file(
    file_name: String,
    bytes: Vec<u8>,
    channel: String,
    message_id: Option<String>,
    state: State<'_, AgentState>,
) -> Result<FileAttachmentDto, String> {
    let url = format!("{}/files/upload", state.api_base_url());
    let file_part = reqwest::multipart::Part::bytes(bytes)
        .file_name(file_name)
        .mime_str("application/octet-stream")
        .map_err(|e| format!("Failed to build file part: {}", e))?;
    let channel_part = reqwest::multipart::Part::text(channel);
    // Use provided message_id or generate a temporary one for GUI uploads
    let message_id = message_id.unwrap_or_else(|| {
        format!(
            "gui_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        )
    });
    let message_id_part = reqwest::multipart::Part::text(message_id);
    let form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .part("channel", channel_part)
        .part("message_id", message_id_part);
    let response = state
        .client
        .post(&url)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Failed to upload file: {}", e))?;

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid upload response: {}", e))?;
    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }
    serde_json::from_value(
        value
            .get("attachment")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    )
    .map_err(|e| format!("Invalid uploaded file payload: {}", e))
}

#[tauri::command]
pub async fn delete_skill(name: String, state: State<'_, AgentState>) -> Result<(), String> {
    let name = urlencoding::encode(&name);
    let url = format!("{}/skills/{}", state.api_base_url(), name);
    let response = state
        .client
        .delete(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to delete skill: {}", e))?;
    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid delete skill response: {}", e))?;
    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn update_tools_config(
    tools: serde_json::Value,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    state.update_tools_config(tools).await
}

#[tauri::command]
pub async fn get_channels(state: State<'_, AgentState>) -> Result<serde_json::Value, String> {
    let url = format!("{}/channels", state.api_base_url());

    let response = state
        .client
        .get(&url)
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch channels: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server error: {}", response.status()));
    }

    let channels: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    Ok(channels)
}

#[tauri::command]
pub async fn update_channel(
    name: String,
    enabled: Option<bool>,
    config: serde_json::Value,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    let url = format!("{}/channels", state.api_base_url());

    let payload = serde_json::json!({
        "name": name,
        "enabled": enabled,
        "config": config
    });

    let response = state
        .client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to update channel: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server error: {}", response.status()));
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeInfo {
    pub platform: String,
    pub is_bundled: bool,
    pub resource_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GatewayProcessStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub executable_path: Option<String>,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatusPayload {
    pub installed: bool,
    pub running: bool,
    pub state: String,
    pub executable_path: Option<String>,
    pub details: Option<String>,
}

// Local runtime bridge helpers below operate on the desktop host process,
// bundled runtime assets, and local filesystem/process state.
fn runtime_platform() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

fn is_bundled_app() -> bool {
    !cfg!(debug_assertions)
}

fn bundled_binary_name(base: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

fn config_loader() -> ConfigLoader {
    match std::env::var("AGENT_DIVA_CONFIG_DIR") {
        Ok(path) if !path.trim().is_empty() => ConfigLoader::with_dir(expand_user_path(&path)),
        _ => ConfigLoader::new(),
    }
}

/// Saves the gateway port to a configuration file
pub(crate) fn save_gateway_port_config(port: u16) -> Result<(), String> {
    let loader = config_loader();
    let config_dir = loader.config_dir();
    std::fs::create_dir_all(config_dir)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;

    let port_file = config_dir.join("gateway.port");
    std::fs::write(&port_file, port.to_string())
        .map_err(|e| format!("Failed to write gateway port config: {}", e))?;

    info!("Saved gateway port {} to {}", port, port_file.display());
    Ok(())
}

#[cfg(test)]
mod gateway_status_tests {
    use super::{
        gateway_process_status_from_runtime, normalize_pet_voice_relative_path,
        normalize_tts_provider,
    };
    use crate::gateway_status::GatewayStatus;

    #[test]
    fn gateway_process_status_uses_embedded_runtime_state() {
        let status = GatewayStatus::new(3456);
        let process_status = gateway_process_status_from_runtime(&status);

        assert!(process_status.running);
        assert_eq!(process_status.pid, None);
        assert_eq!(process_status.executable_path, None);
        assert_eq!(
            process_status.details.as_deref(),
            Some("Gateway: Running (port: 3456)")
        );
    }

    #[test]
    fn pet_voice_relative_path_rejects_parent_escape() {
        let error = normalize_pet_voice_relative_path("voice_resource/../secret.mp3").unwrap_err();
        assert!(error.contains("voice_resource"));
    }

    #[test]
    fn pet_voice_relative_path_accepts_voice_resource_child() {
        let path = normalize_pet_voice_relative_path("voice_resource/custom/sample.mp3").unwrap();
        assert_eq!(path, "voice_resource/custom/sample.mp3");
    }

    #[test]
    fn normalize_tts_provider_accepts_minimax() {
        assert_eq!(normalize_tts_provider("minimax"), "minimax");
    }
}

/// Loads the gateway port from configuration file, defaults to 3000
#[allow(dead_code)]
fn load_gateway_port_config() -> u16 {
    let loader = config_loader();
    let port_file = loader.config_dir().join("gateway.port");

    match std::fs::read_to_string(&port_file) {
        Ok(content) => match content.trim().parse::<u16>() {
            Ok(port) => {
                debug!("Loaded gateway port {} from {}", port, port_file.display());
                port
            }
            Err(e) => {
                warn!("Invalid port in config file: {}. Using default 3000", e);
                3000
            }
        },
        Err(_) => {
            debug!("Gateway port config file not found. Using default 3000");
            3000
        }
    }
}

fn cli_runtime_from_loader(loader: &ConfigLoader) -> CliRuntime {
    CliRuntime::from_paths(
        Some(loader.config_path().to_path_buf()),
        Some(loader.config_dir().to_path_buf()),
        None,
    )
}

fn provider_access_for_test(
    config: &Config,
    provider: &str,
    api_base: Option<String>,
    api_key: Option<String>,
) -> ProviderAccess {
    let mut access = ProviderCatalogService::new()
        .get_provider_access(config, provider)
        .unwrap_or_else(|| ProviderAccess::from_config(None));
    if let Some(api_base) = api_base
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
    {
        access.api_base = Some(api_base);
    }
    if let Some(api_key) = api_key
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        access.api_key = Some(api_key);
    }
    access
}

fn provider_model_catalog_dto(catalog: SharedProviderModelCatalog) -> ProviderModelCatalog {
    ProviderModelCatalog {
        provider: catalog.provider,
        source: catalog.catalog_source,
        runtime_supported: catalog.runtime_supported,
        api_base: catalog.api_base,
        models: catalog.models.into_iter().map(|entry| entry.id).collect(),
        custom_models: catalog.custom_models,
        warnings: catalog.warnings,
        error: catalog.error,
    }
}

fn provider_models_from_catalog(catalog: SharedProviderModelCatalog) -> (Vec<String>, Vec<String>) {
    (
        catalog.models.into_iter().map(|entry| entry.id).collect(),
        catalog.custom_models,
    )
}

fn provider_spec_from_view(
    view: SharedProviderView,
    models: Vec<String>,
    custom_models: Vec<String>,
) -> ProviderSpec {
    ProviderSpec {
        name: view.id,
        display_name: view.display_name,
        api_type: view.api_type,
        source: serde_json::to_value(view.source)
            .ok()
            .and_then(|value| value.as_str().map(ToString::to_string))
            .unwrap_or_else(|| "builtin".to_string()),
        configured: view.configured,
        ready: view.ready,
        default_api_base: view.default_api_base.or(view.api_base).unwrap_or_default(),
        default_model: view.default_model,
        models,
        custom_models,
    }
}

fn expand_user_path(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

fn resolve_configured_path(path: &str, config_dir: &std::path::Path) -> PathBuf {
    let expanded = expand_user_path(path);
    if expanded.is_absolute() {
        expanded
    } else {
        config_dir.join(expanded)
    }
}

fn runtime_info_from_app(app: &AppHandle) -> RuntimeInfo {
    let resource_dir = app
        .path()
        .resolve(".", BaseDirectory::Resource)
        .ok()
        .map(|path| path.display().to_string());

    RuntimeInfo {
        platform: runtime_platform().to_string(),
        is_bundled: is_bundled_app(),
        resource_dir,
    }
}

fn ensure_bundled_runtime(app: &AppHandle) -> Result<RuntimeInfo, String> {
    let info = runtime_info_from_app(app);
    if !info.is_bundled {
        return Err("service management is only available in bundled app".to_string());
    }
    Ok(info)
}

fn candidate_cli_paths(app: &AppHandle) -> Vec<PathBuf> {
    let platform = runtime_platform();
    let binary_name = bundled_binary_name("agent-diva");
    let mut candidates = Vec::new();

    if let Ok(resource_path) = app.path().resolve(
        format!("bin/{platform}/{binary_name}"),
        BaseDirectory::Resource,
    ) {
        candidates.push(resource_path);
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            candidates.push(exe_dir.join(&binary_name));
            candidates.push(
                exe_dir
                    .join("resources")
                    .join("bin")
                    .join(platform)
                    .join(&binary_name),
            );
        }
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .map(PathBuf::from)
        .unwrap_or(manifest_dir);
    candidates.push(
        workspace_root
            .join("target")
            .join("release")
            .join(&binary_name),
    );
    candidates.push(
        workspace_root
            .join("target")
            .join("debug")
            .join(&binary_name),
    );
    if let Ok(path) = which::which(&binary_name) {
        candidates.push(path);
    }

    candidates
}

fn resolve_cli_binary(app: &AppHandle) -> Result<PathBuf, String> {
    candidate_cli_paths(app)
        .into_iter()
        .find(|path| path.exists())
        .ok_or_else(|| {
            format!(
                "unable to locate bundled agent-diva binary for platform {}",
                runtime_platform()
            )
        })
}

async fn run_service_cli(app: &AppHandle, args: &[&str]) -> Result<String, String> {
    if runtime_platform() != "windows" {
        return Err("service management is currently implemented for Windows only".to_string());
    }
    if cfg!(debug_assertions) {
        return Err("service management is only available in bundled app".to_string());
    }

    let cli_binary = resolve_cli_binary(app)?;
    let mut command = TokioCommand::new(&cli_binary);
    configure_background_command(&mut command);
    let output = command
        .arg("service")
        .args(args)
        .output()
        .await
        .map_err(|e| format!("failed to execute {}: {}", cli_binary.display(), e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let message = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("service command failed with status {}", output.status)
        };
        Err(message)
    }
}

async fn run_command_capture<I, S>(program: &str, args: I) -> Result<std::process::Output, String>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut command = TokioCommand::new(program);
    configure_background_command(&mut command);
    command
        .args(args.into_iter().map(Into::into))
        .output()
        .await
        .map_err(|e| format!("failed to execute {program}: {e}"))
}

fn command_output_message(program: &str, output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        format!("{program} exited with status {}", output.status)
    }
}

fn resolve_resource_path(app: &AppHandle, relative: &str) -> Option<PathBuf> {
    app.path().resolve(relative, BaseDirectory::Resource).ok()
}

fn resolve_linux_service_script(app: &AppHandle, file_name: &str) -> Result<PathBuf, String> {
    if let Some(path) = resolve_resource_path(app, &format!("systemd/{file_name}")) {
        if path.exists() {
            return Ok(path);
        }
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .map(PathBuf::from)
        .unwrap_or(manifest_dir);
    let fallback = workspace_root
        .join("contrib")
        .join("systemd")
        .join(file_name);
    if fallback.exists() {
        return Ok(fallback);
    }

    Err(format!(
        "unable to locate Linux service script: {file_name}"
    ))
}

fn resolve_macos_launchd_script(app: &AppHandle, file_name: &str) -> Result<PathBuf, String> {
    if let Some(path) = resolve_resource_path(app, &format!("launchd/{file_name}")) {
        if path.exists() {
            return Ok(path);
        }
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .map(PathBuf::from)
        .unwrap_or(manifest_dir);
    let fallback = workspace_root
        .join("contrib")
        .join("launchd")
        .join(file_name);
    if fallback.exists() {
        return Ok(fallback);
    }

    Err(format!(
        "unable to locate macOS launchd script: {file_name}"
    ))
}

async fn run_macos_launchd_bash(_app: &AppHandle, script: &PathBuf) -> Result<(), String> {
    let script_dir = script
        .parent()
        .ok_or_else(|| "script has no parent directory".to_string())?;
    let bundle_root = script_dir
        .parent()
        .ok_or_else(|| "launchd script has no bundle root".to_string())?;
    let output = TokioCommand::new("bash")
        .arg(script)
        .current_dir(bundle_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| {
            format!(
                "failed to execute launchd script {}: {}",
                script.display(),
                e
            )
        })?;

    if output.status.success() {
        Ok(())
    } else {
        Err(command_output_message("bash", &output))
    }
}

async fn run_macos_launchctl(action: &str) -> Result<(), String> {
    let output = TokioCommand::new("launchctl")
        .arg(action)
        .arg("com.agent-diva.gateway")
        .output()
        .await
        .map_err(|e| format!("failed to execute launchctl {action}: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(command_output_message("launchctl", &output))
    }
}

async fn linux_service_status() -> Result<ServiceStatusPayload, String> {
    let load_state = run_command_capture(
        "systemctl",
        ["show", "agent-diva", "--property=LoadState", "--value"],
    )
    .await?;
    if !load_state.status.success() {
        return Err(command_output_message("systemctl", &load_state));
    }

    let load_state_value = String::from_utf8_lossy(&load_state.stdout)
        .trim()
        .to_string();
    let active_state = run_command_capture(
        "systemctl",
        ["show", "agent-diva", "--property=ActiveState", "--value"],
    )
    .await?;
    if !active_state.status.success() {
        return Err(command_output_message("systemctl", &active_state));
    }

    let active_state_value = String::from_utf8_lossy(&active_state.stdout)
        .trim()
        .to_string();
    let installed = !matches!(load_state_value.as_str(), "" | "not-found");
    let running = active_state_value == "active";
    let state = if installed {
        format!("{load_state_value}/{active_state_value}")
    } else {
        "NotInstalled".to_string()
    };

    Ok(ServiceStatusPayload {
        installed,
        running,
        state: state.clone(),
        executable_path: Some("/usr/bin/agent-diva".to_string()),
        details: Some(if installed {
            format!("systemd load={load_state_value}, active={active_state_value}")
        } else {
            "systemd unit not installed".to_string()
        }),
    })
}

async fn run_linux_privileged_bash(script: &PathBuf) -> Result<(), String> {
    let pkexec = which::which("pkexec").map_err(|_| {
        "pkexec not found; install policykit or run the bundled script manually with sudo"
            .to_string()
    })?;
    let output = TokioCommand::new(pkexec)
        .arg("bash")
        .arg(script)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| {
            format!(
                "failed to execute privileged script {}: {}",
                script.display(),
                e
            )
        })?;

    if output.status.success() {
        Ok(())
    } else {
        Err(command_output_message("pkexec", &output))
    }
}

async fn run_linux_systemctl(action: &str) -> Result<(), String> {
    let pkexec = which::which("pkexec").map_err(|_| {
        "pkexec not found; install policykit or run systemctl manually with sudo".to_string()
    })?;
    let output = TokioCommand::new(pkexec)
        .arg("systemctl")
        .arg(action)
        .arg("agent-diva")
        .output()
        .await
        .map_err(|e| format!("failed to execute systemctl {action}: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(command_output_message("systemctl", &output))
    }
}

async fn macos_service_status() -> Result<ServiceStatusPayload, String> {
    let plist_path = dirs::home_dir()
        .map(|home| {
            home.join("Library")
                .join("LaunchAgents")
                .join("com.agent-diva.gateway.plist")
        })
        .ok_or_else(|| "failed to resolve home directory for launchd status".to_string())?;
    let installed = plist_path.exists();

    let launchctl_output = run_command_capture("launchctl", ["list"]).await?;
    if !launchctl_output.status.success() {
        return Err(command_output_message("launchctl", &launchctl_output));
    }

    let stdout = String::from_utf8_lossy(&launchctl_output.stdout);
    let running = stdout.contains("com.agent-diva.gateway");
    let state = if installed {
        if running {
            "Loaded".to_string()
        } else {
            "Installed".to_string()
        }
    } else {
        "NotInstalled".to_string()
    };

    Ok(ServiceStatusPayload {
        installed,
        running,
        state: state.clone(),
        executable_path: Some(plist_path.display().to_string()),
        details: Some(if installed {
            if running {
                "launchd Loaded".to_string()
            } else {
                "launchd Installed".to_string()
            }
        } else {
            "launchd plist not found".to_string()
        }),
    })
}

fn resolve_log_directory(config: &Config, loader: &ConfigLoader) -> PathBuf {
    resolve_configured_path(&config.logging.dir, loader.config_dir())
}

fn latest_log_file(log_dir: &std::path::Path) -> Result<Option<PathBuf>, String> {
    if !log_dir.exists() {
        return Ok(None);
    }

    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    let entries = std::fs::read_dir(log_dir)
        .map_err(|e| format!("failed to read log directory {}: {}", log_dir.display(), e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to inspect log directory entry: {e}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !(name.starts_with("gateway.log") || name.starts_with("gateway-")) {
            continue;
        }

        let modified = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        match &newest {
            Some((current, _)) if modified <= *current => {}
            _ => newest = Some((modified, path)),
        }
    }

    Ok(newest.map(|(_, path)| path))
}

pub fn gateway_process_status_from_runtime(status: &GatewayStatus) -> GatewayProcessStatus {
    GatewayProcessStatus {
        running: status.running,
        pid: None,
        executable_path: None,
        details: Some(status.format_status()),
    }
}

const EMBEDDED_GATEWAY_AUTOMATIC_MESSAGE: &str =
    "embedded mode: gateway starts automatically with app";
const EMBEDDED_GATEWAY_COMPAT_STOP_MESSAGE: &str =
    "embedded mode: stop_gateway is a compatibility no-op; quit the app or use tray Quit to stop the embedded gateway";
const EMBEDDED_GATEWAY_COMPAT_UNINSTALL_MESSAGE: &str =
    "embedded mode: uninstall_gateway is deprecated and only performs compatibility cleanup for stray legacy gateway processes";

#[tauri::command]
pub fn get_runtime_info(app: AppHandle) -> RuntimeInfo {
    runtime_info_from_app(&app)
}

#[tauri::command]
pub async fn get_service_status(app: AppHandle) -> Result<ServiceStatusPayload, String> {
    let info = ensure_bundled_runtime(&app)?;
    match info.platform.as_str() {
        "windows" => {
            let output = run_service_cli(&app, &["status", "--json"]).await?;
            let mut payload: ServiceStatusPayload = serde_json::from_str(&output)
                .map_err(|e| format!("failed to parse service status payload: {}", e))?;
            if payload.details.is_none() {
                payload.details = Some(payload.state.clone());
            }
            Ok(payload)
        }
        "linux" => linux_service_status().await,
        "macos" => macos_service_status().await,
        _ => Err("unsupported platform".to_string()),
    }
}

#[tauri::command]
pub async fn install_service(app: AppHandle) -> Result<(), String> {
    let info = ensure_bundled_runtime(&app)?;
    match info.platform.as_str() {
        "windows" => {
            let _ = run_service_cli(&app, &["install", "--auto-start"]).await?;
            Ok(())
        }
        "linux" => {
            let script = resolve_linux_service_script(&app, "install.sh")?;
            run_linux_privileged_bash(&script).await
        }
        "macos" => {
            let script = resolve_macos_launchd_script(&app, "install.sh")?;
            run_macos_launchd_bash(&app, &script).await
        }
        _ => Err("unsupported platform".to_string()),
    }
}

#[tauri::command]
pub async fn uninstall_service(app: AppHandle) -> Result<(), String> {
    let info = ensure_bundled_runtime(&app)?;
    match info.platform.as_str() {
        "windows" => {
            let _ = run_service_cli(&app, &["uninstall"]).await?;
            Ok(())
        }
        "linux" => {
            let script = resolve_linux_service_script(&app, "uninstall.sh")?;
            run_linux_privileged_bash(&script).await
        }
        "macos" => {
            let script = resolve_macos_launchd_script(&app, "uninstall.sh")?;
            run_macos_launchd_bash(&app, &script).await
        }
        _ => Err("unsupported platform".to_string()),
    }
}

#[tauri::command]
pub async fn start_service(app: AppHandle) -> Result<(), String> {
    let info = ensure_bundled_runtime(&app)?;
    match info.platform.as_str() {
        "windows" => {
            let _ = run_service_cli(&app, &["start"]).await?;
            Ok(())
        }
        "linux" => run_linux_systemctl("start").await,
        "macos" => run_macos_launchctl("start").await,
        _ => Err("unsupported platform".to_string()),
    }
}

#[tauri::command]
pub async fn stop_service(app: AppHandle) -> Result<(), String> {
    let info = ensure_bundled_runtime(&app)?;
    match info.platform.as_str() {
        "windows" => {
            let _ = run_service_cli(&app, &["stop"]).await?;
            Ok(())
        }
        "linux" => run_linux_systemctl("stop").await,
        "macos" => run_macos_launchctl("stop").await,
        _ => Err("unsupported platform".to_string()),
    }
}

#[tauri::command]
pub async fn get_gateway_process_status(
    state: State<'_, AsyncMutex<GatewayStatus>>,
) -> Result<GatewayProcessStatus, String> {
    let status = state.lock().await.clone();
    Ok(gateway_process_status_from_runtime(&status))
}

#[tauri::command]
pub async fn get_gateway_status(
    state: State<'_, AsyncMutex<GatewayStatus>>,
) -> Result<GatewayStatus, String> {
    Ok(state.lock().await.clone())
}

#[tauri::command]
#[allow(dead_code)]
pub fn get_gateway_port() -> u16 {
    load_gateway_port_config()
}

#[tauri::command]
#[deprecated(note = "Embedded mode starts the gateway automatically; keep for compatibility only.")]
pub async fn start_gateway(_app: AppHandle, _bin_path: Option<String>) -> Result<u16, String> {
    warn!("start_gateway called through deprecated compatibility layer");
    Err(EMBEDDED_GATEWAY_AUTOMATIC_MESSAGE.to_string())
}

#[tauri::command]
#[deprecated(
    note = "Embedded mode manages gateway shutdown with app lifecycle; keep for compatibility only."
)]
pub async fn stop_gateway() -> Result<(), String> {
    warn!("stop_gateway called through deprecated compatibility layer");
    info!("{}", EMBEDDED_GATEWAY_COMPAT_STOP_MESSAGE);
    Ok(())
}

#[tauri::command]
#[deprecated(
    note = "Embedded mode uses in-process lifecycle management; keep only as a compatibility cleanup wrapper."
)]
pub async fn uninstall_gateway(app: AppHandle) -> Result<(), String> {
    warn!("uninstall_gateway called through deprecated compatibility layer");
    info!("{}", EMBEDDED_GATEWAY_COMPAT_UNINSTALL_MESSAGE);

    crate::shutdown_embedded_gateway(&app).await;

    process_utils::cleanup_legacy_gateway_processes()
        .map(|terminated| {
            info!(
                "Compatibility uninstall cleanup finished: terminated {} legacy gateway process(es)",
                terminated
            );
        })
        .map_err(|error| format!("{EMBEDDED_GATEWAY_COMPAT_UNINSTALL_MESSAGE}: {error}"))
}

#[tauri::command]
pub fn load_config() -> Result<String, String> {
    let loader = config_loader();
    let config = loader
        .load()
        .map_err(|e| format!("failed to load config: {}", e))?;
    serde_json::to_string_pretty(&config).map_err(|e| format!("failed to serialize config: {}", e))
}

#[tauri::command]
pub async fn get_config(state: State<'_, AgentState>) -> Result<RuntimeConfigSnapshot, String> {
    let url = format!("{}/config", state.api_base_url());
    let response = state
        .client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch runtime config: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    response
        .json::<RuntimeConfigSnapshot>()
        .await
        .map_err(|e| format!("Invalid runtime config payload: {}", e))
}

#[tauri::command]
pub async fn get_config_status() -> Result<StatusReport, String> {
    let loader = config_loader();
    let runtime = cli_runtime_from_loader(&loader);
    collect_status_report(&runtime)
        .await
        .map_err(|e| format!("failed to collect config status: {}", e))
}

#[tauri::command]
pub fn save_config(raw: String) -> Result<(), String> {
    let loader = config_loader();
    let config: Config =
        serde_json::from_str(&raw).map_err(|e| format!("failed to parse config JSON: {}", e))?;
    loader
        .save(&config)
        .map_err(|e| format!("failed to save config: {}", e))
}

fn validate_wipe_config_root(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("config directory path is empty".to_string());
    }
    if path.components().count() < 2 {
        return Err("config directory path is too short to be safe".to_string());
    }
    if let Some(home) = dirs::home_dir() {
        if path == home.as_path() {
            return Err("refusing to delete user home as config directory".to_string());
        }
        if path.exists() {
            if let (Ok(h_canon), Ok(p_canon)) = (home.canonicalize(), path.canonicalize()) {
                if h_canon == p_canon {
                    return Err("refusing to delete user home as config directory".to_string());
                }
            }
        }
    }
    Ok(())
}

fn validate_external_workspace_delete(ws_canon: &Path) -> Result<(), String> {
    if let Some(home) = dirs::home_dir() {
        if let Ok(h) = home.canonicalize() {
            if ws_canon == h.as_path() {
                return Err("refusing to delete workspace: path is user home".to_string());
            }
        }
    }
    let lossy = ws_canon.to_string_lossy().to_lowercase();
    const BLOCKED: &[&str] = &[
        "\\program files\\",
        "\\program files (x86)\\",
        "\\windows\\",
        "\\programdata\\",
    ];
    for fragment in BLOCKED {
        if lossy.contains(fragment) {
            return Err(format!(
                "refusing to delete workspace under protected system location ({fragment})"
            ));
        }
    }
    Ok(())
}

fn wipe_local_disk_blocking(
    config_root: PathBuf,
    workspace: PathBuf,
) -> Result<WipeSummary, Vec<String>> {
    let mut removed_paths = Vec::new();
    let mut errors = Vec::new();

    let cr_exists = config_root.exists();
    let ws_still_exists = workspace.exists();

    let workspace_inside_config = if cr_exists && ws_still_exists {
        let cr_c = std::fs::canonicalize(&config_root).map_err(|e| {
            vec![format!(
                "failed to canonicalize config directory {}: {}",
                config_root.display(),
                e
            )]
        })?;
        let ws_c = std::fs::canonicalize(&workspace).map_err(|e| {
            vec![format!(
                "failed to canonicalize workspace {}: {}",
                workspace.display(),
                e
            )]
        })?;
        ws_c.starts_with(&cr_c)
    } else {
        false
    };

    if cr_exists {
        match std::fs::remove_dir_all(&config_root) {
            Ok(()) => removed_paths.push(config_root.display().to_string()),
            Err(e) => errors.push(format!(
                "failed to remove config directory {}: {}",
                config_root.display(),
                e
            )),
        }
    }

    if workspace.exists() && !workspace_inside_config {
        let ws_canon = std::fs::canonicalize(&workspace).map_err(|e| {
            vec![format!(
                "failed to canonicalize workspace {}: {}",
                workspace.display(),
                e
            )]
        })?;
        if let Err(msg) = validate_external_workspace_delete(&ws_canon) {
            errors.push(msg);
        } else {
            match std::fs::remove_dir_all(&workspace) {
                Ok(()) => {
                    let label = ws_canon.display().to_string();
                    if !removed_paths.iter().any(|p| p == &label) {
                        removed_paths.push(label);
                    }
                }
                Err(e) => errors.push(format!(
                    "failed to remove workspace {}: {}",
                    workspace.display(),
                    e
                )),
            }
        }
    }

    if errors.is_empty() {
        Ok(WipeSummary { removed_paths })
    } else {
        Err(errors)
    }
}

/// Stops the gateway, terminates stray gateway processes, then deletes the config directory
/// (and the workspace directory when it lies outside the config directory).
#[tauri::command]
pub async fn wipe_local_data(app: AppHandle) -> Result<WipeSummary, String> {
    crate::shutdown_embedded_gateway(&app).await;

    match process_utils::cleanup_legacy_gateway_processes() {
        Ok(terminated) if terminated > 0 => {
            info!(
                "wipe_local_data: terminated {} lingering legacy gateway process(es)",
                terminated
            );
        }
        Ok(_) => {}
        Err(error) => {
            warn!(
                "wipe_local_data: best-effort legacy gateway cleanup failed: {}",
                error
            );
        }
    }
    tokio::time::sleep(std::time::Duration::from_millis(450)).await;

    let loader = config_loader();
    validate_wipe_config_root(loader.config_dir())?;

    let runtime = cli_runtime_from_loader(&loader);
    let config = loader.load().unwrap_or_default();
    let config_root = loader.config_dir().to_path_buf();
    let workspace = runtime.effective_workspace(&config);

    tokio::task::spawn_blocking(move || wipe_local_disk_blocking(config_root, workspace))
        .await
        .map_err(|e| format!("wipe task join error: {}", e))?
        .map_err(|errs| errs.join("; "))
}

#[tauri::command]
pub fn tail_logs(lines: usize) -> Result<Vec<String>, String> {
    let loader = config_loader();
    let config = loader
        .load()
        .map_err(|e| format!("failed to load config for logs: {}", e))?;
    let log_dir = resolve_log_directory(&config, &loader);
    let Some(log_file) = latest_log_file(&log_dir)? else {
        return Ok(Vec::new());
    };

    let content = std::fs::read_to_string(&log_file)
        .map_err(|e| format!("failed to read log file {}: {}", log_file.display(), e))?;
    let mut all_lines: Vec<String> = content.lines().map(ToString::to_string).collect();
    let keep = lines.max(1);
    if all_lines.len() > keep {
        all_lines = all_lines.split_off(all_lines.len().saturating_sub(keep));
    }
    Ok(all_lines)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GuiPrefs {
    pub close_to_tray: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VrmModelInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub source: String,
    pub thumbnail: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetImportVrmModelPayload {
    pub base64_data: String,
    pub file_name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetVrmModelFileData {
    pub base64_data: String,
    pub content_type: String,
    pub file_name: String,
}

const PET_VRM_DIR_NAME: &str = "vrm";
const PET_VRM_MODELS_DIR_NAME: &str = "models";
const PET_VRM_CUSTOM_DIR_NAME: &str = "custom";
const DEFAULT_PET_VRM_MODEL_PATH: &str = "/vrm/models/Alice.vrm";

/// Scans bundled VRM models and custom models under ~/.agent-diva/vrm/models/custom.
#[tauri::command]
pub async fn pet_list_vrm_models(app_handle: AppHandle) -> Result<Vec<VrmModelInfo>, String> {
    let mut models: Vec<VrmModelInfo> = Vec::new();
    append_builtin_vrm_models(&app_handle, &mut models);

    let loader = config_loader();
    append_custom_vrm_models(loader.config_dir(), &mut models)?;

    if !models
        .iter()
        .any(|model| model.path == DEFAULT_PET_VRM_MODEL_PATH)
    {
        models.push(VrmModelInfo {
            id: "Alice".to_string(),
            name: "Alice".to_string(),
            path: DEFAULT_PET_VRM_MODEL_PATH.to_string(),
            source: "builtin".to_string(),
            thumbnail: None,
        });
    }

    models.sort_by(|a, b| a.source.cmp(&b.source).then_with(|| a.name.cmp(&b.name)));
    models.dedup_by(|a, b| a.path == b.path);
    Ok(models)
}

#[tauri::command]
pub fn pet_import_vrm_model(payload: PetImportVrmModelPayload) -> Result<VrmModelInfo, String> {
    let loader = config_loader();
    let mut config = loader
        .load()
        .map_err(|e| format!("failed to load config: {}", e))?;
    let custom_dir = pet_vrm_custom_models_dir(loader.config_dir());
    let sanitized_name = sanitize_pet_vrm_file_name(&payload.file_name)?;
    let target_path = custom_dir.join(&sanitized_name);

    std::fs::create_dir_all(&custom_dir).map_err(|error| {
        format!(
            "failed to create VRM import directory {}: {}",
            custom_dir.display(),
            error
        )
    })?;

    let decoded = BASE64_STANDARD
        .decode(payload.base64_data.as_bytes())
        .map_err(|error| format!("failed to decode imported VRM file: {}", error))?;
    std::fs::write(&target_path, decoded).map_err(|error| {
        format!(
            "failed to write imported VRM file {}: {}",
            target_path.display(),
            error
        )
    })?;

    let relative_path = make_pet_vrm_relative_path(loader.config_dir(), &target_path)?;
    config.pet.vrm_model = relative_path.clone();
    loader
        .save(&config)
        .map_err(|e| format!("failed to save config: {}", e))?;

    let id = target_path
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or("custom")
        .to_string();
    Ok(VrmModelInfo {
        id: id.clone(),
        name: id,
        path: relative_path,
        source: "custom".to_string(),
        thumbnail: None,
    })
}

#[tauri::command]
pub fn pet_delete_vrm_model(relative_path: String) -> Result<(), String> {
    let normalized = normalize_pet_vrm_relative_path(&relative_path)?;
    if !normalized.starts_with("vrm/models/custom/") {
        return Err("only custom VRM models can be deleted".to_string());
    }

    let loader = config_loader();
    let mut config = loader
        .load()
        .map_err(|e| format!("failed to load config: {}", e))?;
    let target_path = resolve_pet_vrm_model_file(loader.config_dir(), &normalized)?;
    std::fs::remove_file(&target_path).map_err(|error| {
        format!(
            "failed to delete VRM model {}: {}",
            target_path.display(),
            error
        )
    })?;

    if config.pet.vrm_model == normalized {
        config.pet.vrm_model = DEFAULT_PET_VRM_MODEL_PATH.to_string();
        loader
            .save(&config)
            .map_err(|e| format!("failed to save config: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub fn pet_read_vrm_model(relative_path: String) -> Result<PetVrmModelFileData, String> {
    let loader = config_loader();
    let normalized = normalize_pet_vrm_relative_path(&relative_path)?;
    if !normalized.starts_with("vrm/models/custom/") {
        return Err("only custom VRM models can be read from the config directory".to_string());
    }

    let file_path = resolve_pet_vrm_model_file(loader.config_dir(), &normalized)?;
    let bytes = std::fs::read(&file_path).map_err(|error| {
        format!(
            "failed to read VRM model {}: {}",
            file_path.display(),
            error
        )
    })?;
    let file_name = file_path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("model.vrm")
        .to_string();

    Ok(PetVrmModelFileData {
        base64_data: BASE64_STANDARD.encode(bytes),
        content_type: "model/gltf-binary".to_string(),
        file_name,
    })
}

fn append_builtin_vrm_models(app_handle: &AppHandle, models: &mut Vec<VrmModelInfo>) {
    let models_dir = match app_handle
        .path()
        .resolve("vrm/models", BaseDirectory::Resource)
    {
        Ok(dir) => dir,
        Err(_) => return,
    };

    if !models_dir.exists() {
        return;
    }

    let entries = match std::fs::read_dir(&models_dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("vrm") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(stem)
            .to_string();
        models.push(VrmModelInfo {
            id: stem.to_string(),
            name: stem.to_string(),
            path: format!("/vrm/models/{file_name}"),
            source: "builtin".to_string(),
            thumbnail: None,
        });
    }
}

const PET_VOICE_DIR_NAME: &str = "voice_resource";
const PET_VOICE_CUSTOM_DIR_NAME: &str = "custom";

// --- Voice Assets types ---

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetVoiceOption {
    pub id: String,
    pub label: String,
    pub relative_path: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetResolvedVoiceConfig {
    pub enabled: bool,
    pub provider: String,
    pub api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub siliconflow_api_key: Option<String>,
    pub minimax_api_key: Option<String>,
    pub base_url: String,
    pub model: Option<String>,
    pub voice_id: Option<String>,
    pub reference_voice: Option<String>,
    pub reference_text: Option<String>,
    pub speed: f64,
    pub volume: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetLoadedVoiceAssets {
    pub active_voice: PetResolvedVoiceConfig,
    pub config_directory_path: String,
    pub voice_options: Vec<PetVoiceOption>,
    pub voice_directory_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetSaveVoiceSelectionPayload {
    pub enabled: bool,
    pub provider: String,
    pub openai_api_key: Option<String>,
    pub siliconflow_api_key: Option<String>,
    pub minimax_api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub voice_id: Option<String>,
    pub reference_voice: Option<String>,
    pub reference_text: Option<String>,
    pub speed: f64,
    pub volume: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetImportVoiceFilePayload {
    pub base64_data: String,
    pub file_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetDeleteVoiceFilePayload {
    pub relative_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetVoiceFileData {
    pub base64_data: String,
    pub content_type: String,
    pub file_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetMiniMaxSynthesizePayload {
    pub text: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub voice_id: Option<String>,
    pub speed: Option<f64>,
    pub volume: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetMiniMaxSynthesizeResponse {
    pub base64_data: String,
    pub content_type: String,
}

type MiniMaxSocket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

struct MiniMaxSynthesizeResult {
    audio_bytes: Vec<u8>,
    chunk_count: usize,
    trace_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetSiliconFlowSynthesizePayload {
    pub text: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub voice: Option<String>,
    pub speed: Option<f64>,
    pub gain: Option<f64>,
    pub references: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetSiliconFlowSynthesizeResponse {
    pub base64_data: String,
    pub content_type: String,
}

#[tauri::command]
pub fn pet_load_voice_assets() -> Result<PetLoadedVoiceAssets, String> {
    let loader = config_loader();
    let config = loader
        .load()
        .map_err(|e| format!("failed to load config: {}", e))?;
    build_pet_voice_assets(loader.config_dir(), &config)
}

#[tauri::command]
pub fn pet_save_voice_selection(
    payload: PetSaveVoiceSelectionPayload,
) -> Result<PetLoadedVoiceAssets, String> {
    let loader = config_loader();
    let mut config = loader
        .load()
        .map_err(|e| format!("failed to load config: {}", e))?;

    config.pet.tts_enabled = payload.enabled;
    config.pet.tts_provider = normalize_tts_provider(&payload.provider);
    config.pet.tts_api_key = None;
    config.pet.tts_openai_api_key = payload
        .openai_api_key
        .filter(|value| !value.trim().is_empty());
    config.pet.tts_siliconflow_api_key = payload
        .siliconflow_api_key
        .filter(|value| !value.trim().is_empty());
    config.pet.tts_minimax_api_key = payload
        .minimax_api_key
        .filter(|value| !value.trim().is_empty());
    config.pet.tts_base_url = payload.base_url.unwrap_or_default();
    config.pet.tts_model = payload.model.filter(|value| !value.trim().is_empty());
    config.pet.tts_voice_id = payload.voice_id.filter(|value| !value.trim().is_empty());
    config.pet.tts_reference_voice = payload
        .reference_voice
        .as_deref()
        .map(normalize_pet_voice_relative_path)
        .transpose()?;
    config.pet.tts_reference_text = payload
        .reference_text
        .filter(|value| !value.trim().is_empty());
    config.pet.tts_speed = sanitized_pet_tts_speed(payload.speed);
    config.pet.tts_volume = sanitized_pet_tts_volume(payload.volume);

    loader
        .save(&config)
        .map_err(|e| format!("failed to save config: {}", e))?;
    build_pet_voice_assets(loader.config_dir(), &config)
}

#[tauri::command]
pub fn pet_import_voice_file(
    payload: PetImportVoiceFilePayload,
) -> Result<PetLoadedVoiceAssets, String> {
    let loader = config_loader();
    let mut config = loader
        .load()
        .map_err(|e| format!("failed to load config: {}", e))?;
    let config_dir = loader.config_dir();
    let voice_custom_dir = config_dir
        .join(PET_VOICE_DIR_NAME)
        .join(PET_VOICE_CUSTOM_DIR_NAME);
    let sanitized_name = sanitize_pet_voice_file_name(&payload.file_name);
    let target_path = voice_custom_dir.join(sanitized_name);

    std::fs::create_dir_all(&voice_custom_dir).map_err(|error| {
        format!(
            "failed to create voice import directory {}: {}",
            voice_custom_dir.display(),
            error
        )
    })?;

    let decoded_bytes = BASE64_STANDARD
        .decode(payload.base64_data.as_bytes())
        .map_err(|error| format!("failed to decode imported voice file: {}", error))?;
    std::fs::write(&target_path, decoded_bytes).map_err(|error| {
        format!(
            "failed to write imported voice file {}: {}",
            target_path.display(),
            error
        )
    })?;

    let relative_path = make_pet_voice_relative_path(config_dir, &target_path)?;
    config.pet.tts_reference_voice = Some(relative_path);
    if config.pet.tts_provider.trim().is_empty() || config.pet.tts_provider == "browser" {
        config.pet.tts_provider = "siliconflow".to_string();
    }
    if config.pet.tts_speed <= 0.0 || !config.pet.tts_speed.is_finite() {
        config.pet.tts_speed = 1.0;
    }
    if !config.pet.tts_volume.is_finite() || !(0.0..=2.0).contains(&config.pet.tts_volume) {
        config.pet.tts_volume = 1.0;
    }

    loader
        .save(&config)
        .map_err(|e| format!("failed to save config: {}", e))?;
    build_pet_voice_assets(config_dir, &config)
}

#[tauri::command]
pub fn pet_delete_voice_file(
    payload: PetDeleteVoiceFilePayload,
) -> Result<PetLoadedVoiceAssets, String> {
    let normalized_relative_path = normalize_pet_voice_relative_path(&payload.relative_path)?;
    if !normalized_relative_path.starts_with("voice_resource/custom/") {
        return Err("only custom voice files can be deleted".to_string());
    }

    let loader = config_loader();
    let mut config = loader
        .load()
        .map_err(|e| format!("failed to load config: {}", e))?;
    let target_path = resolve_pet_voice_file(loader.config_dir(), &normalized_relative_path)?;

    std::fs::remove_file(&target_path).map_err(|error| {
        format!(
            "failed to delete voice file {}: {}",
            target_path.display(),
            error
        )
    })?;

    if config.pet.tts_reference_voice.as_deref() == Some(normalized_relative_path.as_str()) {
        config.pet.tts_reference_voice = None;
        config.pet.tts_reference_text = None;
    }

    loader
        .save(&config)
        .map_err(|e| format!("failed to save config: {}", e))?;
    build_pet_voice_assets(loader.config_dir(), &config)
}

#[tauri::command]
pub fn pet_read_voice_file(relative_path: String) -> Result<PetVoiceFileData, String> {
    let loader = config_loader();
    let normalized_relative_path = normalize_pet_voice_relative_path(&relative_path)?;
    let file_path = resolve_pet_voice_file(loader.config_dir(), &normalized_relative_path)?;
    let file_bytes = std::fs::read(&file_path).map_err(|error| {
        format!(
            "failed to read voice file {}: {}",
            file_path.display(),
            error
        )
    })?;
    let file_name = file_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("voice.mp3")
        .to_string();

    Ok(PetVoiceFileData {
        base64_data: BASE64_STANDARD.encode(file_bytes),
        content_type: pet_voice_content_type(&file_name).to_string(),
        file_name,
    })
}

#[tauri::command]
pub async fn pet_minimax_synthesize(
    payload: PetMiniMaxSynthesizePayload,
) -> Result<PetMiniMaxSynthesizeResponse, String> {
    let api_key = payload.api_key.trim();
    if api_key.is_empty() {
        return Err("MiniMax API key is required".to_string());
    }

    let base_url = payload
        .base_url
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("https://api.minimaxi.com");
    let model = payload
        .model
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("speech-2.8-hd");
    let voice_id = payload
        .voice_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("male-qn-qingse");
    let speed = payload
        .speed
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(1.0)
        .clamp(1.0, 2.0)
        .round() as i32;
    let volume = payload
        .volume
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(1.0)
        .clamp(1.0, 10.0)
        .round() as i32;
    let ws_url = minimax_websocket_url(base_url);

    info!(
        model = model,
        voice_id = voice_id,
        ws_url = %ws_url,
        key_prefix = %&api_key[..api_key.len().min(8)],
        key_len = api_key.len(),
        "calling MiniMax TTS websocket API"
    );

    let mut socket = minimax_establish_connection(base_url, api_key).await?;
    let result =
        minimax_synthesize_sync(&mut socket, model, voice_id, speed, volume, &payload.text).await;
    let finish_result = minimax_finish_socket(&mut socket).await;
    let result = result?;
    finish_result?;

    info!(
        model = model,
        voice_id = voice_id,
        chunk_count = result.chunk_count,
        audio_bytes = result.audio_bytes.len(),
        trace_id = result.trace_id.as_deref().unwrap_or("n/a"),
        "MiniMax TTS synthesis completed via websocket"
    );

    Ok(PetMiniMaxSynthesizeResponse {
        base64_data: BASE64_STANDARD.encode(result.audio_bytes),
        content_type: "audio/mpeg".to_string(),
    })
}

async fn minimax_establish_connection(
    base_url: &str,
    api_key: &str,
) -> Result<MiniMaxSocket, String> {
    let ws_url = minimax_websocket_url(base_url);
    let mut request = ws_url
        .clone()
        .into_client_request()
        .map_err(|error| format!("failed to build MiniMax websocket request: {}", error))?;
    request.headers_mut().insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))
            .map_err(|error| format!("failed to build MiniMax authorization header: {}", error))?,
    );

    let tls = TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .danger_accept_invalid_hostnames(true)
        .build()
        .map_err(|error| format!("failed to build MiniMax TLS connector: {}", error))?;
    let connector = Connector::NativeTls(tls);

    let (mut socket, _) = timeout(
        Duration::from_secs(10),
        connect_async_tls_with_config(request, None, false, Some(connector)),
    )
    .await
    .map_err(|_| {
        format!(
            "timed out while connecting to MiniMax websocket: {}",
            ws_url
        )
    })?
    .map_err(|error| {
        format!(
            "failed to connect to MiniMax websocket: {} (input base_url: {}, resolved ws_url: {})",
            error, base_url, ws_url
        )
    })?;

    minimax_expect_event(&mut socket, "connected_success").await?;
    Ok(socket)
}

async fn minimax_synthesize_sync(
    socket: &mut MiniMaxSocket,
    model: &str,
    voice_id: &str,
    speed: i32,
    volume: i32,
    text: &str,
) -> Result<MiniMaxSynthesizeResult, String> {
    minimax_send_json(
        socket,
        serde_json::json!({
            "event": "task_start",
            "model": model,
            "voice_setting": {
                "voice_id": voice_id,
                "speed": speed,
                "vol": volume,
                "pitch": 0,
                "english_normalization": false
            },
            "audio_setting": {
                "sample_rate": 32000,
                "bitrate": 128000,
                "format": "mp3",
                "channel": 1
            }
        }),
    )
    .await?;
    minimax_expect_event(socket, "task_started").await?;

    minimax_send_json(
        socket,
        serde_json::json!({
            "event": "task_continue",
            "text": text
        }),
    )
    .await?;

    let mut audio_bytes = Vec::new();
    let mut chunk_count = 0usize;
    let mut trace_id = None;

    loop {
        let message = minimax_read_json_message(socket).await?;
        if trace_id.is_none() {
            trace_id = message
                .get("trace_id")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned);
        }

        if let Some(audio_hex) = message
            .get("data")
            .and_then(|value| value.get("audio"))
            .and_then(serde_json::Value::as_str)
        {
            if !audio_hex.trim().is_empty() {
                let chunk = decode_hex_audio(audio_hex)?;
                chunk_count += 1;
                audio_bytes.extend_from_slice(&chunk);
            }
        }

        if let Some(event) = message.get("event").and_then(serde_json::Value::as_str) {
            if event.eq_ignore_ascii_case("error") || event.eq_ignore_ascii_case("task_failed") {
                let detail = message
                    .get("message")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("MiniMax websocket returned an error event");
                return Err(detail.to_string());
            }
        }

        if message
            .get("is_final")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
        {
            break;
        }
    }

    if audio_bytes.is_empty() {
        return Err("MiniMax TTS completed without audio data".to_string());
    }

    Ok(MiniMaxSynthesizeResult {
        audio_bytes,
        chunk_count,
        trace_id,
    })
}

async fn minimax_send_json(
    socket: &mut MiniMaxSocket,
    payload: serde_json::Value,
) -> Result<(), String> {
    socket
        .send(WsMessage::Text(payload.to_string()))
        .await
        .map_err(|error| format!("failed to send MiniMax websocket message: {}", error))
}

async fn minimax_expect_event(socket: &mut MiniMaxSocket, expected: &str) -> Result<(), String> {
    let message = minimax_read_json_message(socket).await?;
    let actual = message
        .get("event")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "MiniMax websocket response is missing event field".to_string())?;

    if actual != expected {
        let detail = message
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        return Err(format!(
            "unexpected MiniMax websocket event: expected {}, got {}{}",
            expected,
            actual,
            if detail.is_empty() {
                String::new()
            } else {
                format!(" ({detail})")
            }
        ));
    }

    Ok(())
}

async fn minimax_read_json_message(
    socket: &mut MiniMaxSocket,
) -> Result<serde_json::Value, String> {
    loop {
        let frame = timeout(Duration::from_secs(30), socket.next())
            .await
            .map_err(|_| "timed out while waiting for MiniMax websocket message".to_string())?
            .ok_or_else(|| "MiniMax websocket closed unexpectedly".to_string())?
            .map_err(|error| format!("MiniMax websocket stream error: {}", error))?;

        match frame {
            WsMessage::Text(text) => {
                let value = serde_json::from_str::<serde_json::Value>(&text)
                    .map_err(|error| format!("invalid MiniMax websocket payload: {}", error))?;
                return Ok(value);
            }
            WsMessage::Ping(payload) => {
                socket
                    .send(WsMessage::Pong(payload))
                    .await
                    .map_err(|error| format!("failed to respond to MiniMax ping: {}", error))?;
            }
            WsMessage::Close(frame) => {
                let detail = frame
                    .map(|value| value.reason.to_string())
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| "unknown close frame".to_string());
                return Err(format!("MiniMax websocket closed: {}", detail));
            }
            WsMessage::Binary(_) => {
                return Err("MiniMax websocket returned unexpected binary frame".to_string());
            }
            WsMessage::Pong(_) | WsMessage::Frame(_) => {}
        }
    }
}

async fn minimax_finish_socket(socket: &mut MiniMaxSocket) -> Result<(), String> {
    let _ = minimax_send_json(socket, serde_json::json!({ "event": "task_finish" })).await;
    socket
        .close(None)
        .await
        .map_err(|error| format!("failed to close MiniMax websocket: {}", error))
}

fn minimax_websocket_url(base_url: &str) -> String {
    const DEFAULT_HOST: &str = "api.minimaxi.com";
    const WS_PATH: &str = "/ws/v1/t2a_v2";

    let trimmed = base_url.trim();
    let candidate = if trimmed.is_empty() {
        format!("https://{DEFAULT_HOST}")
    } else if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{}", trimmed.trim_matches('/'))
    };

    let mut url = match reqwest::Url::parse(&candidate) {
        Ok(url) => url,
        Err(_) => {
            return format!("wss://{DEFAULT_HOST}{WS_PATH}");
        }
    };

    match url.scheme() {
        "http" | "ws" => {
            let _ = url.set_scheme("ws");
        }
        "https" | "wss" => {
            let _ = url.set_scheme("wss");
        }
        _ => {
            let _ = url.set_scheme("wss");
        }
    }

    if matches!(url.host_str(), Some("platform.minimaxi.com")) {
        let _ = url.set_host(Some(DEFAULT_HOST));
    }

    url.set_path(WS_PATH);
    url.set_query(None);
    url.set_fragment(None);
    url.to_string()
}

#[cfg(test)]
mod minimax_url_tests {
    use super::minimax_websocket_url;

    #[test]
    fn minimax_websocket_url_normalizes_api_base() {
        assert_eq!(
            minimax_websocket_url("https://api.minimaxi.com"),
            "wss://api.minimaxi.com/ws/v1/t2a_v2"
        );
        assert_eq!(
            minimax_websocket_url("https://api.minimaxi.com/v1"),
            "wss://api.minimaxi.com/ws/v1/t2a_v2"
        );
    }

    #[test]
    fn minimax_websocket_url_accepts_full_websocket_endpoint() {
        assert_eq!(
            minimax_websocket_url("wss://api.minimaxi.com/ws/v1/t2a_v2"),
            "wss://api.minimaxi.com/ws/v1/t2a_v2"
        );
    }

    #[test]
    fn minimax_websocket_url_rewrites_docs_host_and_paths() {
        assert_eq!(
            minimax_websocket_url("https://platform.minimaxi.com/docs/guides/speech-t2a-websocket"),
            "wss://api.minimaxi.com/ws/v1/t2a_v2"
        );
    }
}

#[tauri::command]
pub async fn pet_siliconflow_synthesize(
    payload: PetSiliconFlowSynthesizePayload,
) -> Result<PetSiliconFlowSynthesizeResponse, String> {
    let api_key = payload.api_key.trim();
    if api_key.is_empty() {
        return Err("SiliconFlow API key is required".to_string());
    }

    let base_url = payload
        .base_url
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("https://api.siliconflow.cn/v1");
    let endpoint = format!("{}/audio/speech", base_url.trim_end_matches('/'));
    let model = payload
        .model
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("fnlp/MOSS-TTSD-v0.5");
    let voice = payload
        .voice
        .as_deref()
        .map(str::trim)
        .unwrap_or("fnlp/MOSS-TTSD-v0.5:anna");
    let speed = payload
        .speed
        .filter(|v| v.is_finite() && *v > 0.0)
        .unwrap_or(1.0);
    let gain = payload.gain.filter(|v| v.is_finite()).unwrap_or(0.0);

    let mut body = serde_json::json!({
        "model": model,
        "input": payload.text,
        "voice": voice,
        "response_format": "mp3",
        "speed": speed,
        "gain": gain,
        "stream": false
    });

    // voice 和 references 互斥：有 references 时必须移除 voice 字段
    if let Some(refs) = &payload.references {
        if !refs.is_empty() {
            body.as_object_mut().and_then(|obj| obj.remove("voice"));
            body["references"] = serde_json::Value::Array(refs.clone());
        }
    }

    info!(
        model = model,
        voice = voice,
        endpoint = endpoint,
        has_references = payload
            .references
            .as_ref()
            .map(|r| !r.is_empty())
            .unwrap_or(false),
        "calling SiliconFlow TTS API"
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|error| format!("failed to build HTTP client: {}", error))?;

    let response = client
        .post(&endpoint)
        .bearer_auth(api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|error| format!("SiliconFlow TTS request failed: {}", error))?;

    let status = response.status();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("audio/mpeg")
        .to_string();
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("failed to read SiliconFlow response: {}", error))?;

    if !status.is_success() {
        let body_preview = String::from_utf8_lossy(&bytes);
        return Err(format!(
            "SiliconFlow TTS error: HTTP {} body={}",
            status, body_preview
        ));
    }

    if bytes.is_empty() {
        return Err("SiliconFlow TTS returned empty audio".to_string());
    }

    info!(
        model = model,
        audio_bytes = bytes.len(),
        content_type = content_type,
        "SiliconFlow TTS synthesis completed"
    );
    Ok(PetSiliconFlowSynthesizeResponse {
        base64_data: BASE64_STANDARD.encode(&bytes),
        content_type,
    })
}

fn build_pet_voice_assets(
    config_dir: &Path,
    config: &Config,
) -> Result<PetLoadedVoiceAssets, String> {
    let voice_dir = config_dir.join(PET_VOICE_DIR_NAME);
    let provider = normalize_tts_provider(&config.pet.tts_provider);
    Ok(PetLoadedVoiceAssets {
        active_voice: PetResolvedVoiceConfig {
            enabled: config.pet.tts_enabled,
            provider: provider.clone(),
            api_key: pet_tts_api_key_for_provider(&config.pet, &provider),
            openai_api_key: config.pet.tts_openai_api_key.clone(),
            siliconflow_api_key: config.pet.tts_siliconflow_api_key.clone(),
            minimax_api_key: config.pet.tts_minimax_api_key.clone(),
            base_url: config.pet.tts_base_url.clone(),
            model: config.pet.tts_model.clone(),
            voice_id: config.pet.tts_voice_id.clone(),
            reference_voice: config.pet.tts_reference_voice.clone(),
            reference_text: config.pet.tts_reference_text.clone(),
            speed: sanitized_pet_tts_speed(config.pet.tts_speed),
            volume: sanitized_pet_tts_volume(config.pet.tts_volume),
        },
        config_directory_path: config_dir.to_string_lossy().to_string(),
        voice_options: scan_pet_voice_files(config_dir)?,
        voice_directory_path: voice_dir.to_string_lossy().to_string(),
    })
}

fn scan_pet_voice_files(config_dir: &Path) -> Result<Vec<PetVoiceOption>, String> {
    let voice_root = config_dir.join(PET_VOICE_DIR_NAME);
    let mut options = Vec::new();
    if !voice_root.exists() {
        return Ok(options);
    }

    let mut stack = vec![voice_root];
    while let Some(directory) = stack.pop() {
        for entry in std::fs::read_dir(&directory).map_err(|error| {
            format!(
                "failed to scan voice directory {}: {}",
                directory.display(),
                error
            )
        })? {
            let entry = entry.map_err(|error| {
                format!(
                    "failed to read voice directory entry in {}: {}",
                    directory.display(),
                    error
                )
            })?;
            let path = entry.path();
            let file_type = entry.file_type().map_err(|error| {
                format!("failed to inspect voice file {}: {}", path.display(), error)
            })?;
            if file_type.is_dir() {
                stack.push(path);
                continue;
            }

            let file_name = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
            if !is_pet_voice_file(file_name) {
                continue;
            }

            let relative_path = make_pet_voice_relative_path(config_dir, &path)?;
            let source = if relative_path.starts_with("voice_resource/custom/") {
                "custom"
            } else {
                "builtin"
            };

            options.push(PetVoiceOption {
                id: relative_path.clone(),
                label: derive_pet_voice_label(file_name),
                relative_path,
                source: source.to_string(),
            });
        }
    }
    options.sort_by(|left, right| left.label.cmp(&right.label));
    Ok(options)
}

fn pet_vrm_custom_models_dir(config_dir: &Path) -> PathBuf {
    config_dir
        .join(PET_VRM_DIR_NAME)
        .join(PET_VRM_MODELS_DIR_NAME)
        .join(PET_VRM_CUSTOM_DIR_NAME)
}

fn append_custom_vrm_models(
    config_dir: &Path,
    models: &mut Vec<VrmModelInfo>,
) -> Result<(), String> {
    let custom_dir = pet_vrm_custom_models_dir(config_dir);
    if !custom_dir.exists() {
        return Ok(());
    }

    for entry in std::fs::read_dir(&custom_dir).map_err(|error| {
        format!(
            "failed to scan custom VRM directory {}: {}",
            custom_dir.display(),
            error
        )
    })? {
        let entry = entry.map_err(|error| {
            format!(
                "failed to read custom VRM directory entry in {}: {}",
                custom_dir.display(),
                error
            )
        })?;
        let path = entry.path();
        if path.extension().and_then(OsStr::to_str) != Some("vrm") {
            continue;
        }
        let stem = path.file_stem().and_then(OsStr::to_str).unwrap_or("custom");
        models.push(VrmModelInfo {
            id: stem.to_string(),
            name: stem.to_string(),
            path: make_pet_vrm_relative_path(config_dir, &path)?,
            source: "custom".to_string(),
            thumbnail: None,
        });
    }
    Ok(())
}

fn normalize_pet_vrm_relative_path(relative_path: &str) -> Result<String, String> {
    let normalized = relative_path.trim().replace('\\', "/");
    if normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.contains('\0')
        || normalized
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || !normalized.starts_with("vrm/models/")
        || !normalized.to_lowercase().ends_with(".vrm")
    {
        return Err("VRM model path must stay under vrm/models and end with .vrm".to_string());
    }
    Ok(normalized)
}

fn make_pet_vrm_relative_path(config_dir: &Path, absolute_path: &Path) -> Result<String, String> {
    let relative_path = absolute_path.strip_prefix(config_dir).map_err(|error| {
        format!(
            "failed to create relative path from {} to {}: {}",
            config_dir.display(),
            absolute_path.display(),
            error
        )
    })?;
    normalize_pet_vrm_relative_path(&relative_path.to_string_lossy())
}

fn resolve_pet_vrm_model_file(config_dir: &Path, relative_path: &str) -> Result<PathBuf, String> {
    let normalized = normalize_pet_vrm_relative_path(relative_path)?;
    let vrm_root = config_dir.join(PET_VRM_DIR_NAME);
    let candidate = config_dir.join(&normalized);
    let canonical_candidate = candidate
        .canonicalize()
        .map_err(|error| format!("failed to resolve VRM model path: {}", error))?;
    let canonical_vrm_root = vrm_root
        .canonicalize()
        .map_err(|error| format!("failed to resolve VRM resource directory: {}", error))?;

    if !canonical_candidate.starts_with(canonical_vrm_root) {
        return Err("VRM model file must be within vrm".to_string());
    }
    Ok(canonical_candidate)
}

fn sanitize_pet_vrm_file_name(file_name: &str) -> Result<String, String> {
    let trimmed = file_name.trim();
    let base_name = if trimmed.is_empty() {
        "custom-model.vrm"
    } else {
        trimmed
    };
    let sanitized: String = base_name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '-',
            _ => c,
        })
        .collect();
    if !sanitized.to_lowercase().ends_with(".vrm") {
        return Err("only .vrm model files can be imported".to_string());
    }
    Ok(sanitized)
}

fn resolve_pet_voice_file(config_dir: &Path, relative_path: &str) -> Result<PathBuf, String> {
    let normalized_relative_path = normalize_pet_voice_relative_path(relative_path)?;
    let voice_root = config_dir.join(PET_VOICE_DIR_NAME);
    let candidate = config_dir.join(&normalized_relative_path);
    let canonical_candidate = candidate
        .canonicalize()
        .map_err(|error| format!("failed to resolve voice file path: {}", error))?;
    let canonical_voice_root = voice_root
        .canonicalize()
        .map_err(|error| format!("failed to resolve voice resource directory: {}", error))?;

    if !canonical_candidate.starts_with(canonical_voice_root) {
        return Err("voice file must be within voice_resource".to_string());
    }
    Ok(canonical_candidate)
}

fn normalize_pet_voice_relative_path(relative_path: &str) -> Result<String, String> {
    let normalized = relative_path.trim().replace('\\', "/");
    if normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.contains('\0')
        || normalized
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || !normalized.starts_with("voice_resource/")
    {
        return Err("voice file path must stay under voice_resource".to_string());
    }
    Ok(normalized)
}

fn make_pet_voice_relative_path(config_dir: &Path, absolute_path: &Path) -> Result<String, String> {
    let relative_path = absolute_path.strip_prefix(config_dir).map_err(|error| {
        format!(
            "failed to create relative path from {} to {}: {}",
            config_dir.display(),
            absolute_path.display(),
            error
        )
    })?;
    normalize_pet_voice_relative_path(&relative_path.to_string_lossy())
}

fn sanitize_pet_voice_file_name(file_name: &str) -> String {
    let trimmed = file_name.trim();
    let base_name = if trimmed.is_empty() {
        "custom-voice.mp3"
    } else {
        trimmed
    };
    base_name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '-',
            _ => c,
        })
        .collect()
}

fn is_pet_voice_file(file_name: &str) -> bool {
    let lower = file_name.to_lowercase();
    lower.ends_with(".mp3")
        || lower.ends_with(".wav")
        || lower.ends_with(".ogg")
        || lower.ends_with(".m4a")
        || lower.ends_with(".webm")
}

fn pet_voice_content_type(file_name: &str) -> &'static str {
    let lower = file_name.to_lowercase();
    if lower.ends_with(".wav") {
        "audio/wav"
    } else if lower.ends_with(".ogg") {
        "audio/ogg"
    } else if lower.ends_with(".m4a") {
        "audio/m4a"
    } else if lower.ends_with(".webm") {
        "audio/webm"
    } else {
        "audio/mpeg"
    }
}

fn derive_pet_voice_label(file_name: &str) -> String {
    file_name
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(file_name)
        .replace(['_', '-'], " ")
        .trim()
        .to_string()
}

#[cfg(test)]
mod pet_vrm_model_tests {
    use super::{
        append_custom_vrm_models, make_pet_vrm_relative_path, normalize_pet_vrm_relative_path,
        sanitize_pet_vrm_file_name,
    };

    #[test]
    fn normalizes_custom_vrm_relative_path() {
        let path = normalize_pet_vrm_relative_path("vrm\\models\\custom\\Alice.vrm").unwrap();
        assert_eq!(path, "vrm/models/custom/Alice.vrm");
    }

    #[test]
    fn rejects_invalid_vrm_relative_paths() {
        assert!(normalize_pet_vrm_relative_path("vrm/models/custom/../secret.vrm").is_err());
        assert!(normalize_pet_vrm_relative_path("/vrm/models/custom/model.vrm").is_err());
        assert!(normalize_pet_vrm_relative_path("vrm/models/custom/model.txt").is_err());
    }

    #[test]
    fn sanitizes_imported_vrm_file_names() {
        assert_eq!(
            sanitize_pet_vrm_file_name("bad:name?.vrm").unwrap(),
            "bad-name-.vrm"
        );
        assert!(sanitize_pet_vrm_file_name("bad.glb").is_err());
    }

    #[test]
    fn scans_custom_vrm_models_under_config_dir() {
        let temp = tempfile::tempdir().unwrap();
        let model_dir = temp.path().join("vrm").join("models").join("custom");
        std::fs::create_dir_all(&model_dir).unwrap();
        let model_path = model_dir.join("Custom.vrm");
        std::fs::write(&model_path, b"vrm").unwrap();
        std::fs::write(model_dir.join("ignored.txt"), b"no").unwrap();

        let relative = make_pet_vrm_relative_path(temp.path(), &model_path).unwrap();
        assert_eq!(relative, "vrm/models/custom/Custom.vrm");

        let mut models = Vec::new();
        append_custom_vrm_models(temp.path(), &mut models).unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "Custom");
        assert_eq!(models[0].source, "custom");
        assert_eq!(models[0].path, "vrm/models/custom/Custom.vrm");
    }
}

fn decode_hex_audio(value: &str) -> Result<Vec<u8>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    if trimmed.len() % 2 != 0 {
        return Err("MiniMax audio chunk is not valid hex".to_string());
    }

    let mut bytes = Vec::with_capacity(trimmed.len() / 2);
    for index in (0..trimmed.len()).step_by(2) {
        let byte = u8::from_str_radix(&trimmed[index..index + 2], 16)
            .map_err(|error| format!("failed to decode MiniMax audio chunk: {}", error))?;
        bytes.push(byte);
    }
    Ok(bytes)
}

fn normalize_tts_provider(provider: &str) -> String {
    match provider.trim().to_lowercase().as_str() {
        "openai" => "openai".to_string(),
        "siliconflow" => "siliconflow".to_string(),
        "minimax" => "minimax".to_string(),
        _ => "browser".to_string(),
    }
}

fn pet_tts_api_key_for_provider(
    config: &agent_diva_core::config::schema::PetConfig,
    provider: &str,
) -> Option<String> {
    match provider {
        "openai" => config.tts_openai_api_key.clone(),
        "siliconflow" => config.tts_siliconflow_api_key.clone(),
        "minimax" => config.tts_minimax_api_key.clone(),
        _ => None,
    }
}

fn sanitized_pet_tts_speed(speed: f64) -> f64 {
    if speed.is_finite() && speed > 0.0 {
        speed
    } else {
        1.0
    }
}

fn sanitized_pet_tts_volume(volume: f64) -> f64 {
    if volume.is_finite() && (0.0..=2.0).contains(&volume) {
        volume
    } else {
        1.0
    }
}

// ============================================================
// Token Usage Statistics Commands
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageTotal {
    pub total_input: i64,
    pub total_output: i64,
    pub total_tokens: i64,
    pub total_cache_creation: i64,
    pub total_cache_read: i64,
    pub request_count: u64,
    pub total_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageSummary {
    pub group_key: String,
    pub total_input: i64,
    pub total_output: i64,
    pub total_tokens: i64,
    pub total_cache_creation: i64,
    pub total_cache_read: i64,
    pub request_count: u64,
    pub total_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenTimelinePoint {
    pub time_bucket: String,
    pub total_input: i64,
    pub total_output: i64,
    pub total_tokens: i64,
    pub request_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSessionUsage {
    pub session_id: String,
    pub total_input: i64,
    pub total_output: i64,
    pub total_tokens: i64,
    pub request_count: u64,
    pub total_cost: f64,
    pub primary_model: String,
    pub channel: Option<String>,
    pub last_activity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenModelDistribution {
    pub model: String,
    pub percentage: f64,
    pub total_tokens: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInMemoryStats {
    pub total_tokens: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub request_count: u64,
    pub total_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiResponse<T> {
    status: String,
    data: Option<T>,
    message: Option<String>,
}

async fn fetch_token_stats<T: serde::de::DeserializeOwned>(
    state: &AgentState,
    endpoint: &str,
) -> Result<T, String> {
    let url = format!("{}{}", state.api_base_url(), endpoint);
    let response = state
        .client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch token stats: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let api_response: ApiResponse<T> = response
        .json()
        .await
        .map_err(|e| format!("Invalid response payload: {}", e))?;

    if api_response.status != "ok" {
        return Err(api_response
            .message
            .unwrap_or_else(|| "Unknown error".to_string()));
    }

    api_response
        .data
        .ok_or_else(|| "No data in response".to_string())
}

#[tauri::command]
pub async fn get_token_usage_total(
    state: State<'_, AgentState>,
    period: String,
) -> Result<TokenUsageTotal, String> {
    let endpoint = format!("/stats/tokens/total?period={}", period);
    fetch_token_stats(&state, &endpoint).await
}

#[tauri::command]
pub async fn get_token_usage_summary(
    state: State<'_, AgentState>,
    period: String,
    group_by: String,
) -> Result<Vec<TokenUsageSummary>, String> {
    let endpoint = format!(
        "/stats/tokens/summary?period={}&group_by={}",
        period, group_by
    );
    fetch_token_stats(&state, &endpoint).await
}

#[tauri::command]
pub async fn get_token_usage_timeline(
    state: State<'_, AgentState>,
    period: String,
    interval: Option<String>,
) -> Result<Vec<TokenTimelinePoint>, String> {
    let endpoint = match interval {
        Some(int) => format!("/stats/tokens/timeline?period={}&interval={}", period, int),
        None => format!("/stats/tokens/timeline?period={}", period),
    };
    fetch_token_stats(&state, &endpoint).await
}

#[tauri::command]
pub async fn get_token_usage_sessions(
    state: State<'_, AgentState>,
    period: String,
    limit: u64,
) -> Result<Vec<TokenSessionUsage>, String> {
    let endpoint = format!("/stats/tokens/sessions?period={}&limit={}", period, limit);
    fetch_token_stats(&state, &endpoint).await
}

#[tauri::command]
pub async fn get_token_usage_models(
    state: State<'_, AgentState>,
    period: String,
) -> Result<Vec<TokenModelDistribution>, String> {
    let endpoint = format!("/stats/tokens/models?period={}", period);
    fetch_token_stats(&state, &endpoint).await
}

#[tauri::command]
pub async fn get_token_usage_realtime(
    state: State<'_, AgentState>,
) -> Result<TokenInMemoryStats, String> {
    fetch_token_stats(&state, "/stats/tokens/realtime").await
}

// ============================================================
// Sandbox Commands
// ============================================================

#[tauri::command]
pub fn get_sandbox_config() -> Result<serde_json::Value, String> {
    let loader = config_loader();
    let config = loader
        .load()
        .map_err(|e| format!("failed to load config: {}", e))?;
    serde_json::to_value(&config.sandbox).map_err(|e| format!("failed to serialize sandbox config: {}", e))
}

#[tauri::command]
pub fn save_sandbox_config(config: serde_json::Value) -> Result<(), String> {
    let sandbox_config: agent_diva_core::config::SandboxConfig =
        serde_json::from_value(config)
            .map_err(|e| format!("Invalid sandbox config: {}", e))?;
    let loader = config_loader();
    let mut current = loader
        .load()
        .map_err(|e| format!("failed to load config: {}", e))?;
    current.sandbox = sandbox_config;
    loader
        .save(&current)
        .map_err(|e| format!("failed to save config: {}", e))
}

#[tauri::command]
pub fn get_gui_prefs(app: AppHandle) -> Result<GuiPrefs, String> {
    let store = app
        .store("settings.json")
        .map_err(|e| format!("Failed to get store: {}", e))?;

    let close_to_tray = store
        .get("closeToTray")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    Ok(GuiPrefs { close_to_tray })
}

#[tauri::command]
pub fn set_gui_prefs(app: AppHandle, prefs: GuiPrefs) -> Result<(), String> {
    let store = app
        .store("settings.json")
        .map_err(|e| format!("Failed to get store: {}", e))?;

    store.set("closeToTray", serde_json::json!(prefs.close_to_tray));
    store
        .save()
        .map_err(|e| format!("Failed to save store: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn open_desktop_pet(app: AppHandle) -> Result<(), String> {
    // Show existing window or create a new one
    if let Some(window) = app.get_webview_window("desktop-pet") {
        window
            .set_ignore_cursor_events(false)
            .map_err(|e| format!("Failed to reset ignore cursor events: {}", e))?;
        let _ = app.emit_to("desktop-pet", "desktop-pet-render-resume", true);
        window
            .show()
            .map_err(|e| format!("Failed to show desktop-pet window: {}", e))?;
        if cfg!(debug_assertions) {
            window.open_devtools();
        }
    } else {
        // Calculate bottom-right position
        let (x, y) = app
            .available_monitors()
            .map_err(|e| e.to_string())?
            .into_iter()
            .next()
            .map(|m| {
                let size = m.size();
                let scale = m.scale_factor();
                let logical_w = size.width as f64 / scale;
                let logical_h = size.height as f64 / scale;
                let lx = (logical_w - 400.0 - 40.0).max(0.0);
                let ly = (logical_h - 600.0 - 60.0).max(0.0);
                (lx, ly)
            })
            .unwrap_or((100.0, 100.0));

        let window = WebviewWindowBuilder::new(
            &app,
            "desktop-pet",
            WebviewUrl::App("desktop-pet.html".into()),
        )
        .inner_size(400.0, 600.0)
        .position(x, y)
        .visible(true)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .build()
        .map_err(|e| format!("Failed to create desktop-pet window: {}", e))?;
        window
            .set_ignore_cursor_events(false)
            .map_err(|e| format!("Failed to reset ignore cursor events: {}", e))?;
        window
            .show()
            .map_err(|e| format!("Failed to show desktop-pet window: {}", e))?;
        if cfg!(debug_assertions) {
            window.open_devtools();
        }
        let _ = app.emit_to("desktop-pet", "desktop-pet-render-resume", true);
    }
    app.emit_to("main", "desktop-pet-active", true)
        .map_err(|e| format!("Failed to emit event: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn close_desktop_pet(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("desktop-pet")
        .ok_or("desktop-pet window not found")?;
    let _ = app.emit_to("desktop-pet", "desktop-pet-render-pause", true);
    window
        .set_ignore_cursor_events(false)
        .map_err(|e| format!("Failed to reset ignore cursor events: {}", e))?;
    window
        .hide()
        .map_err(|e| format!("Failed to hide desktop-pet window: {}", e))?;
    app.emit_to("main", "desktop-pet-inactive", false)
        .map_err(|e| format!("Failed to emit event: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn set_desktop_pet_ignore_mouse(app: AppHandle, ignore: bool) -> Result<(), String> {
    let window = app
        .get_webview_window("desktop-pet")
        .ok_or("desktop-pet window not found")?;
    window
        .set_ignore_cursor_events(ignore)
        .map_err(|e| format!("Failed to set ignore cursor events: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn set_desktop_pet_always_on_top(app: AppHandle, always_on_top: bool) -> Result<(), String> {
    let window = app
        .get_webview_window("desktop-pet")
        .ok_or("desktop-pet window not found")?;
    window
        .set_always_on_top(always_on_top)
        .map_err(|e| format!("Failed to set always_on_top: {}", e))
}

#[tauri::command]
pub fn minimize_desktop_pet(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("desktop-pet")
        .ok_or("desktop-pet window not found")?;
    window
        .minimize()
        .map_err(|e| format!("Failed to minimize desktop-pet window: {}", e))
}
