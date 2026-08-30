//! Neuro-Link v1 loopback gateway.
//!
//! The gateway is the transport boundary for the Super Channel Fabric.  It
//! owns JSON-RPC framing, protocol/session validation, durable projection
//! replay, and command idempotency.  AgentLoop/Fabric ingress is represented
//! by [`NeuroLinkRuntime`]; the gateway deliberately does not reach through
//! the legacy `MessageBus` path.

use agent_diva_core::channel::{
    ChannelAddress, ChannelDirection, ChannelEnvelopeV1, ChannelOrigin, ChannelPayloadV1,
    ContentPart, Correlation, EnvelopeNotificationParams, EventAckParams, EventAckResultV1,
    ProtocolErrorCode, ProtocolHelloParams, ProtocolHelloResult, RpcErrorBody, RpcErrorV1, RpcId,
    RpcNotificationV1, RpcRequestV1, RpcResponseV1, ServiceBindingV1, ServiceListResultV1,
    SessionOpenParams, SessionOpenResultV1, SessionOpenedParams, StateResumeParams,
    StateSyncResultV1, TurnCancelParams, TurnCancelResultV1, TurnStartParams, TurnStartResultV1,
    CHANNEL_SCHEMA_VERSION_V1, JSON_RPC_VERSION, NEURO_LINK_PROTOCOL_V1,
};
use async_trait::async_trait;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures::StreamExt;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::oneshot;
use tokio::time::timeout;
use uuid::Uuid;

use crate::projection_journal::{stable_hash, ProjectionJournalError};
use crate::state::AppState;

/// Maximum JSON-RPC WebSocket frame/message accepted by the loopback gateway.
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;
/// Maximum number of typed content parts in one turn.
pub const MAX_PARTS: usize = 64;
/// Maximum size represented by one attachment reference.
pub const MAX_ATTACHMENT_BYTES: u64 = 50 * 1024 * 1024;
/// A client must identify itself promptly after opening the socket.
pub const HELLO_TIMEOUT: Duration = Duration::from_secs(5);
/// Stable service catalog revision for the first gateway release.
pub const CATALOG_REVISION: u64 = 1;

/// Typed ingress seam used by the gateway to reach the bounded AgentLoop /
/// Fabric runtime.  Implementations are expected to perform admission before
/// returning and to preserve the request and trace identity from the envelope.
#[async_trait]
pub trait NeuroLinkRuntime: Send + Sync {
    async fn start_turn(
        &self,
        envelope: ChannelEnvelopeV1,
    ) -> Result<TurnStartResultV1, NeuroLinkRuntimeError>;

    async fn cancel_turn(
        &self,
        _params: TurnCancelParams,
    ) -> Result<TurnCancelResultV1, NeuroLinkRuntimeError> {
        Err(NeuroLinkRuntimeError::Unavailable(
            "typed cancel runtime is not installed".to_string(),
        ))
    }
}

/// Failure categories mapped to stable Neuro-Link protocol errors.
#[derive(Debug, Error)]
pub enum NeuroLinkRuntimeError {
    #[error("runtime is unavailable: {0}")]
    Unavailable(String),
    #[error("runtime is busy: {0}")]
    Busy(String),
    #[error("runtime rejected the request: {0}")]
    Rejected(String),
    #[error("runtime failed: {0}")]
    Internal(String),
}

/// Build the loopback-only gateway route.
pub fn routes() -> Router<AppState> {
    Router::new().route("/api/neuro-link/v1/ws", get(websocket_handler))
}

/// Upgrade a loopback HTTP connection to the bounded Neuro-Link socket.
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.max_frame_size(MAX_FRAME_BYTES)
        .max_message_size(MAX_FRAME_BYTES)
        .on_upgrade(move |socket| serve_socket(socket, state))
}

#[derive(Default)]
struct ConnectionState {
    frontend_instance_id: Option<String>,
    capabilities: Vec<String>,
    session_key: Option<String>,
}

