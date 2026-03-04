use agent_diva_agent::AgentEvent;
use agent_diva_core::bus::InboundMessage;
use agent_diva_core::config::schema::ChannelsConfig;
use agent_diva_providers::ProviderRegistry;
use axum::{
    extract::{Path, Query, State},
    response::sse::{Event, Sse},
    Json,
};
use futures::stream::{Stream, StreamExt};
use std::convert::Infallible;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::state::{
    ApiRequest, AppState, ChannelUpdate, ConfigResponse, ConfigUpdate, ManagerCommand,
    StopChatRequest, ToolsConfigResponse, ToolsConfigUpdate,
};

#[derive(serde::Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub channel: Option<String>,
    pub chat_id: Option<String>,
}

#[derive(serde::Deserialize, Default)]
pub struct EventsQuery {
    pub channel: Option<String>,
    pub chat_id: Option<String>,
    pub chat_prefix: Option<String>,
}

pub async fn chat_handler(
    State(state): State<AppState>,
    Json(payload): Json<ChatRequest>,
) -> Sse<futures::stream::BoxStream<'static, Result<Event, Infallible>>> {
    let channel = payload.channel.unwrap_or("api".to_string());
    let chat_id = payload.chat_id.unwrap_or("default".to_string());

    if payload.message.trim() == "/stop" {
        let (stop_tx, stop_rx) = oneshot::channel();
        let stop_req = StopChatRequest {
            channel: Some(channel),
            chat_id: Some(chat_id),
        };
        let stop_send_result = state
            .api_tx
            .send(ManagerCommand::StopChat(stop_req, stop_tx))
            .await;

        let stop_message = match stop_send_result {
            Ok(_) => match stop_rx.await {
                Ok(Ok(_)) => "Generation stopped by user.".to_string(),
                Ok(Err(e)) => format!("Failed to stop generation: {}", e),
                Err(e) => format!("Failed to receive stop response: {}", e),
            },
            Err(e) => format!("Failed to send stop request: {}", e),
        };

        let stream = futures::stream::once(async move {
            Ok(Event::default().event("error").data(stop_message))
        })
        .boxed();
        return Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default());
    }

    let (event_tx, event_rx) = mpsc::unbounded_channel();

    let msg = InboundMessage::new(
        channel,
        "user",
        chat_id,
        payload.message,
    );

    let req = ApiRequest { msg, event_tx };

    if let Err(e) = state.api_tx.send(ManagerCommand::Chat(req)).await {
        tracing::error!("Failed to send API request to manager: {}", e);
    }

    let stream = UnboundedReceiverStream::new(event_rx)
        .map(|event| {
        let evt = match event {
            AgentEvent::AssistantDelta { text } => Event::default().event("delta").data(text),
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
            AgentEvent::FinalResponse { content } => Event::default().event("final").data(content),
            AgentEvent::ToolCallStarted {
                name,
                args_preview,
                call_id,
            } => {
                let data = serde_json::json!({
                    "name": name,
                    "args": args_preview,
                    "id": call_id
                });
                Event::default().event("tool_start").data(data.to_string())
            }
            AgentEvent::ToolCallFinished {
                name,
                result,
                is_error,
                call_id,
            } => {
                let data = serde_json::json!({
                    "name": name,
                    "result": result,
                    "error": is_error,
                    "id": call_id
                });
                Event::default().event("tool_finish").data(data.to_string())
            }
            AgentEvent::Error { message } => Event::default().event("error").data(message),
            _ => Event::default().comment("keep-alive"),
        };
        Ok(evt)
    })
        .boxed();

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

