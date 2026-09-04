pub mod approvals;
pub mod ask_user;
pub mod audit;
pub mod autodream;
pub mod command_approvals;
pub mod health;
pub mod laputa;
pub mod logs;
pub mod memory;
pub mod persona;
pub mod planning;
mod provider_companion;
pub mod skills;
pub mod todo;
pub mod token_stats;
pub mod workspace;

pub use audit::{get_audit_events_handler, get_audit_log_handler};
pub use logs::{logs_routes, query_logs_handler};
pub use memory::{
    create_memory_record_handler, delete_actmem_capsule_handler, delete_memory_record_handler,
    get_actmem_capsule_handler, get_actmem_handler, get_memory_record_handler,
    get_memrules_handler, list_actmem_capsules_handler, list_memory_records_handler,
    put_actmem_handler, put_memrules_handler, update_memory_record_handler,
};
pub use todo::{create_todo_handler, query_todos_handler, todo_routes, update_todo_handler};
pub use token_stats::token_stats_routes;

pub use approvals::approval_routes;
pub use ask_user::ask_user_routes;
pub use command_approvals::command_approval_routes;
pub use health::health_handler;
pub use skills::{
    accept_skill_request_handler, create_skill_request_handler, delete_skill_handler,
    disable_skill_handler, featured_marketplace_skills_handler, get_skill_handler,
    get_skill_history_revision_handler, get_skill_request_handler, get_skills_handler,
    install_marketplace_skill_handler, list_skill_history_handler, list_skill_requests_handler,
    reject_skill_request_handler, search_marketplace_skills_handler, update_skill_handler,
    upload_skill_handler,
};
pub use workspace::{get_workspace_handler, WorkspaceStatusResponse};

pub use autodream::{
    cancel_autodream_run_handler, get_autodream_live_text_handler, get_autodream_run_handler,
    list_autodream_run_events_handler, list_autodream_runs_handler, trigger_autodream_run_handler,
};
pub use laputa::list_recall_feedback_handler;
pub use persona::{
    accept_persona_request_handler, create_persona_request_handler, get_persona_document_handler,
    get_persona_history_revision_handler, get_persona_status_handler, initialize_persona_handler,
    list_persona_history_handler, list_persona_requests_handler, reject_persona_request_handler,
    repair_persona_handler, save_persona_document_handler,
};

pub use provider_companion::{
    add_provider_model_handler, create_provider_handler, delete_provider_handler,
    delete_provider_model_handler, get_provider_handler, get_provider_models_handler,
    get_providers_handler, resolve_provider_handler, update_provider_handler,
};

use agent_diva_agent::{runtime_control::RuntimeControlCommand, AgentEvent};
use agent_diva_core::bus::AgentBusEvent;
use agent_diva_core::channel::{
    AttachmentRef, ChannelAddress, ChannelDirection, ChannelEnvelopeV1, ChannelOrigin,
    ChannelPayloadV1, ContentPart, Correlation, OwnerApprovalPolicy, OwnerExecutionContextV1,
    OwnerTurnContextV1, OwnerTurnIntent,
};
use agent_diva_core::config::schema::{ChannelsConfig, SelfEvolutionConfig};
use agent_diva_core::config::ConfigLoader;
use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::sse::{Event, Sse},
    Json,
};
use futures::stream::{Stream, StreamExt};
use std::convert::Infallible;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::state::{
    ApiRequest, AppState, ChannelUpdate, ConfigResponse, ConfigUpdate, FileUploadRequest,
    GenerateSessionTitleRequest, ManagerCommand, McpRefreshRequest, RunCronJobRequest,
    SetCronJobEnabledRequest, SetMcpEnabledRequest, StopChatRequest, ToolsConfigResponse,
    ToolsConfigUpdate,
};

#[derive(serde::Deserialize)]
pub struct ChatRequest {
    pub message: String,
    #[serde(default)]
    pub request_id: Option<String>,
    pub channel: Option<String>,
    pub chat_id: Option<String>,
    pub attachments: Option<Vec<String>>,
    pub mode: Option<String>,
    pub execution_start: Option<bool>,
    pub plan_id: Option<String>,
    pub plan_revision: Option<i64>,
    pub execution_id: Option<String>,
    /// Optional approval-policy override ("on-request" / "on-failure" / "unless-trusted" / "never").
    pub approval_policy: Option<String>,
}

fn normalized_owner_intent(mode: Option<&str>) -> Option<&'static str> {
    match mode.map(str::trim) {
        None => None,
        Some("agent") => Some("agent"),
        Some("plan") => Some("plan"),
        Some("ask") => Some("ask"),
        Some(_) => Some("ask"),
    }
}

fn owner_turn_intent(mode: Option<&str>) -> OwnerTurnIntent {
    match normalized_owner_intent(mode) {
        Some("plan") => OwnerTurnIntent::Plan,
        Some("ask") => OwnerTurnIntent::Ask,
        _ => OwnerTurnIntent::Agent,
    }
}

fn owner_approval_policy(raw: Option<&str>) -> Option<OwnerApprovalPolicy> {
    parse_approval_policy(raw).map(|policy| match policy {
        agent_diva_sandbox::AskForApproval::OnRequest => OwnerApprovalPolicy::OnRequest,
        agent_diva_sandbox::AskForApproval::OnFailure => OwnerApprovalPolicy::OnFailure,
        agent_diva_sandbox::AskForApproval::UnlessTrusted => OwnerApprovalPolicy::UnlessTrusted,
        agent_diva_sandbox::AskForApproval::Never => OwnerApprovalPolicy::Never,
    })
}

async fn resolve_attachment_parts(
    state: &AppState,
    references: Option<Vec<String>>,
) -> Result<Vec<ContentPart>, String> {
    let Some(references) = references else {
        return Ok(Vec::new());
    };
    let Some(authority) = state.attachment_authority.as_ref() else {
        return Err("attachment authority is not initialized".to_string());
    };
    let mut parts = Vec::with_capacity(references.len());
    for reference in references {
        let id = reference.trim();
        if id.is_empty() {
            return Err("attachment reference must not be empty".to_string());
        }
        let handle = authority
            .get(id)
            .await
            .map_err(|error| format!("invalid attachment reference {id}: {error}"))?;
        let media_type = handle
            .metadata
            .mime_type
            .clone()
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let sha256 = attachment_digest(&handle.id)?;
        let attachment = AttachmentRef {
            uri: handle.id.clone(),
            media_type: media_type.clone(),
            size_bytes: handle.metadata.size,
            sha256: sha256.to_string(),
            file_name: Some(handle.metadata.name.clone()),
        };
        parts.push(match media_type.as_str() {
            value if value.starts_with("image/") => ContentPart::Image { attachment },
            value if value.starts_with("audio/") => ContentPart::Audio {
                attachment,
                transcript: None,
            },
            value if value.starts_with("video/") => ContentPart::Video { attachment },
            _ => ContentPart::File { attachment },
        });
    }
    Ok(parts)
}