async fn serve_socket(mut socket: WebSocket, state: AppState) {
    // The first application frame is deliberately required to be hello.  A
    // short deadline prevents idle sockets from consuming a connection slot.
    let first = timeout(HELLO_TIMEOUT, socket.next()).await;
    let Some(first) = (match first {
        Ok(value) => value,
        Err(_) => {
            let _ = socket.send(Message::Close(None)).await;
            return;
        }
    }) else {
        return;
    };
    let Ok(first) = first else {
        return;
    };
    let Message::Text(text) = first else {
        let _ = socket.send(Message::Close(None)).await;
        return;
    };

    let mut connection = ConnectionState::default();
    if !dispatch_request(&mut socket, &state, &mut connection, &text).await
        || connection.frontend_instance_id.is_none()
    {
        let _ = socket.send(Message::Close(None)).await;
        return;
    }

    while let Some(frame) = socket.next().await {
        let Ok(frame) = frame else {
            break;
        };
        match frame {
            Message::Text(text) => {
                if text.len() > MAX_FRAME_BYTES {
                    let _ = send_error(
                        &mut socket,
                        RpcId::Null,
                        ProtocolErrorCode::FrameTooLarge,
                        format!("frame exceeds {} bytes", MAX_FRAME_BYTES),
                        None,
                    )
                    .await;
                    break;
                }
                if !dispatch_request(&mut socket, &state, &mut connection, &text).await {
                    break;
                }
            }
            Message::Binary(_) => {
                let _ = send_error(
                    &mut socket,
                    RpcId::Null,
                    ProtocolErrorCode::InvalidRequest,
                    "binary WebSocket frames are not supported".to_string(),
                    None,
                )
                .await;
            }
            Message::Ping(payload) => {
                if socket.send(Message::Pong(payload)).await.is_err() {
                    break;
                }
            }
            Message::Pong(_) => {}
            Message::Close(_) => break,
        }
    }
}

async fn dispatch_request(
    socket: &mut WebSocket,
    state: &AppState,
    connection: &mut ConnectionState,
    text: &str,
) -> bool {
    let request = match serde_json::from_str::<RpcRequestV1<Value>>(text) {
        Ok(request) => request,
        Err(error) => {
            return send_error(
                socket,
                RpcId::Null,
                ProtocolErrorCode::InvalidRequest,
                format!("invalid JSON-RPC request: {error}"),
                None,
            )
            .await
            .is_ok();
        }
    };

    if request.jsonrpc != JSON_RPC_VERSION {
        return send_error(
            socket,
            request.id,
            ProtocolErrorCode::InvalidRequest,
            format!("jsonrpc must be {JSON_RPC_VERSION}"),
            None,
        )
        .await
        .is_ok();
    }

    if connection.frontend_instance_id.is_none() && request.method != "protocol/hello" {
        return send_error(
            socket,
            request.id,
            ProtocolErrorCode::InvalidRequest,
            "protocol/hello must be the first request".to_string(),
            None,
        )
        .await
        .is_ok();
    }

    match request.method.as_str() {
        "protocol/hello" => handle_hello(socket, connection, request).await,
        "service/list" => handle_service_list(socket, request).await,
        "session/open" => handle_session_open(socket, state, connection, request).await,
        "turn/start" => handle_turn_start(socket, state, connection, request).await,
        "turn/cancel" => handle_turn_cancel(socket, state, connection, request).await,
        "event/ack" => handle_event_ack(socket, state, connection, request).await,
        "state/resume" => handle_state_resume(socket, state, connection, request).await,
        _ => send_error(
            socket,
            request.id,
            ProtocolErrorCode::MethodNotFound,
            format!("unknown Neuro-Link method: {}", request.method),
            None,
        )
        .await
        .is_ok(),
    }
}

async fn handle_hello(
    socket: &mut WebSocket,
    connection: &mut ConnectionState,
    request: RpcRequestV1<Value>,
) -> bool {
    if connection.frontend_instance_id.is_some() {
        return send_error(
            socket,
            request.id,
            ProtocolErrorCode::InvalidRequest,
            "protocol/hello may only be sent once".to_string(),
            None,
        )
        .await
        .is_ok();
    }
    let params = match parse_params::<ProtocolHelloParams>(request.params) {
        Ok(params) => params,
        Err(error) => return send_failure(socket, request.id, error).await,
    };
    if params.protocol != NEURO_LINK_PROTOCOL_V1 {
        return send_error(
            socket,
            request.id,
            ProtocolErrorCode::ProtocolVersionMismatch,
            format!("unsupported protocol: {}", params.protocol),
            None,
        )
        .await
        .is_ok();
    }
    if params.schema_version != CHANNEL_SCHEMA_VERSION_V1 {
        return send_error(
            socket,
            request.id,
            ProtocolErrorCode::ProtocolVersionMismatch,
            format!("unsupported schema version: {}", params.schema_version),
            None,
        )
        .await
        .is_ok();
    }
    if params.frontend_instance_id.trim().is_empty()
        || params
            .capabilities
            .iter()
            .any(|capability| capability.trim().is_empty())
    {
        return send_error(
            socket,
            request.id,
            ProtocolErrorCode::InvalidParams,
            "frontend_instance_id and capabilities must be non-empty".to_string(),
            None,
        )
        .await
        .is_ok();
    }
    let result = ProtocolHelloResult {
        protocol: NEURO_LINK_PROTOCOL_V1.to_string(),
        schema_version: CHANNEL_SCHEMA_VERSION_V1,
        capabilities: vec![
            "bounded_frames".to_string(),
            "durable_projection".to_string(),
            "idempotent_commands".to_string(),
            "state_sync".to_string(),
        ],
    };
    connection.frontend_instance_id = Some(params.frontend_instance_id);
    connection.capabilities = params.capabilities;
    send_response(socket, request.id, result).await.is_ok()
}