pub async fn stop_chat_handler(
    State(state): State<AppState>,
    Json(payload): Json<StopChatRequest>,
) -> Json<serde_json::Value> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::StopChat(payload, tx))
        .await
    {
        tracing::error!("Failed to send StopChat request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }

    match rx.await {
        Ok(Ok(stopped)) => Json(serde_json::json!({ "status": "ok", "stopped": stopped })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => {
            tracing::error!("Failed to receive StopChat response: {}", e);
            Json(serde_json::json!({ "status": "error", "message": e.to_string() }))
        }
    }
}

pub async fn reset_session_handler(
    State(state): State<AppState>,
    Json(payload): Json<crate::state::ResetSessionRequest>,
) -> Json<serde_json::Value> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::ResetSession(payload, tx))
        .await
    {
        tracing::error!("Failed to send ResetSession request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }

    match rx.await {
        Ok(Ok(reset)) => Json(serde_json::json!({ "status": "ok", "reset": reset })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => {
            tracing::error!("Failed to receive ResetSession response: {}", e);
            Json(serde_json::json!({ "status": "error", "message": e.to_string() }))
        }
    }
}

pub async fn get_sessions_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state.api_tx.send(ManagerCommand::GetSessions(tx)).await {
        tracing::error!("Failed to send GetSessions request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }

    match rx.await {
        Ok(Ok(sessions)) => Json(serde_json::json!({ "status": "ok", "sessions": sessions })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => {
            tracing::error!("Failed to receive GetSessions response: {}", e);
            Json(serde_json::json!({ "status": "error", "message": e.to_string() }))
        }
    }
}

pub async fn get_session_history_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    // If the path just gives an id (e.g. from frontend gui), then assume channel is implicit, normally the id comes as format `channel:chat_id` but frontend may just send `chat_id`. Wait, let the frontend send `channel:chat_id` via the path or query.
    // To support fetching any session_key, we will decode the path parameter if it's url encoded, or just use it as is.
    let session_key = if !id.contains(':') {
        format!("gui:{}", id) // fallback for backwards compatibility or assumptions
    } else {
        id
    };

    let (tx, rx) = oneshot::channel();
    if let Err(e) = state.api_tx.send(ManagerCommand::GetSessionHistory(session_key.clone(), tx)).await {
        tracing::error!("Failed to send GetSessionHistory request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }

    match rx.await {
        Ok(Ok(Some(session))) => Json(serde_json::json!({ "status": "ok", "session": session })),
        Ok(Ok(None)) => Json(serde_json::json!({ "status": "error", "message": "Session not found" })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => {
            tracing::error!("Failed to receive GetSessionHistory response: {}", e);
            Json(serde_json::json!({ "status": "error", "message": e.to_string() }))
        }
    }
}

pub async fn events_handler(
    State(state): State<AppState>,
    Query(query): Query<EventsQuery>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let event_rx = state.bus.subscribe_events();
    let channel_filter = query.channel;
    let chat_id_filter = query.chat_id;
    let chat_prefix_filter = query.chat_prefix;

    let stream = BroadcastStream::new(event_rx).filter_map(move |evt| {
        let channel_filter = channel_filter.clone();
        let chat_id_filter = chat_id_filter.clone();
        let chat_prefix_filter = chat_prefix_filter.clone();
        async move {
            let Ok(bus_event) = evt else {
                return None;
            };

            if let Some(ch) = &channel_filter {
                if bus_event.channel != *ch {
                    return None;
                }
            }
            if let Some(chat_id) = &chat_id_filter {
                if bus_event.chat_id != *chat_id {
                    return None;
                }
            }
            if let Some(prefix) = &chat_prefix_filter {
                if !bus_event.chat_id.starts_with(prefix) {
                    return None;
                }
            }

            match bus_event.event {
                AgentEvent::FinalResponse { content } => {
                    let data = serde_json::json!({
                        "channel": bus_event.channel,
                        "chat_id": bus_event.chat_id,
                        "content": content
                    });
                    Some(Ok(Event::default().event("final").data(data.to_string())))
                }
                AgentEvent::Error { message } => {
                    let data = serde_json::json!({
                        "channel": bus_event.channel,
                        "chat_id": bus_event.chat_id,
                        "message": message
                    });
                    Some(Ok(Event::default().event("error").data(data.to_string())))
                }
                _ => None,
            }
        }
    });

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

pub async fn get_config_handler(State(state): State<AppState>) -> Json<ConfigResponse> {
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
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::UpdateConfig(payload))
        .await
    {
        tracing::error!("Failed to send UpdateConfig request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }

    Json(serde_json::json!({ "status": "ok" }))
}

pub async fn get_channels_handler(State(state): State<AppState>) -> Json<ChannelsConfig> {
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

pub async fn get_tools_handler(State(state): State<AppState>) -> Json<ToolsConfigResponse> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state.api_tx.send(ManagerCommand::GetTools(tx)).await {
        tracing::error!("Failed to send GetTools request: {}", e);
        return Json(ToolsConfigResponse {
            web: agent_diva_core::config::schema::WebToolsConfig::default().into(),
        });
    }
    match rx.await {
        Ok(config) => Json(config),
        Err(e) => {
            tracing::error!("Failed to receive GetTools response: {}", e);
            Json(ToolsConfigResponse {
                web: agent_diva_core::config::schema::WebToolsConfig::default().into(),
            })
        }
    }
}

pub async fn update_tools_handler(
    State(state): State<AppState>,
    Json(payload): Json<ToolsConfigUpdate>,
) -> Json<serde_json::Value> {
    tracing::info!("Received update tools request");
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::UpdateTools(payload))
        .await
    {
        tracing::error!("Failed to send UpdateTools request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }
    Json(serde_json::json!({ "status": "ok" }))
}

pub async fn update_channel_handler(
    State(state): State<AppState>,
    Json(payload): Json<ChannelUpdate>,
) -> Json<serde_json::Value> {
    tracing::info!("Received update channel request: {}", payload.name);
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::UpdateChannel(payload))
        .await
    {
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

    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::TestChannel(payload, tx))
        .await
    {
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

pub async fn heartbeat_handler() -> &'static str {
    "ok"
}