fn attachment_digest(handle_id: &str) -> Result<&str, String> {
    handle_id
        .strip_prefix("sha256:")
        .filter(|digest| !digest.is_empty())
        .ok_or_else(|| format!("attachment authority returned non-sha256 id: {handle_id}"))
}

async fn build_typed_chat_envelope(
    state: &AppState,
    channel: String,
    chat_id: String,
    request_id: String,
    payload: ChatRequest,
) -> Result<ChannelEnvelopeV1, String> {
    let ChatRequest {
        message,
        attachments,
        mode,
        execution_start,
        plan_id,
        plan_revision,
        execution_id,
        approval_policy,
        ..
    } = payload;
    let mut address = ChannelAddress::new(channel.clone(), chat_id.clone());
    address.sender_id = Some("user".to_string());
    let mut correlation = Correlation::new(format!("{channel}:{chat_id}"));
    correlation.request_id = Some(request_id);
    correlation.trace_id = Some(uuid::Uuid::new_v4().to_string());
    correlation.message_id = Some(uuid::Uuid::new_v4().to_string());

    let mut parts = vec![ContentPart::Text { text: message }];
    parts.extend(resolve_attachment_parts(state, attachments).await?);
    let execution = if execution_start.unwrap_or(false) {
        Some(OwnerExecutionContextV1 {
            plan_id: plan_id
                .ok_or_else(|| "plan_id is required for execution continuation".to_string())?,
            revision: plan_revision.ok_or_else(|| {
                "plan_revision is required for execution continuation".to_string()
            })?,
            execution_id,
        })
    } else {
        None
    };
    Ok(ChannelEnvelopeV1::new(
        ChannelDirection::Ingress,
        address,
        correlation,
        ChannelOrigin::OwnerFrontend,
        ChannelPayloadV1::Message {
            parts,
            subject: None,
            locale: None,
            context: Some(OwnerTurnContextV1 {
                intent: owner_turn_intent(mode.as_deref()),
                approval_policy: owner_approval_policy(approval_policy.as_deref()),
                execution,
            }),
        },
    ))
}

pub(crate) fn parse_approval_policy(
    raw: Option<&str>,
) -> Option<agent_diva_sandbox::AskForApproval> {
    match raw.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
        None | Some("") => None,
        Some("on-request" | "on_request" | "cautious") => {
            Some(agent_diva_sandbox::AskForApproval::OnRequest)
        }
        Some("on-failure" | "on_failure" | "smart") => {
            Some(agent_diva_sandbox::AskForApproval::OnFailure)
        }
        Some("unless-trusted" | "unless_trusted" | "trusted") => {
            Some(agent_diva_sandbox::AskForApproval::UnlessTrusted)
        }
        Some("never") => Some(agent_diva_sandbox::AskForApproval::Never),
        Some(other) => {
            tracing::warn!("Unknown approval_policy '{}', ignoring", other);
            None
        }
    }
}

#[derive(serde::Deserialize, Default)]
pub struct EventsQuery {
    pub channel: Option<String>,
    pub chat_id: Option<String>,
    pub chat_prefix: Option<String>,
    pub request_id: Option<String>,
}

pub async fn chat_handler(
    State(state): State<AppState>,
    Json(payload): Json<ChatRequest>,
) -> Sse<futures::stream::BoxStream<'static, Result<Event, Infallible>>> {
    let channel = payload.channel.clone().unwrap_or_else(|| "api".to_string());
    let chat_id = payload
        .chat_id
        .clone()
        .unwrap_or_else(|| "default".to_string());
    let request_id = payload
        .request_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.len() <= 128)
        .map(str::to_owned)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    if payload.message.trim() == "/stop" {
        let (stop_tx, stop_rx) = oneshot::channel();
        let stop_req = StopChatRequest {
            channel: Some(channel),
            chat_id: Some(chat_id),
            request_id: Some(request_id.clone()),
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

        let stream =
            futures::stream::once(
                async move { Ok(Event::default().event("error").data(stop_message)) },
            )
            .boxed();
        return Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default());
    }

    match state.persona.status() {
        Ok(status) if status.status == agent_diva_laputa::PersonaStatus::Ready => {}
        Ok(status) => {
            let data = serde_json::json!({
                "code": match status.status {
                    agent_diva_laputa::PersonaStatus::Uninitialized => "persona_uninitialized",
                    agent_diva_laputa::PersonaStatus::Incomplete => "persona_incomplete",
                    agent_diva_laputa::PersonaStatus::Ready => unreachable!(),
                },
                "message": "Persona setup must be completed before chat",
                "persona_status": status.status,
            });
            let stream = futures::stream::once(async move {
                Ok(Event::default().event("error").data(data.to_string()))
            })
            .boxed();
            return Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default());
        }
        Err(error) => {
            let data = serde_json::json!({
                "code": error.code(),
                "message": error.to_string(),
            });
            let stream = futures::stream::once(async move {
                Ok(Event::default().event("error").data(data.to_string()))
            })
            .boxed();
            return Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default());
        }
    }

    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let envelope = match build_typed_chat_envelope(
        &state,
        channel,
        chat_id,
        request_id.clone(),
        payload,
    )
    .await
    {
        Ok(envelope) => envelope,
        Err(error) => {
            let stream =
                futures::stream::once(
                    async move { Ok(Event::default().event("error").data(error)) },
                )
                .boxed();
            return Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default());
        }
    };
    let req = ApiRequest { envelope, event_tx };

    if let Err(e) = state.api_tx.send(ManagerCommand::Chat(Box::new(req))).await {
        tracing::error!("Failed to send API request to manager: {}", e);
    }

    let stream = UnboundedReceiverStream::new(event_rx)
        .map(move |event| {
            let evt = match event {
                AgentEvent::SessionAdmission { observation } => Event::default()
                    .event("session_admission")
                    .data(serde_json::to_string(&observation).unwrap_or_default()),
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
                AgentEvent::FinalResponse { content } => {
                    Event::default().event("final").data(content)
                }
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
                AgentEvent::TodoCreated { plan, todo } => Event::default()
                    .event("todo_created")
                    .data(serde_json::json!({ "plan": plan, "todo": todo }).to_string()),
                AgentEvent::TodoStepUpdated { plan, todo } => Event::default()
                    .event("todo_step_updated")
                    .data(serde_json::json!({ "plan": plan, "todo": todo }).to_string()),
                AgentEvent::TodoCompleted { plan, todo } => Event::default()
                    .event("todo_completed")
                    .data(serde_json::json!({ "plan": plan, "todo": todo }).to_string()),
                AgentEvent::TodoCancelled { plan, todo } => Event::default()
                    .event("todo_cancelled")
                    .data(serde_json::json!({ "plan": plan, "todo": todo }).to_string()),
                AgentEvent::PlanReadyForApproval { plan } => Event::default()
                    .event("plan_ready_for_approval")
                    .data(serde_json::json!({ "plan": plan }).to_string()),
                AgentEvent::PlanReportReadyForApproval { report } => Event::default()
                    .event("plan_report_ready_for_approval")
                    .data(serde_json::json!({ "report": report }).to_string()),
                AgentEvent::ChatPlanUpdate { args } => Event::default()
                    .event("turn_plan_updated")
                    .data(serde_json::to_string(&args).unwrap()),
                AgentEvent::Error { message } => Event::default().event("error").data(message),
                AgentEvent::ProviderRetry {
                    model,
                    attempt,
                    max_retries,
                    delay_ms,
                    reason,
                } => {
                    let data = serde_json::json!({
                        "model": model,
                        "attempt": attempt,
                        "max_retries": max_retries,
                        "delay_ms": delay_ms,
                        "reason": reason
                    });
                    Event::default()
                        .event("provider_retry")
                        .data(data.to_string())
                }
                AgentEvent::ProviderStalled { model } => {
                    let data = serde_json::json!({ "model": model });
                    Event::default()
                        .event("provider_stalled")
                        .data(data.to_string())
                }
                AgentEvent::ContextCompaction {
                    session_id,
                    trigger,
                    phase,
                    summary,
                } => {
                    let data = serde_json::json!({
                        "session_id": session_id,
                        "trigger": trigger,
                        "phase": phase,
                        "summary": summary,
                    });
                    Event::default()
                        .event("context_compaction")
                        .data(data.to_string())
                }
                _ => Event::default().comment("keep-alive"),
            };
            Ok(evt.id(request_id.clone()))
        })
        .boxed();

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