async fn handle_service_list(socket: &mut WebSocket, request: RpcRequestV1<Value>) -> bool {
    if !request.params.is_object()
        || request
            .params
            .as_object()
            .is_some_and(|object| !object.is_empty())
    {
        return send_error(
            socket,
            request.id,
            ProtocolErrorCode::InvalidParams,
            "service/list expects an empty params object".to_string(),
            None,
        )
        .await
        .is_ok();
    }
    send_response(socket, request.id, service_catalog())
        .await
        .is_ok()
}

async fn handle_session_open(
    socket: &mut WebSocket,
    state: &AppState,
    connection: &mut ConnectionState,
    request: RpcRequestV1<Value>,
) -> bool {
    let params = match parse_params::<SessionOpenParams>(request.params) {
        Ok(params) => params,
        Err(error) => return send_failure(socket, request.id, error).await,
    };
    if params.session_key.trim().is_empty() {
        return send_error(
            socket,
            request.id,
            ProtocolErrorCode::InvalidParams,
            "session_key must be non-empty".to_string(),
            None,
        )
        .await
        .is_ok();
    }
    let journal = match state.projection_journal().await {
        Ok(journal) => journal,
        Err(error) => {
            return send_error(
                socket,
                request.id,
                ProtocolErrorCode::ServiceUnavailable,
                format!("projection journal unavailable: {error}"),
                None,
            )
            .await
            .is_ok();
        }
    };
    let sync = match journal
        .replay(&params.session_key, params.durable_cursor.as_ref())
        .await
    {
        Ok(sync) => sync,
        Err(error) => return send_journal_error(socket, request.id, error).await,
    };
    let opened = SessionOpenedParams {
        session_key: params.session_key.clone(),
        cursor: sync.head.clone(),
        catalog_revision: CATALOG_REVISION,
        services: service_catalog().services,
    };
    let result = SessionOpenResultV1 {
        opened: opened.clone(),
        sync,
    };
    connection.session_key = Some(params.session_key);
    if send_response(socket, request.id, result).await.is_err() {
        return false;
    }
    send_notification(socket, "session/opened", opened)
        .await
        .is_ok()
}

