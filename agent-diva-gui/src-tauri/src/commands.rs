use crate::app_state::AgentState;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State, Window};
use tracing::{error, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSpec {
    pub name: String,
    pub display_name: String,
    pub api_type: String,
    pub keywords: Vec<String>,
    pub env_key: String,
    pub litellm_prefix: String,
    pub skip_prefixes: Vec<String>,
    pub is_gateway: bool,
    pub is_local: bool,
    pub default_api_base: String,
    pub models: Vec<String>,
}

#[tauri::command]
pub async fn get_providers(state: State<'_, AgentState>) -> Result<Vec<ProviderSpec>, String> {
    let url = format!("{}/providers", state.api_base_url);

    // Try to fetch from API
    let response = state
        .client
        .get(&url)
        .timeout(std::time::Duration::from_secs(2)) // Short timeout for UI responsiveness
        .send()
        .await
        .map_err(|e| format!("Failed to fetch providers: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server error: {}", response.status()));
    }

    let specs: Vec<serde_json::Value> = response
        .json()
        .await
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    let providers = specs
        .into_iter()
        .map(|spec| ProviderSpec {
            name: spec["name"].as_str().unwrap_or_default().to_string(),
            display_name: spec["display_name"]
                .as_str()
                .unwrap_or(spec["name"].as_str().unwrap_or("Unknown"))
                .to_string(),
            api_type: spec["api_type"].as_str().unwrap_or("other").to_string(),
            keywords: spec["keywords"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .map(|k| k.as_str().unwrap_or_default().to_string())
                .collect(),
            env_key: spec["env_key"].as_str().unwrap_or_default().to_string(),
            litellm_prefix: spec["litellm_prefix"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            skip_prefixes: spec["skip_prefixes"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .map(|k| k.as_str().unwrap_or_default().to_string())
                .collect(),
            is_gateway: spec["is_gateway"].as_bool().unwrap_or(false),
            is_local: spec["is_local"].as_bool().unwrap_or(false),
            default_api_base: spec["default_api_base"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            models: spec["models"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .map(|m| m.as_str().unwrap_or_default().to_string())
                .collect(),
        })
        .collect();

    Ok(providers)
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

#[tauri::command]
pub async fn send_message(
    message: String,
    channel: Option<String>,
    chat_id: Option<String>,
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
                        let _ = window.emit("agent-response-delta", event.data);
                    }
                    "reasoning_delta" => {
                        let _ = window.emit("agent-reasoning-delta", event.data);
                    }
                    "tool_delta" => {
                        if let Ok(data) = serde_json::from_str::<ToolDeltaEvent>(&event.data) {
                            let _ = window.emit("agent-tool-delta", data.delta);
                        }
                    }
                    "final" => {
                        let _ = window.emit("agent-response-complete", event.data);
                    }
                    "tool_start" => {
                        if let Ok(data) = serde_json::from_str::<ToolStartEvent>(&event.data) {
                            let _ = window.emit("agent-tool-start", data);
                        } else {
                            // Fallback if parsing fails
                            let _ = window.emit(
                                "agent-tool-start",
                                serde_json::json!({
                                    "name": "unknown",
                                    "args_preview": event.data,
                                    "call_id": serde_json::Value::Null
                                }),
                            );
                        }
                    }
                    "tool_finish" => {
                        if let Ok(data) = serde_json::from_str::<ToolFinishEvent>(&event.data) {
                            let _ = window.emit("agent-tool-end", data);
                        } else {
                            let _ = window.emit(
                                "agent-tool-end",
                                serde_json::json!({
                                    "name": "unknown",
                                    "result": event.data,
                                    "is_error": false,
                                    "call_id": serde_json::Value::Null
                                }),
                            );
                        }
                    }
                    "error" => {
                        let _ = window.emit("agent-error", event.data);
                    }
                    _ => {}
                }
            }
            Err(e) => {
                error!("Stream error: {}", e);
                let _ = window.emit("agent-error", e.to_string());
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

    Ok(value
        .get("reset")
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

    Ok(value.get("sessions").cloned().unwrap_or(serde_json::Value::Array(vec![])))
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

    Ok(value.get("session").cloned().unwrap_or(serde_json::Value::Null))
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
    model: Option<String>,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    info!(
        "Updating config via API: model={:?}, base={:?}",
        model, api_base
    );
    state.reconfigure(api_base, api_key, model).await
}

#[tauri::command]
pub async fn get_tools_config(state: State<'_, AgentState>) -> Result<serde_json::Value, String> {
    state.get_tools_config().await
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

#[tauri::command]
pub async fn test_channel(
    name: String,
    config: serde_json::Value,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    let url = format!("{}/channels/test", state.api_base_url);

    let payload = serde_json::json!({
        "name": name,
        "enabled": true, // Test usually implies temporarily enabling or just checking config
        "config": config
    });

    let response = state
        .client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to test channel: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Test failed: {}", error_text));
    }

    Ok(())
}