pub async fn stop_chat_handler(
    State(state): State<AppState>,
    Json(payload): Json<StopChatRequest>,
) -> Json<serde_json::Value> {
    if let (Some(channel), Some(chat_id)) = (payload.channel.clone(), payload.chat_id.clone()) {
        let scope = agent_diva_sandbox::CommandApprovalScope {
            session_key: format!("{channel}:{chat_id}"),
            channel,
            chat_id,
        };
        state.command_approvals.cancel_scope(&scope).await;
    }
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
        Ok(Ok(outcome)) => Json(serde_json::json!({
            "status": "ok",
            "stopped": outcome.running_cancelled,
            "outcome": outcome,
        })),
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
        Ok(Ok(outcome)) => Json(serde_json::json!({
            "status": "ok",
            "reset": outcome.cleanup_complete,
            "outcome": outcome,
        })),
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
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::GetSessionHistory(session_key.clone(), tx))
        .await
    {
        tracing::error!("Failed to send GetSessionHistory request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }

    match rx.await {
        Ok(Ok(Some(session))) => Json(serde_json::json!({ "status": "ok", "session": session })),
        Ok(Ok(None)) => {
            Json(serde_json::json!({ "status": "error", "message": "Session not found" }))
        }
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => {
            tracing::error!("Failed to receive GetSessionHistory response: {}", e);
            Json(serde_json::json!({ "status": "error", "message": e.to_string() }))
        }
    }
}

pub async fn delete_session_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    do_delete_session(state, id).await
}