async fn handle_turn_start(
    socket: &mut WebSocket,
    state: &AppState,
    connection: &mut ConnectionState,
    request: RpcRequestV1<Value>,
) -> bool {
    let raw_params = request.params.clone();
    let params = match parse_params::<TurnStartParams>(request.params) {
        Ok(params) => params,
        Err(error) => return send_failure(socket, request.id, error).await,
    };
    if !ensure_session(connection, &params.session_key) {
        return send_error(
            socket,
            request.id,
            ProtocolErrorCode::InvalidRequest,
            "session/open is required for this session".to_string(),
            None,
        )
        .await
        .is_ok();
    }
    if let Err(error) = validate_parts(&params.parts) {
        return send_failure(socket, request.id, error).await;
    }

    let journal = match state.projection_journal().await {
        Ok(journal) => journal,
        Err(error) => {
            return send_error(
                socket,
                request.id,
                ProtocolErrorCode::ServiceUnavailable,
                format!("projection journal unavailable: {error}"),
                None,
            )
            .await
            .is_ok();
        }
    };
    if let Some(client_message_id) = params.client_message_id.as_deref() {
        let request_hash = match stable_hash(&raw_params) {
            Ok(hash) => hash,
            Err(error) => {
                return send_error(
                    socket,
                    request.id,
                    ProtocolErrorCode::InternalError,
                    format!("failed to hash idempotency request: {error}"),
                    None,
                )
                .await
                .is_ok();
            }
        };
        match journal
            .lookup_idempotency(&params.session_key, client_message_id)
            .await
        {
            Ok(Some(record)) if record.request_hash != request_hash => {
                return send_error(
                    socket,
                    request.id,
                    ProtocolErrorCode::IdempotencyConflict,
                    "client_message_id was already used with different parameters".to_string(),
                    None,
                )
                .await
                .is_ok();
            }
            Ok(Some(record)) => {
                let result = match serde_json::from_value::<TurnStartResultV1>(record.result) {
                    Ok(result) => result,
                    Err(error) => {
                        return send_error(
                            socket,
                            request.id,
                            ProtocolErrorCode::InternalError,
                            format!("stored idempotency result is invalid: {error}"),
                            None,
                        )
                        .await
                        .is_ok();
                    }
                };
                return send_response(socket, request.id, result).await.is_ok();
            }
            Ok(None) => {}
            Err(error) => return send_journal_error(socket, request.id, error).await,
        }
    }

    let request_id = Uuid::new_v4().to_string();
    let trace_id = Uuid::new_v4().to_string();
    let envelope = make_turn_envelope(connection, &params, &request_id, &trace_id);
    let Some(runtime) = state.neuro_link_runtime.clone() else {
        return send_error(
            socket,
            request.id,
            ProtocolErrorCode::ServiceUnavailable,
            "typed Neuro-Link runtime is not installed".to_string(),
            None,
        )
        .await
        .is_ok();
    };
    let result = match runtime.start_turn(envelope.clone()).await {
        Ok(result) => result,
        Err(error) => return send_runtime_error(socket, request.id, error).await,
    };
    if result.session_key != params.session_key
        || result.request_id != request_id
        || result.trace_id != trace_id
    {
        return send_error(
            socket,
            request.id,
            ProtocolErrorCode::InternalError,
            "runtime returned mismatched request correlation".to_string(),
            None,
        )
        .await
        .is_ok();
    }

    let admission_envelope = make_admission_envelope(&result);
    let projection = match journal
        .append(
            &params.session_key,
            "turn/admission",
            admission_envelope,
            false,
        )
        .await
    {
        Ok(projection) => projection,
        Err(error) => return send_journal_error(socket, request.id, error).await,
    };
    if let Some(client_message_id) = params.client_message_id.as_deref() {
        let result_value = match serde_json::to_value(&result) {
            Ok(value) => value,
            Err(error) => {
                return send_error(
                    socket,
                    request.id,
                    ProtocolErrorCode::InternalError,
                    format!("failed to encode idempotency result: {error}"),
                    None,
                )
                .await
                .is_ok();
            }
        };
        if let Err(error) = journal
            .remember_idempotency(
                &params.session_key,
                client_message_id,
                &raw_params,
                &result_value,
            )
            .await
        {
            return send_journal_error(socket, request.id, error).await;
        }
    }
    if send_response(socket, request.id, result).await.is_err() {
        return false;
    }
    send_notification(
        socket,
        &projection.method,
        EnvelopeNotificationParams {
            envelope: projection.envelope,
        },
    )
    .await
    .is_ok()
}

async fn handle_turn_cancel(
    socket: &mut WebSocket,
    state: &AppState,
    connection: &ConnectionState,
    request: RpcRequestV1<Value>,
) -> bool {
    let params = match parse_params::<TurnCancelParams>(request.params) {
        Ok(params) => params,
        Err(error) => return send_failure(socket, request.id, error).await,
    };
    if !ensure_session(connection, &params.session_key) {
        return send_error(
            socket,
            request.id,
            ProtocolErrorCode::InvalidRequest,
            "session/open is required for this session".to_string(),
            None,
        )
        .await
        .is_ok();
    }
    let outcome = if let Some(runtime) = state.neuro_link_runtime.clone() {
        match runtime.cancel_turn(params.clone()).await {
            Ok(result) => result,
            Err(error) => return send_runtime_error(socket, request.id, error).await,
        }
    } else if let Some(control_tx) = state.runtime_control_tx.clone() {
        let (reply_tx, reply_rx) = oneshot::channel();
        if control_tx
            .send(
                agent_diva_agent::runtime_control::RuntimeControlCommand::StopSession {
                    session_key: params.session_key.clone(),
                    request_id: Some(params.request_id.clone()),
                    reply_tx,
                },
            )
            .is_err()
        {
            return send_error(
                socket,
                request.id,
                ProtocolErrorCode::ServiceUnavailable,
                "runtime control channel is closed".to_string(),
                None,
            )
            .await
            .is_ok();
        }
        let outcome = match timeout(Duration::from_secs(2), reply_rx).await {
            Ok(Ok(outcome)) => outcome,
            Ok(Err(_)) | Err(_) => {
                return send_error(
                    socket,
                    request.id,
                    ProtocolErrorCode::ServiceUnavailable,
                    "runtime control did not return a cancellation outcome".to_string(),
                    None,
                )
                .await
                .is_ok();
            }
        };
        TurnCancelResultV1 { outcome }
    } else {
        return send_error(
            socket,
            request.id,
            ProtocolErrorCode::ServiceUnavailable,
            "typed Neuro-Link runtime is not installed".to_string(),
            None,
        )
        .await
        .is_ok();
    };
    send_response(socket, request.id, outcome).await.is_ok()
}

