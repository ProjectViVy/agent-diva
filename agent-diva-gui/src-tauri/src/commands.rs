use tauri::{Emitter, State, Window};
use crate::app_state::AgentState;
use tracing::{info, error};
use serde::{Deserialize, Serialize};
use futures::StreamExt;
use eventsource_stream::Eventsource;

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
    let response = state.client.get(&url)
        .timeout(std::time::Duration::from_secs(2)) // Short timeout for UI responsiveness
        .send()
        .await
        .map_err(|e| format!("Failed to fetch providers: {}", e))?;
        
    if !response.status().is_success() {
         return Err(format!("Server error: {}", response.status()));
    }
    
    let specs: Vec<serde_json::Value> = response.json().await
        .map_err(|e| format!("Invalid JSON: {}", e))?;
        
    let providers = specs.into_iter().map(|spec| {
        ProviderSpec {
            name: spec["name"].as_str().unwrap_or_default().to_string(),
            display_name: spec["display_name"].as_str().unwrap_or(spec["name"].as_str().unwrap_or("Unknown")).to_string(),
            api_type: spec["api_type"].as_str().unwrap_or("other").to_string(),
            keywords: spec["keywords"].as_array().unwrap_or(&vec![]).iter()
                .map(|k| k.as_str().unwrap_or_default().to_string())
                .collect(),
            env_key: spec["env_key"].as_str().unwrap_or_default().to_string(),
            litellm_prefix: spec["litellm_prefix"].as_str().unwrap_or_default().to_string(),
            skip_prefixes: spec["skip_prefixes"].as_array().unwrap_or(&vec![]).iter()
                .map(|k| k.as_str().unwrap_or_default().to_string())
                .collect(),
            is_gateway: spec["is_gateway"].as_bool().unwrap_or(false),
            is_local: spec["is_local"].as_bool().unwrap_or(false),
            default_api_base: spec["default_api_base"].as_str().unwrap_or_default().to_string(),
            models: spec["models"].as_array().unwrap_or(&vec![]).iter()
                .map(|m| m.as_str().unwrap_or_default().to_string())
                .collect(),
        }
    }).collect();
    
    Ok(providers)
}

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[derive(Deserialize)]
struct ToolStartEvent {
    name: String,
    #[serde(alias = "args")]
    args_preview: String,
}

#[derive(Deserialize)]
struct ToolFinishEvent {
    name: String,
    result: String,
}

#[derive(Deserialize)]
struct ToolDeltaEvent {
    delta: String,
}

#[tauri::command]
pub async fn send_message(
    message: String, 
    window: Window,
    state: State<'_, AgentState>
) -> Result<(), String> {
    info!("Sending message to API: {}", message);
    
    let client = &state.client;
    let url = format!("{}/chat", state.api_base_url);

    let response = client.post(&url)
        .json(&serde_json::json!({ "message": message }))
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
                            let _ = window.emit("agent-tool-start", format!("Using tool {}: {}", data.name, data.args_preview));
                        } else {
                            // Fallback if parsing fails
                             let _ = window.emit("agent-tool-start", format!("Using tool..."));
                        }
                    }
                    "tool_finish" => {
                        if let Ok(data) = serde_json::from_str::<ToolFinishEvent>(&event.data) {
                             // Truncate logic
                            let result = data.result;
                            let display_result = if result.len() > 100 {
                                let mut end = 100;
                                while !result.is_char_boundary(end) {
                                    end -= 1;
                                }
                                format!("{}...", &result[..end])
                            } else {
                                result
                            };
                            let _ = window.emit("agent-tool-end", format!("Tool {} finished: {}", data.name, display_result));
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
pub async fn update_config(
    api_base: Option<String>,
    api_key: Option<String>,
    model: Option<String>,
    state: State<'_, AgentState>
) -> Result<(), String> {
    info!("Updating config via API: model={:?}, base={:?}", model, api_base);
    state.reconfigure(api_base, api_key, model).await
}

#[tauri::command]
pub async fn get_channels(state: State<'_, AgentState>) -> Result<serde_json::Value, String> {
    let url = format!("{}/channels", state.api_base_url);
    
    let response = state.client.get(&url)
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch channels: {}", e))?;
        
    if !response.status().is_success() {
         return Err(format!("Server error: {}", response.status()));
    }
    
    let channels: serde_json::Value = response.json().await
        .map_err(|e| format!("Invalid JSON: {}", e))?;
        
    Ok(channels)
}

#[tauri::command]
pub async fn update_channel(
    name: String,
    enabled: Option<bool>,
    config: serde_json::Value,
    state: State<'_, AgentState>
) -> Result<(), String> {
    let url = format!("{}/channels", state.api_base_url);
    
    let payload = serde_json::json!({
        "name": name,
        "enabled": enabled,
        "config": config
    });
    
    let response = state.client.post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to update channel: {}", e))?;
        
    if !response.status().is_success() {
         return Err(format!("Server error: {}", response.status()));
    }
    
    Ok(())
}