async fn do_delete_session(state: AppState, id: String) -> Json<serde_json::Value> {
    let session_key = if !id.contains(':') {
        format!("gui:{}", id)
    } else {
        id
    };

    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::DeleteSession(session_key.clone(), tx))
        .await
    {
        tracing::error!("Failed to send DeleteSession request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }

    match rx.await {
        Ok(Ok(deleted)) => {
            tracing::info!(
                session_key = %session_key,
                deleted,
                "DeleteSession completed"
            );
            Json(serde_json::json!({ "status": "ok", "deleted": deleted }))
        }
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => {
            tracing::error!("Failed to receive DeleteSession response: {}", e);
            Json(serde_json::json!({ "status": "error", "message": e.to_string() }))
        }
    }
}

pub async fn update_session_title_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let session_key = if !id.contains(':') {
        format!("gui:{}", id)
    } else {
        id
    };

    let new_title = body
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::UpdateSessionTitle(
            session_key.clone(),
            new_title,
            tx,
        ))
        .await
    {
        tracing::error!("Failed to send UpdateSessionTitle request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }

    match rx.await {
        Ok(Ok(Some(title))) => Json(serde_json::json!({ "status": "ok", "title": title })),
        Ok(Ok(None)) => Json(serde_json::json!({ "status": "ok", "title": null })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => {
            tracing::error!("Failed to receive UpdateSessionTitle response: {}", e);
            Json(serde_json::json!({ "status": "error", "message": e.to_string() }))
        }
    }
}

pub async fn generate_session_title_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<GenerateSessionTitleRequest>,
) -> Json<serde_json::Value> {
    let session_key = if !id.contains(':') {
        format!("gui:{}", id)
    } else {
        id
    };

    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::GenerateSessionTitle(
            session_key,
            payload,
            tx,
        ))
        .await
    {
        tracing::error!("Failed to send GenerateSessionTitle request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }

    match rx.await {
        Ok(Ok(response)) => Json(serde_json::json!({
            "status": "ok",
            "title": response.title,
            "title_generated": response.title_generated,
            "title_manually_set": response.title_manually_set,
        })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => {
            tracing::error!("Failed to receive GenerateSessionTitle response: {}", e);
            Json(serde_json::json!({ "status": "error", "message": e.to_string() }))
        }
    }
}

fn agent_bus_event_to_sse(bus_event: &AgentBusEvent) -> Option<Event> {
    let event = match &bus_event.event {
        AgentEvent::SessionAdmission { observation } => Some(
            Event::default()
                .event("session_admission")
                .data(serde_json::to_string(observation).ok()?),
        ),
        AgentEvent::FinalResponse { content } => {
            let data = serde_json::json!({
                "channel": &bus_event.channel,
                "chat_id": &bus_event.chat_id,
                "session_key": &bus_event.session_key,
                "request_id": &bus_event.request_id,
                "trace_id": &bus_event.trace_id,
                "content": content
            });
            Some(Event::default().event("final").data(data.to_string()))
        }
        AgentEvent::Error { message } => {
            let data = serde_json::json!({
                "channel": &bus_event.channel,
                "chat_id": &bus_event.chat_id,
                "session_key": &bus_event.session_key,
                "request_id": &bus_event.request_id,
                "trace_id": &bus_event.trace_id,
                "message": message
            });
            Some(Event::default().event("error").data(data.to_string()))
        }
        AgentEvent::ChatPlanUpdate { args } => {
            let data = serde_json::json!({
                "channel": &bus_event.channel,
                "chat_id": &bus_event.chat_id,
                "session_key": &bus_event.session_key,
                "request_id": &bus_event.request_id,
                "trace_id": &bus_event.trace_id,
                "args": args,
            });
            Some(
                Event::default()
                    .event("turn_plan_updated")
                    .data(data.to_string()),
            )
        }
        _ => None,
    };
    event.map(|event| match bus_event.request_id.as_deref() {
        Some(request_id) => event.id(request_id),
        None => event,
    })
}

pub async fn events_handler(
    State(state): State<AppState>,
    Query(query): Query<EventsQuery>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let event_rx = state.bus.subscribe_events();
    let channel_filter = query.channel;
    let chat_id_filter = query.chat_id;
    let chat_prefix_filter = query.chat_prefix;
    let request_id_filter = query.request_id;

    let stream = BroadcastStream::new(event_rx).filter_map(move |evt| {
        let channel_filter = channel_filter.clone();
        let chat_id_filter = chat_id_filter.clone();
        let chat_prefix_filter = chat_prefix_filter.clone();
        let request_id_filter = request_id_filter.clone();
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
            if let Some(request_id) = &request_id_filter {
                if bus_event.request_id.as_ref() != Some(request_id) {
                    return None;
                }
            }

            agent_bus_event_to_sse(&bus_event).map(Ok)
        }
    });

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

pub async fn get_config_handler(State(state): State<AppState>) -> Json<ConfigResponse> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state.api_tx.send(ManagerCommand::GetConfig(tx)).await {
        tracing::error!("Failed to send GetConfig request: {}", e);
        return Json(ConfigResponse {
            provider: Some("deepseek".to_string()),
            api_base: None,
            model: "deepseek-chat".to_string(),
            has_api_key: false,
        });
    }

    match rx.await {
        Ok(resp) => Json(resp),
        Err(e) => {
            tracing::error!("Failed to receive GetConfig response: {}", e);
            Json(ConfigResponse {
                provider: Some("deepseek".to_string()),
                api_base: None,
                model: "deepseek-chat".to_string(),
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

pub async fn get_self_evolution_config_handler(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::GetSelfEvolutionConfig(tx))
        .await
    {
        tracing::error!("Failed to send GetSelfEvolutionConfig request: {}", e);
        return Json(serde_json::json!({
            "status": "error",
            "message": e.to_string(),
        }));
    }

    match rx.await {
        Ok(Ok(config)) => Json(serde_json::json!({ "status": "ok", "config": config })),
        Ok(Err(message)) => Json(serde_json::json!({ "status": "error", "message": message })),
        Err(e) => {
            tracing::error!("Failed to receive GetSelfEvolutionConfig response: {}", e);
            Json(serde_json::json!({ "status": "error", "message": e.to_string() }))
        }
    }
}

pub async fn update_self_evolution_config_handler(
    State(state): State<AppState>,
    Json(payload): Json<SelfEvolutionConfig>,
) -> Json<serde_json::Value> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::UpdateSelfEvolutionConfig(payload, tx))
        .await
    {
        tracing::error!("Failed to send UpdateSelfEvolutionConfig request: {}", e);
        return Json(serde_json::json!({
            "status": "error",
            "message": e.to_string(),
        }));
    }

    match rx.await {
        Ok(Ok(config)) => Json(serde_json::json!({ "status": "ok", "config": config })),
        Ok(Err(message)) => Json(serde_json::json!({ "status": "error", "message": message })),
        Err(e) => {
            tracing::error!(
                "Failed to receive UpdateSelfEvolutionConfig response: {}",
                e
            );
            Json(serde_json::json!({ "status": "error", "message": e.to_string() }))
        }
    }
}

pub async fn get_channels_handler(State(state): State<AppState>) -> Json<ChannelsConfig> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state.api_tx.send(ManagerCommand::GetChannels(tx)).await {
        tracing::error!("Failed to send GetChannels request: {}", e);
        return Json(channels_from_disk_fallback(&state));
    }
    match rx.await {
        Ok(config) => Json(config),
        Err(e) => {
            tracing::error!("Failed to receive GetChannels response: {}", e);
            Json(channels_from_disk_fallback(&state))
        }
    }
}

fn channels_from_disk_fallback(state: &AppState) -> ChannelsConfig {
    match ConfigLoader::with_dir(&state.config_dir).load() {
        Ok(config) => {
            tracing::warn!("Runtime unavailable; serving channels from config file");
            config.channels
        }
        Err(e) => {
            tracing::warn!("Failed to load channels from config file: {}", e);
            ChannelsConfig::default()
        }
    }
}

pub async fn get_tools_handler(State(state): State<AppState>) -> Json<ToolsConfigResponse> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state.api_tx.send(ManagerCommand::GetTools(tx)).await {
        tracing::error!("Failed to send GetTools request: {}", e);
        return Json(ToolsConfigResponse {
            web: agent_diva_core::config::schema::WebToolsConfig::default().into(),
            budget: agent_diva_core::config::CompactionBudgetConfig::default(),
        });
    }
    match rx.await {
        Ok(config) => Json(config),
        Err(e) => {
            tracing::error!("Failed to receive GetTools response: {}", e);
            Json(ToolsConfigResponse {
                web: agent_diva_core::config::schema::WebToolsConfig::default().into(),
                budget: agent_diva_core::config::CompactionBudgetConfig::default(),
            })
        }
    }
}

pub async fn get_mcps_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state.api_tx.send(ManagerCommand::GetMcps(tx)).await {
        tracing::error!("Failed to send GetMcps request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }
    match rx.await {
        Ok(Ok(mcps)) => Json(serde_json::json!({ "status": "ok", "mcps": mcps })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    }
}

pub async fn create_mcp_handler(
    State(state): State<AppState>,
    Json(payload): Json<crate::mcp_service::McpServerUpsert>,
) -> Json<serde_json::Value> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::CreateMcp(payload, tx))
        .await
    {
        tracing::error!("Failed to send CreateMcp request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }
    match rx.await {
        Ok(Ok(mcp)) => Json(serde_json::json!({ "status": "ok", "mcp": mcp })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    }
}

pub async fn update_mcp_handler(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(payload): Json<crate::mcp_service::McpServerUpsert>,
) -> Json<serde_json::Value> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::UpdateMcp(name, payload, tx))
        .await
    {
        tracing::error!("Failed to send UpdateMcp request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }
    match rx.await {
        Ok(Ok(mcp)) => Json(serde_json::json!({ "status": "ok", "mcp": mcp })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    }
}

pub async fn delete_mcp_handler(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<serde_json::Value> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state.api_tx.send(ManagerCommand::DeleteMcp(name, tx)).await {
        tracing::error!("Failed to send DeleteMcp request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }
    match rx.await {
        Ok(Ok(())) => Json(serde_json::json!({ "status": "ok" })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    }
}

pub async fn set_mcp_enabled_handler(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(payload): Json<SetMcpEnabledRequest>,
) -> Json<serde_json::Value> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::SetMcpEnabled(name, payload.enabled, tx))
        .await
    {
        tracing::error!("Failed to send SetMcpEnabled request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }
    match rx.await {
        Ok(Ok(mcp)) => Json(serde_json::json!({ "status": "ok", "mcp": mcp })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    }
}

pub async fn refresh_mcp_status_handler(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(_payload): Json<McpRefreshRequest>,
) -> Json<serde_json::Value> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::RefreshMcpStatus(name, tx))
        .await
    {
        tracing::error!("Failed to send RefreshMcpStatus request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }
    match rx.await {
        Ok(Ok(mcp)) => Json(serde_json::json!({ "status": "ok", "mcp": mcp })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    }
}

pub async fn upload_file_handler(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Json<serde_json::Value> {
    let mut file_name: Option<String> = None;
    let mut bytes: Option<Vec<u8>> = None;
    let mut channel: Option<String> = None;
    let mut message_id: Option<String> = None;

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(e) => {
                return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
            }
        };
        let name = field.name().map(ToString::to_string);
        match name.as_deref() {
            Some("file") => {
                file_name = field.file_name().map(ToString::to_string);
                match field.bytes().await {
                    Ok(body) => bytes = Some(body.to_vec()),
                    Err(e) => {
                        return Json(
                            serde_json::json!({ "status": "error", "message": e.to_string() }),
                        );
                    }
                }
            }
            Some("channel") => match field.text().await {
                Ok(text) => channel = Some(text),
                Err(e) => {
                    return Json(
                        serde_json::json!({ "status": "error", "message": e.to_string() }),
                    );
                }
            },
            Some("message_id") => {
                if let Ok(text) = field.text().await {
                    message_id = Some(text);
                }
            }
            _ => {}
        }
    }

    let Some(file_name) = file_name else {
        return Json(serde_json::json!({ "status": "error", "message": "missing file upload" }));
    };
    let Some(bytes) = bytes else {
        return Json(serde_json::json!({ "status": "error", "message": "missing file body" }));
    };
    let Some(channel) = channel else {
        return Json(serde_json::json!({ "status": "error", "message": "missing channel" }));
    };

    let (tx, rx) = oneshot::channel();
    let request = FileUploadRequest {
        file_name,
        bytes,
        channel,
        message_id: message_id.clone(),
    };
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::UploadFile(request, tx))
        .await
    {
        tracing::error!("Failed to send UploadFile request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }
    match rx.await {
        Ok(Ok(attachment)) => Json(serde_json::json!({ "status": "ok", "attachment": attachment })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
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
    let (reply_tx, reply_rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::UpdateChannel(payload, reply_tx))
        .await
    {
        tracing::error!("Failed to send UpdateChannel request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }

    match reply_rx.await {
        Ok(Ok(())) => Json(serde_json::json!({ "status": "ok" })),
        Ok(Err(message)) => Json(serde_json::json!({ "status": "error", "message": message })),
        Err(error) => Json(serde_json::json!({ "status": "error", "message": error.to_string() })),
    }
}

fn channel_probe_error_response(
    error: agent_diva_channels::ChannelProbeError,
) -> (StatusCode, Json<serde_json::Value>) {
    use agent_diva_channels::ChannelProbeError;

    let (status, message) = if error.public_code() == "probe_busy" {
        (
            StatusCode::TOO_MANY_REQUESTS,
            "channel probe is already in progress",
        )
    } else {
        match &error {
            ChannelProbeError::UnknownChannel { .. } | ChannelProbeError::InvalidConfig => {
                (StatusCode::BAD_REQUEST, "invalid channel probe request")
            }
            ChannelProbeError::Timeout => (StatusCode::GATEWAY_TIMEOUT, "channel probe timed out"),
            ChannelProbeError::Adapter { .. } => (StatusCode::BAD_GATEWAY, "channel probe failed"),
            ChannelProbeError::Build => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "channel probe unavailable",
            ),
            ChannelProbeError::Cleanup { .. } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "channel probe cleanup failed",
            ),
        }
    };
    (
        status,
        Json(serde_json::json!({
            "status": "error",
            "code": error.public_code(),
            "message": message,
            "retryable": error.retryable(),
            "retry_after_ms": error.retry_after_ms(),
        })),
    )
}

pub async fn probe_channel_handler(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(payload): Json<crate::state::ChannelProbeRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let name = name.trim().to_string();
    let (reply_tx, reply_rx) = oneshot::channel();
    state
        .api_tx
        .send(ManagerCommand::ProbeChannel(
            name.clone(),
            payload.config,
            reply_tx,
        ))
        .await
        .map_err(|_error| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "status": "error",
                    "code": "manager_unavailable",
                    "message": "channel manager is unavailable",
                    "retryable": true,
                    "retry_after_ms": null,
                })),
            )
        })?;

    match tokio::time::timeout(std::time::Duration::from_secs(37), reply_rx).await {
        Err(_) => Err((
            StatusCode::GATEWAY_TIMEOUT,
            Json(serde_json::json!({
                "status": "error",
                "code": "channel_probe_timeout",
                "message": "channel probe exceeded its bounded deadline",
                "retryable": true,
                "retry_after_ms": null,
            })),
        )),
        Ok(Err(_)) => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "status": "error",
                "code": "manager_unavailable",
                "message": "channel manager is unavailable",
                "retryable": true,
                "retry_after_ms": null,
            })),
        )),
        Ok(Ok(Ok(receipt))) => Ok(Json(serde_json::json!({
            "status": "ok",
            "channel": name,
            "receipt": receipt,
        }))),
        Ok(Ok(Err(error))) => Err(channel_probe_error_response(error)),
    }
}

pub async fn delete_channel_handler(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let name = name.trim().to_string();
    let (reply_tx, reply_rx) = oneshot::channel();
    state
        .api_tx
        .send(ManagerCommand::DeleteChannel(name.clone(), reply_tx))
        .await
        .map_err(|_| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "status": "error",
                    "code": "manager_unavailable",
                    "message": "channel manager is unavailable",
                })),
            )
        })?;

    match reply_rx.await {
        Ok(Ok(())) => Ok(Json(serde_json::json!({
            "status": "ok",
            "channel": name,
            "removed": true,
        }))),
        Ok(Err(message)) => {
            let (status, code, safe_message) = if message.starts_with("Unknown channel:") {
                (StatusCode::BAD_REQUEST, "unknown_channel", message)
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "channel_delete_failed",
                    "channel deletion failed".to_string(),
                )
            };
            Err((
                status,
                Json(serde_json::json!({
                    "status": "error",
                    "code": code,
                    "message": safe_message,
                })),
            ))
        }
        Err(_) => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "status": "error",
                "code": "manager_unavailable",
                "message": "channel manager is unavailable",
            })),
        )),
    }
}