async fn handle_event_ack(
    socket: &mut WebSocket,
    state: &AppState,
    connection: &ConnectionState,
    request: RpcRequestV1<Value>,
) -> bool {
    let params = match parse_params::<EventAckParams>(request.params) {
        Ok(params) => params,
        Err(error) => return send_failure(socket, request.id, error).await,
    };
    if !ensure_session(connection, &params.session_key) {
        return send_error(
            socket,
            request.id,
            ProtocolErrorCode::InvalidRequest,
            "session/open is required for this session".to_string(),
            None,
        )
        .await
        .is_ok();
    }
    let Some(frontend_instance_id) = connection.frontend_instance_id.as_deref() else {
        return false;
    };
    let journal = match state.projection_journal().await {
        Ok(journal) => journal,
        Err(error) => {
            return send_error(
                socket,
                request.id,
                ProtocolErrorCode::ServiceUnavailable,
                format!("projection journal unavailable: {error}"),
                None,
            )
            .await
            .is_ok();
        }
    };
    if let Err(error) = journal
        .acknowledge(frontend_instance_id, &params.session_key, &params.cursor)
        .await
    {
        return send_journal_error(socket, request.id, error).await;
    }
    send_response(
        socket,
        request.id,
        EventAckResultV1 {
            cursor: params.cursor,
        },
    )
    .await
    .is_ok()
}

async fn handle_state_resume(
    socket: &mut WebSocket,
    state: &AppState,
    connection: &ConnectionState,
    request: RpcRequestV1<Value>,
) -> bool {
    let params = match parse_params::<StateResumeParams>(request.params) {
        Ok(params) => params,
        Err(error) => return send_failure(socket, request.id, error).await,
    };
    if !ensure_session(connection, &params.session_key) {
        return send_error(
            socket,
            request.id,
            ProtocolErrorCode::InvalidRequest,
            "session/open is required for this session".to_string(),
            None,
        )
        .await
        .is_ok();
    }
    let journal = match state.projection_journal().await {
        Ok(journal) => journal,
        Err(error) => {
            return send_error(
                socket,
                request.id,
                ProtocolErrorCode::ServiceUnavailable,
                format!("projection journal unavailable: {error}"),
                None,
            )
            .await
            .is_ok();
        }
    };
    let sync: StateSyncResultV1 = match journal
        .replay(&params.session_key, Some(&params.cursor))
        .await
    {
        Ok(sync) => sync,
        Err(error) => return send_journal_error(socket, request.id, error).await,
    };
    send_response(socket, request.id, sync).await.is_ok()
}

fn parse_params<T: DeserializeOwned>(params: Value) -> Result<T, ResponseFailure> {
    serde_json::from_value(params).map_err(|error| ResponseFailure {
        code: ProtocolErrorCode::InvalidParams,
        message: format!("invalid params: {error}"),
        data: None,
    })
}

fn ensure_session(connection: &ConnectionState, expected: &str) -> bool {
    connection
        .session_key
        .as_deref()
        .is_some_and(|session| session == expected)
}

fn validate_parts(parts: &[ContentPart]) -> Result<(), ResponseFailure> {
    if parts.is_empty() {
        return Err(ResponseFailure {
            code: ProtocolErrorCode::InvalidParams,
            message: "turn/start requires at least one content part".to_string(),
            data: None,
        });
    }
    if parts.len() > MAX_PARTS {
        return Err(ResponseFailure {
            code: ProtocolErrorCode::InvalidParams,
            message: format!("turn/start accepts at most {MAX_PARTS} content parts"),
            data: None,
        });
    }
    for attachment in parts.iter().filter_map(attachment_ref) {
        if attachment.size_bytes > MAX_ATTACHMENT_BYTES {
            return Err(ResponseFailure {
                code: ProtocolErrorCode::AttachmentTooLarge,
                message: format!(
                    "attachment {} exceeds {} bytes",
                    attachment.uri, MAX_ATTACHMENT_BYTES
                ),
                data: None,
            });
        }
    }
    Ok(())
}

fn attachment_ref(part: &ContentPart) -> Option<&agent_diva_core::channel::AttachmentRef> {
    match part {
        ContentPart::Image { attachment }
        | ContentPart::Audio { attachment, .. }
        | ContentPart::Video { attachment }
        | ContentPart::File { attachment } => Some(attachment),
        _ => None,
    }
}

