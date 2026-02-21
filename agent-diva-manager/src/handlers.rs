use axum::{
    extract::{State, Path},
    response::sse::{Event, Sse},
    Json,
};
use futures::stream::{Stream, StreamExt};
use std::convert::Infallible;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::UnboundedReceiverStream;
use agent_diva_agent::AgentEvent;
use agent_diva_core::bus::InboundMessage;
use agent_diva_core::config::schema::ChannelsConfig;
use agent_diva_providers::ProviderRegistry;

use crate::state::{AppState, ApiRequest, ManagerCommand, ConfigUpdate, ConfigResponse, ChannelUpdate};

#[derive(serde::Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub channel: Option<String>,
    pub chat_id: Option<String>,
}

pub async fn chat_handler(
    State(state): State<AppState>,
    Json(payload): Json<ChatRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    
    let msg = InboundMessage::new(
        payload.channel.unwrap_or("api".to_string()),
        "user",
        payload.chat_id.unwrap_or("default".to_string()),
        payload.message,
    );

    let req = ApiRequest {
        msg,
        event_tx,
    };

    if let Err(e) = state.api_tx.send(ManagerCommand::Chat(req)).await {
        tracing::error!("Failed to send API request to manager: {}", e);
    }

    let stream = UnboundedReceiverStream::new(event_rx).map(|event| {
        let evt = match event {
            AgentEvent::AssistantDelta { text } => {
                Event::default().event("delta").data(text)
            }
            AgentEvent::ReasoningDelta { text } => {
                Event::default().event("reasoning_delta").data(text)
            }
            AgentEvent::ToolCallDelta { name, args_delta } => {
                let data = serde_json::json!({
                    "name": name,
                    "delta": args_delta
                });
                Event::default().event("tool_delta").data(data.to_string())
            }
            AgentEvent::FinalResponse { content } => {
                Event::default().event("final").data(content)
            }
            AgentEvent::ToolCallStarted { name, args_preview, call_id } => {
                let data = serde_json::json!({
                    "name": name,
                    "args": args_preview,
                    "id": call_id
                });
                Event::default().event("tool_start").data(data.to_string())
            }
            AgentEvent::ToolCallFinished { name, result, is_error, call_id } => {
                let data = serde_json::json!({
                    "name": name,
                    "result": result,
                    "error": is_error,
                    "id": call_id
                });
                Event::default().event("tool_finish").data(data.to_string())
            }
            AgentEvent::Error { message } => {
                Event::default().event("error").data(message)
            }
            _ => Event::default().comment("keep-alive"),
        };
        Ok(evt)
    });

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

pub async fn get_config_handler(
    State(state): State<AppState>,
) -> Json<ConfigResponse> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state.api_tx.send(ManagerCommand::GetConfig(tx)).await {
        tracing::error!("Failed to send GetConfig request: {}", e);
        return Json(ConfigResponse {
            api_base: None,
            model: "unknown".to_string(),
            has_api_key: false,
        });
    }
    
    match rx.await {
        Ok(resp) => Json(resp),
        Err(e) => {
            tracing::error!("Failed to receive GetConfig response: {}", e);
            Json(ConfigResponse {
                api_base: None,
                model: "error".to_string(),
                has_api_key: false,
            })
        }
    }
}

pub async fn update_config_handler(
    State(state): State<AppState>,
    Json(payload): Json<ConfigUpdate>,
) -> Json<serde_json::Value> {
    tracing::info!("Received update config request: {:?}", payload);
    if let Err(e) = state.api_tx.send(ManagerCommand::UpdateConfig(payload)).await {
        tracing::error!("Failed to send UpdateConfig request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }
    
    Json(serde_json::json!({ "status": "ok" }))
}

pub async fn get_channels_handler(
    State(state): State<AppState>,
) -> Json<ChannelsConfig> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state.api_tx.send(ManagerCommand::GetChannels(tx)).await {
        tracing::error!("Failed to send GetChannels request: {}", e);
        return Json(ChannelsConfig::default());
    }
    match rx.await {
        Ok(config) => Json(config),
        Err(e) => {
            tracing::error!("Failed to receive GetChannels response: {}", e);
            Json(ChannelsConfig::default())
        }
    }
}

pub async fn update_channel_handler(
    State(state): State<AppState>,
    Json(payload): Json<ChannelUpdate>,
) -> Json<serde_json::Value> {
    tracing::info!("Received update channel request: {}", payload.name);
    if let Err(e) = state.api_tx.send(ManagerCommand::UpdateChannel(payload)).await {
        tracing::error!("Failed to send UpdateChannel request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }
    
    Json(serde_json::json!({ "status": "ok" }))
}

pub async fn test_channel_handler(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(config): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    tracing::info!("Received test channel request: {}", name);
    
    let payload = ChannelUpdate {
        name: name.clone(),
        enabled: Some(true),
        config,
    };

    let (tx, rx) = oneshot::channel();

    if let Err(e) = state.api_tx.send(ManagerCommand::TestChannel(payload, tx)).await {
        tracing::error!("Failed to send TestChannel request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }

    match rx.await {
        Ok(Ok(())) => Json(serde_json::json!({ "status": "ok" })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => {
            tracing::error!("Failed to receive TestChannel response: {}", e);
            Json(serde_json::json!({ "status": "error", "message": e.to_string() }))
        }
    }
}

pub async fn get_providers_handler() -> Json<Vec<agent_diva_providers::registry::ProviderSpec>> {
    let registry = ProviderRegistry::new();
    Json(registry.all().to_vec())
}