pub async fn get_channel_runtime_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let (reply_tx, reply_rx) = oneshot::channel();
    if let Err(error) = state
        .api_tx
        .send(ManagerCommand::GetChannelRuntime(reply_tx))
        .await
    {
        return Json(serde_json::json!({ "status": "error", "message": error.to_string() }));
    }
    match reply_rx.await {
        Ok(channels) => Json(serde_json::json!({ "status": "ok", "channels": channels })),
        Err(error) => Json(serde_json::json!({ "status": "error", "message": error.to_string() })),
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct CompactSessionRequest {
    pub session_key: String,
}

pub async fn compact_session_handler(
    State(state): State<AppState>,
    Json(payload): Json<CompactSessionRequest>,
) -> Json<serde_json::Value> {
    let Some(control_tx) = state.runtime_control_tx.clone() else {
        return Json(
            serde_json::json!({ "status": "error", "message": "AgentLoop runtime unavailable" }),
        );
    };
    let session_key = payload.session_key.trim().to_string();
    if session_key.is_empty() {
        return Json(
            serde_json::json!({ "status": "error", "message": "session_key is required" }),
        );
    }
    let (reply_tx, reply_rx) = oneshot::channel();
    if control_tx
        .send(RuntimeControlCommand::CompactSession {
            session_key,
            reply_tx,
        })
        .await
        .is_err()
    {
        return Json(
            serde_json::json!({ "status": "error", "message": "AgentLoop control lane is closed" }),
        );
    }
    match reply_rx.await {
        Ok(Ok(summary)) => Json(serde_json::json!({ "status": "ok", "summary": summary })),
        Ok(Err(message)) => Json(serde_json::json!({ "status": "error", "message": message })),
        Err(error) => Json(serde_json::json!({ "status": "error", "message": error.to_string() })),
    }
}

pub async fn heartbeat_handler() -> &'static str {
    "ok"
}