fn make_turn_envelope(
    connection: &ConnectionState,
    params: &TurnStartParams,
    request_id: &str,
    trace_id: &str,
) -> ChannelEnvelopeV1 {
    let mut address = ChannelAddress::new("neuro-link", params.session_key.clone());
    address.sender_id = connection.frontend_instance_id.clone();
    address.thread_id = params.thread_id.clone();
    let mut correlation = Correlation::new(params.session_key.clone());
    correlation.request_id = Some(request_id.to_string());
    correlation.trace_id = Some(trace_id.to_string());
    correlation.message_id = params.client_message_id.clone();
    ChannelEnvelopeV1::new(
        ChannelDirection::Ingress,
        address,
        correlation,
        ChannelOrigin::OwnerFrontend,
        ChannelPayloadV1::Message {
            parts: params.parts.clone(),
            subject: params.subject.clone(),
            locale: params.locale.clone(),
        },
    )
}

fn make_admission_envelope(result: &TurnStartResultV1) -> ChannelEnvelopeV1 {
    let mut correlation = Correlation::new(result.session_key.clone());
    correlation.request_id = Some(result.request_id.clone());
    correlation.trace_id = Some(result.trace_id.clone());
    ChannelEnvelopeV1::new(
        ChannelDirection::InternalProjection,
        ChannelAddress::new("neuro-link", result.session_key.clone()),
        correlation,
        ChannelOrigin::Runtime,
        ChannelPayloadV1::Control {
            operation: "turn/admission".to_string(),
            body: serde_json::to_value(&result.admission).unwrap_or(Value::Null),
        },
    )
}

/// Stable catalog exposed by `service/list` and `session/open`.
pub fn service_catalog() -> ServiceListResultV1 {
    ServiceListResultV1 {
        catalog_revision: CATALOG_REVISION,
        services: vec![
            binding(
                "neuro-link",
                [
                    "protocol/hello",
                    "service/list",
                    "session/open",
                    "turn/start",
                    "turn/cancel",
                    "event/ack",
                    "state/resume",
                ],
                "/api/neuro-link/v1",
                Some("neuro-link/v1"),
            ),
            binding(
                "sessions",
                [
                    "sessions/list",
                    "sessions/get",
                    "sessions/delete",
                    "sessions/title",
                ],
                "/api/sessions",
                None,
            ),
            binding("workspace", ["workspace/get"], "/api/workspace", None),
            binding(
                "persona",
                [
                    "persona/status",
                    "persona/initialize",
                    "persona/repair",
                    "persona/document",
                    "persona/history",
                ],
                "/api/persona",
                None,
            ),
            binding(
                "memory",
                ["memory/records", "memory/actmem", "memory/memrules"],
                "/api/memory",
                None,
            ),
            binding(
                "planning",
                ["planning/reports", "planning/executions", "planning/todos"],
                "/api/plan-reports",
                None,
            ),
            binding(
                "approval",
                ["approval/list", "approval/resolve"],
                "/api/approvals",
                None,
            ),
            binding(
                "ask-user",
                ["ask-user/list", "ask-user/answer"],
                "/api/ask-user",
                None,
            ),
            binding("files", ["files/upload"], "/api/files", None),
            binding(
                "skills",
                [
                    "skills/list",
                    "skills/get",
                    "skills/upload",
                    "skills/requests",
                ],
                "/api/skills",
                None,
            ),
            binding(
                "providers",
                ["providers/list", "providers/get", "providers/models"],
                "/api/providers",
                None,
            ),
            binding(
                "cron",
                ["cron/list", "cron/get", "cron/run", "cron/stop"],
                "/api/cron",
                None,
            ),
            binding(
                "autodream",
                [
                    "autodream/list",
                    "autodream/get",
                    "autodream/run",
                    "autodream/cancel",
                ],
                "/api/autodream",
                None,
            ),
            binding("audit", ["audit/log", "audit/events"], "/api/audit", None),
            binding("health", ["health/get", "heartbeat/get"], "/api", None),
        ],
    }
}

fn binding<const N: usize>(
    service_id: &str,
    methods: [&str; N],
    http_base: &str,
    required_frontend_capability: Option<&str>,
) -> ServiceBindingV1 {
    ServiceBindingV1 {
        service_id: service_id.to_string(),
        schema_version: CHANNEL_SCHEMA_VERSION_V1,
        methods: methods.into_iter().map(str::to_string).collect(),
        http_base: http_base.to_string(),
        required_frontend_capability: required_frontend_capability.map(str::to_string),
    }
}

