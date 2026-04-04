use crate::app_state::AgentState;
use crate::process_utils;
use agent_diva_cli::cli_runtime::{collect_status_report, CliRuntime, StatusReport};
use agent_diva_core::config::{Config, ConfigLoader};
use agent_diva_neuron::{LlmNeuron, NeuronNode, NeuronRequest};
use agent_diva_providers::{
    CustomProviderUpsert, LiteLLMClient, Message, ProviderAccess, ProviderCatalogService,
    ProviderModelCatalogView as SharedProviderModelCatalog, ProviderView as SharedProviderView,
};
use eventsource_stream::Eventsource;
use futures::StreamExt;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Emitter, Manager, State, Window};
use tauri_plugin_store::StoreExt;
use tokio::process::Command as TokioCommand;
use tokio::sync::Mutex as AsyncMutex;
use tracing::{debug, error, info, warn};

struct GatewayProcess {
    child: tokio::process::Child,
    executable_path: String,
}

static GATEWAY_PROCESS: Lazy<AsyncMutex<Option<GatewayProcess>>> =
    Lazy::new(|| AsyncMutex::new(None));

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
    chat_id: Option<String>,
    stream_request_id: String,
    window: Window,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    info!("Sending message to API: {}", message);

    let client = &state.client;
    let url = format!("{}/chat", state.api_base_url);

    let response = client
        .post(&url)
        .json(&serde_json::json!({
            "message": message,
            "channel": channel,
            "chat_id": chat_id
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
    let url = format!("{}/chat/stop", state.api_base_url);
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
    let url = format!("{}/sessions/reset", state.api_base_url);
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
    let url = format!("{}/sessions/{}", state.api_base_url, id_encoded);

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
    let url = format!("{}/sessions", state.api_base_url);

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
    let url = format!("{}/sessions/{}", state.api_base_url, id_encoded);

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
    let url = format!("{}/cron/jobs", state.api_base_url);
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
        state.api_base_url,
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
    let url = format!("{}/cron/jobs", state.api_base_url);
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
        state.api_base_url,
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
        state.api_base_url,
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
        state.api_base_url,
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
        state.api_base_url,
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
        state.api_base_url,
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
) -> Result<(), String> {
    let client = state.client.clone();
    let url = format!(
        "{}/events?channel=api&chat_prefix=cron:",
        state.api_base_url
    );

    tauri::async_runtime::spawn(async move {
        loop {
            let response = match client.get(&url).send().await {
                Ok(resp) => resp,
                Err(e) => {
                    error!("Failed to connect background stream: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    continue;
                }
            };

            if !response.status().is_success() {
                error!("Background stream server error: {}", response.status());
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                continue;
            }

            let mut stream = response.bytes_stream().eventsource();
            while let Some(event) = stream.next().await {
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

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn check_health(state: State<'_, AgentState>) -> Result<bool, String> {
    let url = format!("{}/health", state.api_base_url);

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
    let url = format!("{}/mcps", state.api_base_url);
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
    let url = format!("{}/mcps/{}", state.api_base_url, urlencoding::encode(&name));
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
    let url = format!("{}/mcps/{}", state.api_base_url, urlencoding::encode(&name));
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
        state.api_base_url,
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
        state.api_base_url,
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
    let url = format!("{}/skills", state.api_base_url);
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
pub async fn delete_skill(name: String, state: State<'_, AgentState>) -> Result<(), String> {
    let name = urlencoding::encode(&name);
    let url = format!("{}/skills/{}", state.api_base_url, name);
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
    let url = format!("{}/channels", state.api_base_url);

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
    let url = format!("{}/channels", state.api_base_url);

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
fn save_gateway_port_config(port: u16) -> Result<(), String> {
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

fn resolved_cli_binary_for_launch(
    app: &AppHandle,
    bin_path: Option<String>,
) -> Result<PathBuf, String> {
    if let Some(bin_path) = bin_path {
        let candidate = PathBuf::from(bin_path);
        if candidate.exists() {
            return Ok(candidate);
        }
        return Err(format!(
            "gateway binary not found at {}",
            candidate.display()
        ));
    }

    resolve_cli_binary(app)
}

async fn refresh_gateway_process_status() -> GatewayProcessStatus {
    let mut guard = GATEWAY_PROCESS.lock().await;
    if let Some(process) = guard.as_mut() {
        match process.child.try_wait() {
            Ok(Some(status)) => {
                let detail = format!("gateway process exited with status {status}");
                *guard = None;
                GatewayProcessStatus {
                    running: false,
                    pid: None,
                    executable_path: None,
                    details: Some(detail),
                }
            }
            Ok(None) => GatewayProcessStatus {
                running: true,
                pid: process.child.id(),
                executable_path: Some(process.executable_path.clone()),
                details: Some("gateway process is running".to_string()),
            },
            Err(error) => GatewayProcessStatus {
                running: false,
                pid: None,
                executable_path: Some(process.executable_path.clone()),
                details: Some(format!("failed to inspect gateway process: {error}")),
            },
        }
    } else {
        // GATEWAY_PROCESS is empty, check port and system processes
        let port_occupied = process_utils::is_port_3000_occupied();
        let gateway_pids = process_utils::find_gateway_processes();

        if port_occupied || !gateway_pids.is_empty() {
            // Gateway is running but not managed by GUI
            let mut details = String::from("gateway process detected but not managed by GUI");
            if port_occupied {
                details.push_str(" (port 3000 occupied)");
            }
            if !gateway_pids.is_empty() {
                details.push_str(&format!(
                    " (found {} gateway process(es))",
                    gateway_pids.len()
                ));
            }

            GatewayProcessStatus {
                running: true,
                pid: gateway_pids.first().copied(),
                executable_path: None,
                details: Some(details),
            }
        } else {
            GatewayProcessStatus {
                running: false,
                pid: None,
                executable_path: None,
                details: Some("gateway process is not running".to_string()),
            }
        }
    }
}

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
pub async fn get_gateway_process_status() -> GatewayProcessStatus {
    refresh_gateway_process_status().await
}

#[tauri::command]
#[allow(dead_code)]
pub fn get_gateway_port() -> u16 {
    load_gateway_port_config()
}

#[tauri::command]
pub async fn start_gateway(app: AppHandle, bin_path: Option<String>) -> Result<u16, String> {
    let current_status = refresh_gateway_process_status().await;
    if current_status.running {
        return Err("gateway process is already running".to_string());
    }

    // Strategy 1: Try to use port 3000 directly
    let port = if process_utils::is_port_3000_occupied() {
        info!("Port 3000 is occupied, attempting mixed strategy...");

        // Strategy 2: Try to clean up and use port 3000
        match process_utils::force_cleanup_all_gateway_processes().await {
            Ok(count) => {
                if count > 0 {
                    info!("Forcefully terminated {} agent-diva process(es)", count);
                } else {
                    info!("No agent-diva processes found to terminate");
                }
            }
            Err(e) => {
                warn!("Error during forceful cleanup: {}", e);
            }
        }

        // Wait for port 3000 to become available
        match process_utils::wait_for_port_available(5, 3000).await {
            Ok(true) => {
                info!("Port 3000 is now available, using it");
                3000
            }
            Ok(false) => {
                // Strategy 3: Fall back to dynamic port
                info!("Port 3000 still occupied after cleanup, switching to dynamic port...");
                match process_utils::find_first_available_port(3001, 3010) {
                    Some(available_port) => {
                        info!("Found available port: {}", available_port);
                        available_port
                    }
                    None => {
                        return Err("All ports in range 3001-3010 are unavailable".to_string());
                    }
                }
            }
            Err(e) => {
                return Err(format!("Error while waiting for port 3000: {}", e));
            }
        }
    } else {
        // Port 3000 is available, use it
        info!("Port 3000 is available, using it");
        3000
    };

    let loader = config_loader();
    std::fs::create_dir_all(loader.config_dir()).map_err(|e| {
        format!(
            "failed to create config directory {}: {}",
            loader.config_dir().display(),
            e
        )
    })?;

    let executable = resolved_cli_binary_for_launch(&app, bin_path)?;
    let mut command = TokioCommand::new(&executable);
    configure_background_command(&mut command);
    command
        .arg("--config-dir")
        .arg(loader.config_dir())
        .arg("gateway")
        .arg("run")
        .current_dir(loader.config_dir())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = command.spawn().map_err(|e| {
        format!(
            "failed to spawn gateway process {}: {}",
            executable.display(),
            e
        )
    })?;

    let mut guard = GATEWAY_PROCESS.lock().await;
    *guard = Some(GatewayProcess {
        child,
        executable_path: executable.display().to_string(),
    });

    // Save the port to config for frontend to use
    if let Err(e) = save_gateway_port_config(port) {
        warn!("Failed to save gateway port config: {}", e);
    }

    info!("Gateway process started successfully on port {}", port);
    Ok(port)
}

#[tauri::command]
pub async fn stop_gateway() -> Result<(), String> {
    let mut guard = GATEWAY_PROCESS.lock().await;
    let Some(process) = guard.as_mut() else {
        return Ok(());
    };

    process
        .child
        .kill()
        .await
        .map_err(|e| format!("failed to stop gateway process: {e}"))?;
    *guard = None;
    Ok(())
}

#[tauri::command]
pub async fn uninstall_gateway() -> Result<(), String> {
    info!("Uninstalling gateway: terminating all gateway processes...");

    // First, stop the managed gateway process
    let _ = stop_gateway().await;

    // Then, find and terminate all gateway processes in the system
    let gateway_pids = process_utils::find_gateway_processes();
    if gateway_pids.is_empty() {
        info!("No gateway processes found to uninstall");
        return Ok(());
    }

    info!(
        "Found {} gateway process(es) to terminate",
        gateway_pids.len()
    );
    let mut errors = Vec::new();

    for pid in gateway_pids {
        match process_utils::terminate_process(pid) {
            Ok(()) => {
                info!("Terminated gateway process {}", pid);
            }
            Err(e) => {
                warn!("Failed to terminate gateway process {}: {}", pid, e);
                errors.push(format!("PID {}: {}", pid, e));
            }
        }
    }

    if !errors.is_empty() {
        return Err(format!(
            "Failed to terminate some processes: {}",
            errors.join(", ")
        ));
    }

    info!("Gateway uninstalled successfully");
    Ok(())
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
    let url = format!("{}/config", state.api_base_url);
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
pub async fn wipe_local_data() -> Result<WipeSummary, String> {
    stop_gateway().await?;

    let gateway_pids = process_utils::find_gateway_processes();
    if !gateway_pids.is_empty() {
        info!(
            "wipe_local_data: terminating {} lingering gateway process(es)",
            gateway_pids.len()
        );
    }
    for pid in gateway_pids {
        if let Err(e) = process_utils::terminate_process(pid) {
            warn!(
                "wipe_local_data: failed to terminate gateway pid {}: {}",
                pid, e
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