pub async fn list_cron_jobs_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state.api_tx.send(ManagerCommand::ListCronJobs(tx)).await {
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }
    match rx.await {
        Ok(Ok(jobs)) => Json(serde_json::json!({ "status": "ok", "jobs": jobs })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    }
}

pub async fn get_cron_job_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state.api_tx.send(ManagerCommand::GetCronJob(id, tx)).await {
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }
    match rx.await {
        Ok(Ok(Some(job))) => Json(serde_json::json!({ "status": "ok", "job": job })),
        Ok(Ok(None)) => Json(serde_json::json!({ "status": "error", "message": "Job not found" })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    }
}

pub async fn create_cron_job_handler(
    State(state): State<AppState>,
    Json(payload): Json<agent_diva_core::cron::CreateCronJobRequest>,
) -> Json<serde_json::Value> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::CreateCronJob(payload, tx))
        .await
    {
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }
    match rx.await {
        Ok(Ok(job)) => Json(serde_json::json!({ "status": "ok", "job": job })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    }
}

pub async fn update_cron_job_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<agent_diva_core::cron::UpdateCronJobRequest>,
) -> Json<serde_json::Value> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::UpdateCronJob(id, payload, tx))
        .await
    {
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }
    match rx.await {
        Ok(Ok(job)) => Json(serde_json::json!({ "status": "ok", "job": job })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    }
}

pub async fn set_cron_job_enabled_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<SetCronJobEnabledRequest>,
) -> Json<serde_json::Value> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::SetCronJobEnabled(id, payload.enabled, tx))
        .await
    {
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }
    match rx.await {
        Ok(Ok(job)) => Json(serde_json::json!({ "status": "ok", "job": job })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    }
}

pub async fn run_cron_job_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<RunCronJobRequest>,
) -> Json<serde_json::Value> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::RunCronJobNow(id, payload.force, tx))
        .await
    {
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }
    match rx.await {
        Ok(Ok(job)) => Json(serde_json::json!({ "status": "ok", "job": job })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    }
}

pub async fn stop_cron_job_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::StopCronJobRun(id, tx))
        .await
    {
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }
    match rx.await {
        Ok(Ok(run)) => Json(serde_json::json!({ "status": "ok", "run": run })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    }
}