#[derive(Debug)]
struct ResponseFailure {
    code: ProtocolErrorCode,
    message: String,
    data: Option<Value>,
}

async fn send_response<R: Serialize>(
    socket: &mut WebSocket,
    id: RpcId,
    result: R,
) -> Result<(), axum::Error> {
    let response = RpcResponseV1 {
        jsonrpc: JSON_RPC_VERSION.to_string(),
        id,
        result,
    };
    let text = serde_json::to_string(&response).map_err(axum::Error::new)?;
    socket.send(Message::Text(text)).await
}

async fn send_notification<P: Serialize>(
    socket: &mut WebSocket,
    method: &str,
    params: P,
) -> Result<(), axum::Error> {
    let notification = RpcNotificationV1 {
        jsonrpc: JSON_RPC_VERSION.to_string(),
        method: method.to_string(),
        params,
    };
    let text = serde_json::to_string(&notification).map_err(axum::Error::new)?;
    socket.send(Message::Text(text)).await
}

async fn send_error(
    socket: &mut WebSocket,
    id: RpcId,
    code: ProtocolErrorCode,
    message: String,
    data: Option<Value>,
) -> Result<(), axum::Error> {
    let response = RpcErrorV1 {
        jsonrpc: JSON_RPC_VERSION.to_string(),
        id,
        error: RpcErrorBody {
            code,
            message,
            data,
        },
    };
    let text = serde_json::to_string(&response).map_err(axum::Error::new)?;
    socket.send(Message::Text(text)).await
}

async fn send_failure(socket: &mut WebSocket, id: RpcId, failure: ResponseFailure) -> bool {
    send_error(socket, id, failure.code, failure.message, failure.data)
        .await
        .is_ok()
}

async fn send_journal_error(
    socket: &mut WebSocket,
    id: RpcId,
    error: ProjectionJournalError,
) -> bool {
    let (code, data) = match &error {
        ProjectionJournalError::CursorInvalid { .. }
        | ProjectionJournalError::CursorAhead { .. } => (ProtocolErrorCode::CursorInvalid, None),
        ProjectionJournalError::CursorOutOfRange { cursor, head } => (
            ProtocolErrorCode::CursorOutOfRange,
            Some(serde_json::json!({"cursor": cursor, "head": head})),
        ),
        _ => (ProtocolErrorCode::InternalError, None),
    };
    send_error(socket, id, code, error.to_string(), data)
        .await
        .is_ok()
}