pub async fn delete_cron_job_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::DeleteCronJob(id, tx))
        .await
    {
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }
    match rx.await {
        Ok(Ok(())) => Json(serde_json::json!({ "status": "ok" })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        agent_bus_event_to_sse, attachment_digest, build_typed_chat_envelope,
        channel_probe_error_response, chat_handler, generate_session_title_handler,
        get_session_history_handler, get_sessions_handler, normalized_owner_intent,
        parse_approval_policy, update_session_title_handler, AgentEvent, ChatRequest, Sse,
    };
    use crate::state::{AppState, GenerateSessionTitleResponse, ManagerCommand};
    use agent_diva_core::bus::AgentEventBus;
    use agent_diva_core::session::store::{ChatMessage, Session};
    use agent_diva_core::session::{SessionInfo, SessionKind};
    use agent_diva_files::handle::FileMetadata;
    use agent_diva_files::{FileConfig, FileManager};
    use axum::{
        body::to_bytes,
        extract::{Path, State},
        http::StatusCode,
        response::IntoResponse,
        Json,
    };
    use chrono::Utc;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    #[test]
    fn normalized_owner_intent_accepts_known_modes_only() {
        assert_eq!(normalized_owner_intent(Some("plan")), Some("plan"));
        assert_eq!(normalized_owner_intent(Some(" ask ")), Some("ask"));
        assert_eq!(normalized_owner_intent(Some("agent")), Some("agent"));
        assert_eq!(normalized_owner_intent(Some("execute")), Some("ask"));
        assert_eq!(normalized_owner_intent(None), None);
    }

    #[tokio::test]
    async fn runtime_chat_resolves_typed_attachments_before_admission() {
        let (api_tx, _api_rx) = mpsc::channel::<ManagerCommand>(1);
        let bus = AgentEventBus::new();
        let temp_dir = tempfile::tempdir().unwrap();
        let state = AppState::new(api_tx, bus, temp_dir.path()).unwrap();
        let files = Arc::new(
            FileManager::new(FileConfig::with_path(temp_dir.path().join("files")))
                .await
                .unwrap(),
        );
        let handle = files
            .store(
                &[0x89, b'P', b'N', b'G'],
                FileMetadata {
                    name: "diagram.png".to_string(),
                    size: 4,
                    mime_type: Some("image/png".to_string()),
                    source: Some("api".to_string()),
                    created_at: Utc::now(),
                    last_accessed_at: None,
                    preview: None,
                },
            )
            .await
            .unwrap();
        let state = state.with_attachment_authority(files);
        let payload = ChatRequest {
            message: "describe this".to_string(),
            request_id: Some("request-1".to_string()),
            channel: Some("api".to_string()),
            chat_id: Some("chat-1".to_string()),
            attachments: Some(vec![handle.id.clone()]),
            mode: Some("plan".to_string()),
            execution_start: None,
            plan_id: None,
            plan_revision: None,
            execution_id: None,
            approval_policy: None,
        };
        let envelope = build_typed_chat_envelope(
            &state,
            "api".to_string(),
            "chat-1".to_string(),
            "request-1".to_string(),
            payload,
        )
        .await
        .unwrap();

        assert_eq!(envelope.correlation.session_key, "api:chat-1");
        assert_eq!(
            envelope.correlation.request_id.as_deref(),
            Some("request-1")
        );
        assert_eq!(
            envelope.origin,
            agent_diva_core::channel::ChannelOrigin::OwnerFrontend
        );
        match envelope.payload {
            agent_diva_core::channel::ChannelPayloadV1::Message {
                parts,
                context: Some(context),
                ..
            } => {
                assert_eq!(
                    context.intent,
                    agent_diva_core::channel::OwnerTurnIntent::Plan
                );
                assert!(matches!(
                    parts.as_slice(),
                    [
                        agent_diva_core::channel::ContentPart::Text { text },
                        agent_diva_core::channel::ContentPart::Image { attachment }
                    ] if text == "describe this"
                        && attachment.media_type == "image/png"
                        && attachment.file_name.as_deref() == Some("diagram.png")
                        && attachment.uri == handle.id
                        && attachment.sha256 == handle.id.strip_prefix("sha256:").unwrap()
                ));
            }
            payload => panic!("unexpected typed HTTP payload: {payload:?}"),
        }

        let invalid = build_typed_chat_envelope(
            &state,
            "api".to_string(),
            "chat-1".to_string(),
            "request-2".to_string(),
            ChatRequest {
                message: "bad attachment".to_string(),
                request_id: Some("request-2".to_string()),
                channel: Some("api".to_string()),
                chat_id: Some("chat-1".to_string()),
                attachments: Some(vec!["sha256:not-found".to_string()]),
                mode: None,
                execution_start: None,
                plan_id: None,
                plan_revision: None,
                execution_id: None,
                approval_policy: None,
            },
        )
        .await;
        assert!(invalid
            .expect_err("unknown attachment must fail before admission")
            .contains("invalid attachment reference"));
    }

    #[test]
    fn attachment_digest_requires_sha256_authority_id() {
        assert_eq!(attachment_digest("sha256:abc").unwrap(), "abc");
        assert!(attachment_digest("abc").is_err());
        assert!(attachment_digest("sha256:").is_err());
    }

    #[test]
    fn busy_probe_error_is_rate_limited_without_adapter_diagnostics() {
        let error = agent_diva_channels::ChannelProbeError::Adapter {
            code: "probe_busy".to_string(),
            retry_after_ms: Some(250),
            retryable: true,
        };
        let (status, Json(body)) = channel_probe_error_response(error);

        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(body["code"], "probe_busy");
        assert_eq!(body["message"], "channel probe is already in progress");
        assert_eq!(body["retryable"], true);
        assert_eq!(body["retry_after_ms"], 250);
        assert!(!body.to_string().contains("smtp"));
        assert!(!body.to_string().contains("secret"));
    }

    #[tokio::test]
    async fn get_sessions_response_includes_title() {
        let (api_tx, mut api_rx) = mpsc::channel::<ManagerCommand>(10);
        let bus = AgentEventBus::new();
        let temp_dir = tempfile::tempdir().unwrap();
        let state = AppState::new(api_tx, bus, temp_dir.path()).unwrap();

        tokio::spawn(async move {
            while let Some(cmd) = api_rx.recv().await {
                if let ManagerCommand::GetSessions(tx) = cmd {
                    let sessions = vec![
                        SessionInfo {
                            key: "gui:with-title".to_string(),
                            created_at: None,
                            updated_at: None,
                            path: "/tmp/a.json".to_string(),
                            title: Some("Test Session".to_string()),
                            last_message: Some("Latest reply".to_string()),
                            message_count: 2,
                            title_generated: true,
                            title_manually_set: false,
                            pinned: false,
                            workspace_id: "workspace-test".to_string(),
                            channel: "gui".to_string(),
                            kind: SessionKind::Root,
                            root_session_key: Some("gui:with-title".to_string()),
                            parent_session_key: None,
                            branch_label: None,
                            legacy: false,
                        },
                        SessionInfo {
                            key: "gui:no-title".to_string(),
                            created_at: None,
                            updated_at: None,
                            path: "/tmp/b.json".to_string(),
                            title: None,
                            last_message: None,
                            message_count: 0,
                            title_generated: false,
                            title_manually_set: false,
                            pinned: false,
                            workspace_id: "workspace-test".to_string(),
                            channel: "gui".to_string(),
                            kind: SessionKind::Root,
                            root_session_key: Some("gui:no-title".to_string()),
                            parent_session_key: None,
                            branch_label: None,
                            legacy: false,
                        },
                    ];
                    let _ = tx.send(Ok(sessions));
                }
            }
        });

        let axum::Json(response) = get_sessions_handler(State(state)).await;
        assert_eq!(response["status"], "ok");
        let sessions = response["sessions"].as_array().unwrap();
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0]["title"], "Test Session");
        assert_eq!(sessions[0]["last_message"], "Latest reply");
        assert_eq!(sessions[0]["message_count"], 2);
        assert_eq!(sessions[0]["title_generated"], true);
        assert!(sessions[1]["title"].is_null());
    }

    #[tokio::test]
    async fn get_session_history_response_includes_title() {
        let (api_tx, mut api_rx) = mpsc::channel::<ManagerCommand>(10);
        let bus = AgentEventBus::new();
        let temp_dir = tempfile::tempdir().unwrap();
        let state = AppState::new(api_tx, bus, temp_dir.path()).unwrap();

        tokio::spawn(async move {
            while let Some(cmd) = api_rx.recv().await {
                if let ManagerCommand::GetSessionHistory(key, tx) = cmd {
                    assert_eq!(key, "gui:test-id");
                    let session = Session {
                        key: key.clone(),
                        messages: vec![ChatMessage::new("user", "Hello")],
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                        metadata: serde_json::Value::Object(serde_json::Map::new()),
                        title: Some("History Title".to_string()),
                        last_consolidated: 0,
                        canonical_checkpoint: None,
                    };
                    let _ = tx.send(Ok(Some(session)));
                }
            }
        });

        let axum::Json(response) =
            get_session_history_handler(State(state), Path("test-id".to_string())).await;
        assert_eq!(response["status"], "ok");
        assert_eq!(response["session"]["title"], "History Title");
    }

    #[tokio::test]
    async fn generate_session_title_response_includes_flags() {
        let (api_tx, mut api_rx) = mpsc::channel::<ManagerCommand>(10);
        let bus = AgentEventBus::new();
        let temp_dir = tempfile::tempdir().unwrap();
        let state = AppState::new(api_tx, bus, temp_dir.path()).unwrap();

        tokio::spawn(async move {
            while let Some(cmd) = api_rx.recv().await {
                if let ManagerCommand::GenerateSessionTitle(key, _payload, tx) = cmd {
                    assert_eq!(key, "gui:test-id");
                    let _ = tx.send(Ok(GenerateSessionTitleResponse {
                        title: "Generated Title".to_string(),
                        title_generated: true,
                        title_manually_set: false,
                    }));
                }
            }
        });

        let axum::Json(response) = generate_session_title_handler(
            State(state),
            Path("test-id".to_string()),
            Json(crate::state::GenerateSessionTitleRequest {
                first_user_message: "hello".to_string(),
                first_assistant_message: "world".to_string(),
            }),
        )
        .await;
        assert_eq!(response["status"], "ok");
        assert_eq!(response["title"], "Generated Title");
        assert_eq!(response["title_generated"], true);
        assert_eq!(response["title_manually_set"], false);
    }

    #[tokio::test]
    async fn update_session_title_returns_title_field() {
        let (api_tx, mut api_rx) = mpsc::channel::<ManagerCommand>(10);
        let bus = AgentEventBus::new();
        let temp_dir = tempfile::tempdir().unwrap();
        let state = AppState::new(api_tx, bus, temp_dir.path()).unwrap();

        tokio::spawn(async move {
            while let Some(cmd) = api_rx.recv().await {
                if let ManagerCommand::UpdateSessionTitle(key, title, tx) = cmd {
                    assert_eq!(key, "gui:test-id");
                    assert_eq!(title.as_deref(), Some("Manual Title"));
                    let _ = tx.send(Ok(title));
                }
            }
        });

        let axum::Json(response) = update_session_title_handler(
            State(state),
            Path("test-id".to_string()),
            Json(serde_json::json!({ "title": "Manual Title" })),
        )
        .await;
        assert_eq!(response["status"], "ok");
        assert_eq!(response["title"], "Manual Title");
    }

    #[tokio::test]
    async fn chat_plan_update_forward_serializes_via_chat_sse() {
        use agent_diva_core::planning::update_plan::{PlanItem, PlanItemStatus, UpdatePlanArgs};

        let (api_tx, mut api_rx) = mpsc::channel::<ManagerCommand>(1);
        let bus = AgentEventBus::new();
        let temp_dir = tempfile::tempdir().unwrap();
        agent_diva_laputa::PersonaService::open(temp_dir.path())
            .unwrap()
            .initialize(agent_diva_laputa::PersonaInitialization {
                identity: "Diva".into(),
                relationship: "Test partner".into(),
                redline: "No unsafe actions".into(),
                user: "Concise output".into(),
                world: "Local handler test".into(),
            })
            .unwrap();
        let state = AppState::new(api_tx, bus, temp_dir.path()).unwrap();

        tokio::spawn(async move {
            if let Some(ManagerCommand::Chat(req)) = api_rx.recv().await {
                let _ = req.event_tx.send(AgentEvent::ChatPlanUpdate {
                    args: UpdatePlanArgs {
                        explanation: Some("test plan".to_string()),
                        plan: vec![PlanItem {
                            step: "step 1".to_string(),
                            status: PlanItemStatus::Completed,
                        }],
                    },
                });
                let _ = req.event_tx.send(AgentEvent::FinalResponse {
                    content: "done".to_string(),
                });
            }
        });

        let sse = chat_handler(
            State(state),
            Json(ChatRequest {
                message: "hello".to_string(),
                request_id: None,
                channel: None,
                chat_id: None,
                attachments: None,
                mode: None,
                execution_start: None,
                plan_id: None,
                plan_revision: None,
                execution_id: None,
                approval_policy: None,
            }),
        )
        .await;
        let response = sse.into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();

        assert!(
            text.contains("event: turn_plan_updated"),
            "SSE should emit turn_plan_updated event; got: {text}"
        );
        assert!(
            text.contains("test plan"),
            "serialized explanation missing; got: {text}"
        );
        assert!(
            text.contains("step 1"),
            "serialized plan step missing; got: {text}"
        );
        assert!(
            text.contains("completed"),
            "serialized status missing; got: {text}"
        );
    }

    #[tokio::test]
    async fn chat_plan_update_forward_via_events_bus_event() {
        use agent_diva_core::bus::AgentBusEvent;
        use agent_diva_core::planning::update_plan::{PlanItem, PlanItemStatus, UpdatePlanArgs};
        use std::convert::Infallible;

        let args = UpdatePlanArgs {
            explanation: Some("via bus".to_string()),
            plan: vec![PlanItem {
                step: "bus step".to_string(),
                status: PlanItemStatus::InProgress,
            }],
        };
        let bus_event = AgentBusEvent {
            channel: "gui".to_string(),
            chat_id: "chat-7".to_string(),
            session_key: None,
            request_id: None,
            trace_id: None,
            event: AgentEvent::ChatPlanUpdate { args: args.clone() },
        };

        let event =
            agent_bus_event_to_sse(&bus_event).expect("expected SSE event for ChatPlanUpdate");
        let stream = futures::stream::once(async move { Ok::<_, Infallible>(event) });
        let sse = Sse::new(stream);
        let response = sse.into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();

        assert!(
            text.contains("event: turn_plan_updated"),
            "SSE should emit turn_plan_updated event; got: {text}"
        );
        assert!(text.contains("gui"), "channel missing; got: {text}");
        assert!(text.contains("chat-7"), "chat_id missing; got: {text}");
        assert!(text.contains("via bus"), "explanation missing; got: {text}");
        assert!(text.contains("bus step"), "step missing; got: {text}");
        assert!(text.contains("in_progress"), "status missing; got: {text}");
    }

    #[tokio::test]
    async fn g0_turn_plan_sse_contract_matches_fixture() {
        use agent_diva_core::bus::AgentBusEvent;
        use agent_diva_core::planning::update_plan::{PlanItem, PlanItemStatus, UpdatePlanArgs};
        use std::convert::Infallible;

        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/g0_runtime_contract.json"))
                .unwrap();
        let expected = &fixture["turn_plan_updated"];
        let bus_event = AgentBusEvent {
            channel: expected["channel"].as_str().unwrap().to_string(),
            chat_id: expected["chat_id"].as_str().unwrap().to_string(),
            session_key: None,
            request_id: None,
            trace_id: None,
            event: AgentEvent::ChatPlanUpdate {
                args: UpdatePlanArgs {
                    explanation: Some(
                        expected["args"]["explanation"]
                            .as_str()
                            .unwrap()
                            .to_string(),
                    ),
                    plan: vec![PlanItem {
                        step: expected["args"]["plan"][0]["step"]
                            .as_str()
                            .unwrap()
                            .to_string(),
                        status: PlanItemStatus::InProgress,
                    }],
                },
            },
        };

        let event = agent_bus_event_to_sse(&bus_event).expect("turn plan event");
        let response = Sse::new(futures::stream::once(
            async move { Ok::<_, Infallible>(event) },
        ))
        .into_response();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();

        assert!(text.contains(&format!("event: {}", expected["event"].as_str().unwrap())));
        assert!(text.contains(expected["channel"].as_str().unwrap()));
        assert!(text.contains(expected["chat_id"].as_str().unwrap()));
        assert!(text.contains(expected["args"]["explanation"].as_str().unwrap()));
        assert!(text.contains(expected["args"]["plan"][0]["step"].as_str().unwrap()));
        assert!(text.contains(expected["args"]["plan"][0]["status"].as_str().unwrap()));
    }

    #[test]
    fn parse_approval_policy_maps_gui_modes_and_canonical_aliases() {
        assert_eq!(
            parse_approval_policy(Some("cautious")),
            Some(agent_diva_sandbox::AskForApproval::OnRequest)
        );
        assert_eq!(
            parse_approval_policy(Some("smart")),
            Some(agent_diva_sandbox::AskForApproval::OnFailure)
        );
        assert_eq!(
            parse_approval_policy(Some("trusted")),
            Some(agent_diva_sandbox::AskForApproval::UnlessTrusted)
        );
        assert_eq!(
            parse_approval_policy(Some("on-request")),
            Some(agent_diva_sandbox::AskForApproval::OnRequest)
        );
        assert_eq!(
            parse_approval_policy(Some("On_Failure")),
            Some(agent_diva_sandbox::AskForApproval::OnFailure)
        );
        assert_eq!(
            parse_approval_policy(Some("never")),
            Some(agent_diva_sandbox::AskForApproval::Never)
        );
        assert_eq!(parse_approval_policy(None), None);
        assert_eq!(parse_approval_policy(Some("")), None);
        assert_eq!(parse_approval_policy(Some("garbage")), None);
    }
}