async fn send_runtime_error(
    socket: &mut WebSocket,
    id: RpcId,
    error: NeuroLinkRuntimeError,
) -> bool {
    let code = match error {
        NeuroLinkRuntimeError::Busy(_) => ProtocolErrorCode::Busy,
        NeuroLinkRuntimeError::Unavailable(_) => ProtocolErrorCode::ServiceUnavailable,
        NeuroLinkRuntimeError::Rejected(_) => ProtocolErrorCode::InvalidParams,
        NeuroLinkRuntimeError::Internal(_) => ProtocolErrorCode::InternalError,
    };
    send_error(socket, id, code, error.to_string(), None)
        .await
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::bus::{SessionAdmissionObservation, SessionAdmissionPhase};

    struct FakeRuntime;

    #[async_trait]
    impl NeuroLinkRuntime for FakeRuntime {
        async fn start_turn(
            &self,
            envelope: ChannelEnvelopeV1,
        ) -> Result<TurnStartResultV1, NeuroLinkRuntimeError> {
            let session_key = envelope.correlation.session_key.clone();
            let request_id = envelope.correlation.request_id.clone().unwrap_or_default();
            let trace_id = envelope.correlation.trace_id.clone().unwrap_or_default();
            Ok(TurnStartResultV1 {
                session_key: session_key.clone(),
                request_id: request_id.clone(),
                trace_id: trace_id.clone(),
                admission: SessionAdmissionObservation {
                    code: None,
                    phase: SessionAdmissionPhase::Running,
                    session_key,
                    request_id,
                    trace_id,
                    queue_depth: 0,
                    wait_latency_ms: 0,
                },
            })
        }
    }

    #[test]
    fn catalog_is_revisioned_and_contains_only_v1_methods() {
        let catalog = service_catalog();
        assert_eq!(catalog.catalog_revision, CATALOG_REVISION);
        assert!(catalog.services.len() >= 14);
        assert!(catalog.services[0]
            .methods
            .iter()
            .all(|method| method.contains('/')));
    }

    #[test]
    fn attachment_and_part_limits_are_enforced() {
        let too_many = vec![
            ContentPart::Text {
                text: "x".to_string(),
            };
            MAX_PARTS + 1
        ];
        assert_eq!(
            validate_parts(&too_many).unwrap_err().code,
            ProtocolErrorCode::InvalidParams
        );
        let too_large = vec![ContentPart::File {
            attachment: agent_diva_core::channel::AttachmentRef {
                uri: "file:///oversized".to_string(),
                media_type: "application/octet-stream".to_string(),
                size_bytes: MAX_ATTACHMENT_BYTES + 1,
                sha256: "0".repeat(64),
                file_name: None,
            },
        }];
        assert_eq!(
            validate_parts(&too_large).unwrap_err().code,
            ProtocolErrorCode::AttachmentTooLarge
        );
    }

    #[test]
    fn envelope_has_owner_frontend_and_request_trace_correlation() {
        let connection = ConnectionState {
            frontend_instance_id: Some("frontend".to_string()),
            ..Default::default()
        };
        let params = TurnStartParams {
            session_key: "session".to_string(),
            thread_id: Some("thread".to_string()),
            parts: vec![ContentPart::Text {
                text: "hello".to_string(),
            }],
            client_message_id: Some("client".to_string()),
            subject: None,
            locale: None,
        };
        let envelope = make_turn_envelope(&connection, &params, "request", "trace");
        assert_eq!(envelope.origin, ChannelOrigin::OwnerFrontend);
        assert_eq!(envelope.correlation.request_id.as_deref(), Some("request"));
        assert_eq!(envelope.correlation.trace_id.as_deref(), Some("trace"));
        assert_eq!(envelope.address.sender_id.as_deref(), Some("frontend"));
    }

    #[tokio::test]
    async fn loopback_websocket_handshake_session_and_turn_smoke() {
        use agent_diva_core::bus::MessageBus;
        use futures::{SinkExt, StreamExt};
        use tempfile::TempDir;
        use tokio_tungstenite::tungstenite::Message as ClientMessage;

        async fn next_json<S>(socket: &mut tokio_tungstenite::WebSocketStream<S>) -> Value
        where
            S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
        {
            let frame = socket.next().await.unwrap().unwrap();
            serde_json::from_str(&frame.into_text().unwrap()).unwrap()
        }

        let temp = TempDir::new().unwrap();
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let state = AppState::new(api_tx, MessageBus::new(), temp.path())
            .unwrap()
            .with_neuro_link_runtime(std::sync::Arc::new(FakeRuntime));
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let address = listener.local_addr().unwrap();
        let app = crate::server::build_router(state);
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await
                .unwrap();
        });

        let url = format!("ws://{address}/api/neuro-link/v1/ws");
        let (mut socket, _) = tokio_tungstenite::connect_async(url).await.unwrap();
        socket
            .send(ClientMessage::Text(
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": "hello",
                    "method": "protocol/hello",
                    "params": {
                        "protocol": NEURO_LINK_PROTOCOL_V1,
                        "schema_version": CHANNEL_SCHEMA_VERSION_V1,
                        "frontend_instance_id": "smoke-frontend",
                        "capabilities": ["state_sync"]
                    }
                })
                .to_string(),
            ))
            .await
            .unwrap();
        let hello = next_json(&mut socket).await;
        assert_eq!(hello["result"]["protocol"], NEURO_LINK_PROTOCOL_V1);

        socket
            .send(ClientMessage::Text(
                serde_json::json!({
                    "jsonrpc": "2.0", "id": "open", "method": "session/open",
                    "params": {"session_key": "smoke-session"}
                })
                .to_string(),
            ))
            .await
            .unwrap();
        let opened = next_json(&mut socket).await;
        assert_eq!(
            opened["result"]["opened"]["catalog_revision"],
            CATALOG_REVISION
        );
        let opened_notification = next_json(&mut socket).await;
        assert_eq!(opened_notification["method"], "session/opened");

        socket
            .send(ClientMessage::Text(
                serde_json::json!({
                    "jsonrpc": "2.0", "id": "turn", "method": "turn/start",
                    "params": {
                        "session_key": "smoke-session",
                        "client_message_id": "client-1",
                        "parts": [{"kind": "text", "text": "hello"}]
                    }
                })
                .to_string(),
            ))
            .await
            .unwrap();
        let turn = next_json(&mut socket).await;
        assert_eq!(turn["result"]["session_key"], "smoke-session");
        assert_eq!(turn["result"]["admission"]["phase"], "running");
        let admission = next_json(&mut socket).await;
        assert_eq!(admission["method"], "turn/admission");
        assert_eq!(
            admission["params"]["envelope"]["correlation"]["session_key"],
            "smoke-session"
        );

        let _ = shutdown_tx.send(());
        server.abort();
    }
}
