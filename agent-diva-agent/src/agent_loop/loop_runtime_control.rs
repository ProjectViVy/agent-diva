use super::AgentLoop;
use crate::compaction::{CheckpointCompactor, CheckpointSnapshot};
use crate::runtime_control::RuntimeControlCommand;
use agent_diva_core::bus::{AgentEvent, PlanRuntimeState};
use agent_diva_core::bus::{
    SessionAdmissionCode, SessionControlAction, SessionControlOutcome, SessionControlTargetState,
};
use agent_diva_core::channel::ChannelEnvelopeV1;
use agent_diva_core::memory::{SessionEndRequest, SystemPromptRefreshRequest};
use agent_diva_core::session::CheckpointTrigger;
use agent_diva_providers::Message;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::{mpsc, oneshot};
use tracing::{info, warn};

impl AgentLoop {
    pub(super) async fn finish_pending_session_cleanups(&mut self) {
        for cleanup in std::mem::take(&mut self.pending_session_cleanups) {
            match cleanup {
                super::PendingSessionCleanup::Reset {
                    session_key,
                    running_cancelled,
                    queued_cancelled,
                    reply_tx,
                } => {
                    self.finish_reset_session(
                        session_key,
                        running_cancelled,
                        queued_cancelled,
                        reply_tx,
                    )
                    .await;
                }
                super::PendingSessionCleanup::Delete {
                    session_key,
                    reply_tx,
                } => {
                    let result = self.finish_delete_session(&session_key).await;
                    let _ = reply_tx.send(result);
                }
            }
        }
    }

    /// Mark the cached machine Skill section for every Session.
    /// Callers use this after an already-committed skill mutation; the next
    /// prompt assembly performs the actual disk read.
    pub(crate) fn mark_machine_skills_for_reload(&self, change_id: &str) -> usize {
        let invalidated = self.context.invalidate_skills_for_all_sessions();
        tracing::info!(
            change_id,
            invalidated_sessions = invalidated,
            "machine skills marked for lazy reload"
        );
        invalidated
    }

    pub(super) async fn finish_reset_session(
        &mut self,
        session_key: String,
        running_cancelled: bool,
        queued_cancelled: usize,
        reply_tx: tokio::sync::oneshot::Sender<SessionControlOutcome>,
    ) {
        self.cancel_actmem_session(&session_key).await;
        if let Err(error) = self
            .memory_provider
            .on_session_end(SessionEndRequest {
                workspace_root: self.workspace.clone(),
                session_id: Some(session_key.clone()),
            })
            .await
        {
            warn!(
                session_id = %session_key,
                error = %error,
                "session checkpoint cleanup failed during reset"
            );
        }
        agent_diva_laputa::release_frozen_core_session(&self.persona_root, &session_key);
        self.context.reset_session_cache(&session_key);
        self.clear_active_deferred_tools(&session_key);
        if let Some(planning) = self.tool_config.planning.as_ref() {
            planning.registry.discard_session(&session_key).await;
        }
        if let Err(error) = self.sessions.archive_and_reset(&session_key) {
            tracing::error!(%error, "failed to archive and reset session");
        } else {
            info!(session_key = %session_key, "archived and reset session");
        }
        let _ = reply_tx.send(SessionControlOutcome {
            action: SessionControlAction::Reset,
            session_key,
            request_id: None,
            trace_id: None,
            code: Some(SessionAdmissionCode::SessionReset),
            target_state: SessionControlTargetState::Session,
            running_cancelled,
            queued_cancelled,
            cleanup_complete: true,
        });
    }

    pub(super) async fn finish_delete_session(
        &mut self,
        session_key: &str,
    ) -> Result<bool, String> {
        let result = self
            .sessions
            .delete(session_key)
            .map_err(|error| error.to_string());
        if result.is_ok() {
            self.cancel_actmem_session(session_key).await;
            if let Err(error) = self
                .memory_provider
                .on_session_end(SessionEndRequest {
                    workspace_root: self.workspace.clone(),
                    session_id: Some(session_key.to_string()),
                })
                .await
            {
                warn!(
                    session_id = %session_key,
                    error = %error,
                    "session checkpoint cleanup failed during delete"
                );
            }
            let store = agent_diva_core::tool_artifact::ToolArtifactStore::new(&self.workspace);
            let artifact_context = agent_diva_core::tool_artifact::ToolArtifactSecurityContext::new(
                &self.workspace,
                session_key,
            );
            match tokio::task::spawn_blocking(move || store.delete_session(&artifact_context)).await
            {
                Ok(Ok(())) => {}
                Ok(Err(error)) => warn!(
                    error_code = error.code(),
                    "failed to delete session tool artifacts"
                ),
                Err(error) => warn!(%error, "tool artifact deletion task failed"),
            }
            agent_diva_laputa::release_frozen_core_session(&self.persona_root, session_key);
            self.context.end_session_cache(session_key);
            self.cache_observer.clear_session(session_key);
            self.clear_active_deferred_tools(session_key);
            if let Some(planning) = self.tool_config.planning.as_ref() {
                planning.registry.discard_session(session_key).await;
            }
        }
        match &result {
            Ok(deleted) => info!(
                session_key,
                deleted = *deleted,
                "runtime delete session completed"
            ),
            Err(error) => tracing::error!(session_key, %error, "runtime delete session failed"),
        }
        result
    }

    pub(super) async fn handle_runtime_control_command(&mut self, cmd: RuntimeControlCommand) {
        match cmd {
            RuntimeControlCommand::StartChannelTurn { envelope, reply_tx } => {
                self.spawn_typed_channel_turn(*envelope, reply_tx);
            }
            RuntimeControlCommand::ReloadMachineSkills { change_id } => {
                self.mark_machine_skills_for_reload(&change_id);
            }
            RuntimeControlCommand::RefreshMemoryAuthority {
                workspace_id,
                authority_revision,
                change_id,
            } => {
                let expected_workspace_id =
                    agent_diva_core::workspace_identity::canonical_workspace_id(&self.workspace);
                if workspace_id != expected_workspace_id {
                    tracing::warn!(
                        expected_workspace_id,
                        received_workspace_id = %workspace_id,
                        authority_revision,
                        change_id = %change_id,
                        "ignored Memory authority refresh for another workspace"
                    );
                    return;
                }
                let request = SystemPromptRefreshRequest {
                    workspace_root: self.workspace.clone(),
                    authority_revision,
                };
                match self
                    .memory_provider
                    .refresh_system_prompt_projection(request)
                    .await
                {
                    Ok(result) => tracing::info!(
                        authority_revision = result.authority_revision,
                        projection_changed = result.projection_changed,
                        change_id = %change_id,
                        "Memory authority projection refresh applied"
                    ),
                    Err(error) => tracing::error!(
                        authority_revision,
                        change_id = %change_id,
                        error = %error,
                        "Memory authority projection refresh failed"
                    ),
                }
            }
            RuntimeControlCommand::UpdateNetwork(network) => {
                self.apply_network_config(network).await;
                self.rebuild_tools_for_active_phase().await;
            }
            RuntimeControlCommand::UpdateMcp { servers } => {
                self.apply_mcp_config(servers).await;
                self.rebuild_tools_for_active_phase().await;
            }
            RuntimeControlCommand::StopSession {
                session_key,
                request_id,
                reply_tx,
            } => {
                let target = self
                    .session_dispatcher
                    .stop_request(&session_key, request_id.as_deref());
                let (target_state, identity, running_cancelled, code) = match target {
                    super::dispatcher::SessionStopTarget::Running(identity) => {
                        self.cancelled_sessions.insert(session_key.clone());
                        (
                            SessionControlTargetState::Running,
                            Some(identity),
                            true,
                            Some(SessionAdmissionCode::SessionTurnCancelled),
                        )
                    }
                    super::dispatcher::SessionStopTarget::QueuedPreserved(identity) => (
                        SessionControlTargetState::QueuedPreserved,
                        Some(identity),
                        false,
                        None,
                    ),
                    super::dispatcher::SessionStopTarget::Absent => {
                        (SessionControlTargetState::Absent, None, false, None)
                    }
                };
                let _ = reply_tx.send(SessionControlOutcome {
                    action: SessionControlAction::Stop,
                    session_key,
                    request_id: identity
                        .as_ref()
                        .map(|identity| identity.request_id.clone()),
                    trace_id: identity.map(|identity| identity.trace_id),
                    code,
                    target_state,
                    running_cancelled,
                    queued_cancelled: 0,
                    cleanup_complete: true,
                });
            }
            RuntimeControlCommand::ResetSession {
                session_key,
                reply_tx,
            } => {
                let running_cancelled = self.session_dispatcher.stop_running(&session_key);
                let queued_cancelled = self.session_dispatcher.reset_session(&session_key);
                if !self.actor_dispatch_enabled {
                    if self.active_turn_cancellation.is_some() {
                        self.pending_session_cleanups
                            .push(super::PendingSessionCleanup::Reset {
                                session_key,
                                running_cancelled,
                                queued_cancelled,
                                reply_tx,
                            });
                        return;
                    }
                    self.finish_reset_session(
                        session_key,
                        running_cancelled,
                        queued_cancelled,
                        reply_tx,
                    )
                    .await;
                    return;
                }
                let worker_tx = self.session_worker_sender(&session_key);
                if let Err(error) = worker_tx.send(super::SessionWorkerCommand::Reset {
                    session_key,
                    running_cancelled,
                    queued_cancelled,
                    reply_tx,
                }) {
                    if let super::SessionWorkerCommand::Reset {
                        session_key,
                        running_cancelled,
                        queued_cancelled,
                        reply_tx,
                    } = error.0
                    {
                        let _ = reply_tx.send(SessionControlOutcome {
                            action: SessionControlAction::Reset,
                            session_key,
                            request_id: None,
                            trace_id: None,
                            code: Some(SessionAdmissionCode::SessionWorkerUnavailable),
                            target_state: SessionControlTargetState::Session,
                            running_cancelled,
                            queued_cancelled,
                            cleanup_complete: false,
                        });
                    }
                }
            }
            RuntimeControlCommand::GetSessions { reply_tx } => {
                let sessions = self.sessions.list_sessions();
                let _ = reply_tx.send(sessions);
            }
            RuntimeControlCommand::GetSession {
                session_key,
                reply_tx,
            } => {
                let session = self.sessions.get_or_load(&session_key).cloned();
                let _ = reply_tx.send(session);
            }
            RuntimeControlCommand::DeleteSession {
                session_key,
                reply_tx,
            } => {
                self.session_dispatcher.reset_session(&session_key);
                if !self.actor_dispatch_enabled {
                    if self.active_turn_cancellation.is_some() {
                        self.pending_session_cleanups
                            .push(super::PendingSessionCleanup::Delete {
                                session_key,
                                reply_tx,
                            });
                        return;
                    }
                    let result = self.finish_delete_session(&session_key).await;
                    let _ = reply_tx.send(result);
                    return;
                }
                let worker_tx = self.session_worker_sender(&session_key);
                if let Err(error) = worker_tx.send(super::SessionWorkerCommand::Delete {
                    session_key,
                    reply_tx,
                }) {
                    if let super::SessionWorkerCommand::Delete { reply_tx, .. } = error.0 {
                        let _ = reply_tx.send(Err("session worker unavailable".to_string()));
                    }
                }
            }
            RuntimeControlCommand::UpdateSessionTitle {
                session_key,
                title,
                reply_tx,
            } => {
                let result = self.handle_update_session_title(&session_key, title).await;
                let _ = reply_tx.send(result);
            }
            RuntimeControlCommand::GenerateSessionTitle {
                session_key,
                first_user_message,
                first_assistant_message,
                fallback_title,
                reply_tx,
            } => {
                let result = self
                    .handle_generate_session_title(
                        &session_key,
                        &first_user_message,
                        &first_assistant_message,
                        &fallback_title,
                    )
                    .await;
                let _ = reply_tx.send(result);
            }
            RuntimeControlCommand::SetThinking { mode } => {
                self.thinking_mode = mode;
                info!("Thinking mode set to: {:?}", mode);
            }
            RuntimeControlCommand::SetApprovalPolicy { policy } => {
                self.set_approval_policy(policy);
                info!("Approval policy set to: {:?}", policy);
            }
            RuntimeControlCommand::CompactSession {
                session_key,
                reply_tx,
            } => {
                let result = self.handle_compact_session(&session_key).await;
                let _ = reply_tx.send(result);
            }
            RuntimeControlCommand::ApproveActivePlan { reply_tx, .. } => {
                let _ = reply_tx.send(Err(
                    "legacy global plan approval has been removed".to_string()
                ));
            }
            RuntimeControlCommand::ReturnActivePlanToDraft { reply_tx } => {
                let _ = reply_tx.send(Err(
                    "legacy global plan drafts have been removed".to_string()
                ));
            }
        }
    }

    pub(super) async fn drain_runtime_control_commands(&mut self) {
        while let Some(rx) = self.runtime_control_rx.as_mut() {
            let cmd = match rx.try_recv() {
                Ok(cmd) => cmd,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    info!("Runtime control channel closed");
                    self.runtime_control_rx = None;
                    break;
                }
            };

            self.handle_runtime_control_command(cmd).await;
        }
    }

    pub(super) fn is_session_cancelled(&self, session_key: &str) -> bool {
        self.cancelled_sessions.contains(session_key)
            || self
                .active_turn_cancellation
                .as_ref()
                .is_some_and(tokio_util::sync::CancellationToken::is_cancelled)
    }

    pub(super) fn clear_session_cancellation(&mut self, session_key: &str) {
        self.cancelled_sessions.remove(session_key);
    }

    pub(super) fn emit_error_event(
        &self,
        envelope: &ChannelEnvelopeV1,
        event_tx: Option<&mpsc::UnboundedSender<AgentEvent>>,
        message: impl Into<String>,
    ) {
        let event = AgentEvent::Error {
            message: message.into(),
        };
        if let Some(tx) = event_tx {
            let _ = tx.send(event.clone());
        }
        super::publish_envelope_event(&self.bus, envelope, event);
    }

    /// Handle manual `/compact` command: run compaction on the session and
    /// return a human-readable summary string.
    pub(super) async fn handle_compact_session(
        &mut self,
        session_key: &str,
    ) -> Result<String, String> {
        // Check session exists and has messages
        {
            let session = self
                .sessions
                .get(session_key)
                .ok_or_else(|| "session not found".to_string())?;
            let history = session.get_history(usize::MAX);
            if history.is_empty() {
                return Err("session has no messages to compact".to_string());
            }
        };

        let budget_config = self.tool_config.budget.clone();
        let provider = self.provider.clone();
        let model = self.model.clone();

        // Run compaction with Manual trigger (immutable borrow, like auto-compaction)
        let compact_result = {
            if let Some(session) = self.sessions.get(session_key) {
                CheckpointCompactor::compact_snapshot(
                    CheckpointSnapshot::from_session(session, Vec::new()),
                    &budget_config,
                    provider,
                    &model,
                    CheckpointTrigger::Manual,
                )
                .await
            } else {
                return Err("session disappeared during compaction".to_string());
            }
        };

        match compact_result {
            Ok(None) => Ok("nothing to compact — session is already lean".to_string()),
            Ok(Some(result)) => {
                let source_message_count = result.checkpoint.source_message_count;
                let source_token_count = result.checkpoint.source_token_count;
                let body = result.checkpoint.body.clone();

                {
                    let session = self.sessions.get_or_create(session_key);
                    session.canonical_checkpoint = Some(result.checkpoint.clone());
                }
                if let Some(s) = self.sessions.get(session_key) {
                    if let Err(e) = self.sessions.save(s) {
                        warn!("Failed to persist compaction state: {}", e);
                    }
                }

                info!(
                    "Manual checkpoint complete: {} msgs → {} chars",
                    source_message_count,
                    body.len()
                );

                Ok(format!(
                    "compact done — {} messages compressed, ~{} tokens saved\ncheckpoint: {}",
                    source_message_count, source_token_count, body
                ))
            }
            Err(e) => Err(format!("compaction failed: {}", e)),
        }
    }

    async fn handle_update_session_title(
        &mut self,
        session_key: &str,
        title: Option<String>,
    ) -> Result<Option<String>, String> {
        let clean_title = title
            .as_ref()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty());
        let session_key = session_key.to_string();

        let exists = self.sessions.get_or_load(&session_key).is_some();
        if !exists {
            return Err("session not found".to_string());
        }

        let session_to_save = {
            let session = self.sessions.get_or_create(session_key.clone());
            session.set_conversation_title(clean_title.clone());
            session.set_title_generated(false);
            session.set_title_manually_set(clean_title.is_some());
            session.clone()
        };

        self.sessions
            .save(&session_to_save)
            .map_err(|e| e.to_string())?;

        Ok(clean_title)
    }

    async fn handle_generate_session_title(
        &mut self,
        session_key: &str,
        first_user_message: &str,
        first_assistant_message: &str,
        fallback_title: &str,
    ) -> Result<(String, bool, bool), String> {
        let Some(existing) = self.sessions.get_or_load(session_key).cloned() else {
            return Err("session not found".to_string());
        };
        if existing.title_manually_set() {
            let title = existing
                .conversation_title()
                .unwrap_or_else(|| fallback_title.to_string());
            return Ok((title, existing.title_generated(), true));
        }
        if existing.title_generated() {
            let title = existing
                .conversation_title()
                .unwrap_or_else(|| fallback_title.to_string());
            return Ok((title, true, existing.title_manually_set()));
        }

        let generated = self
            .provider
            .chat(
                vec![
                    Message::system(
                        "You write short chat titles. Output only a short title with no quotes, no markdown, and no explanation.",
                    ),
                    Message::user(format!(
                        "Generate a concise conversation title under 12 words.\n\nFirst user message:\n{}\n\nFirst assistant message:\n{}",
                        first_user_message.trim(),
                        first_assistant_message.trim()
                    )),
                ],
                None,
                agent_diva_providers::ToolChoiceMode::Unspecified,
                Some(self.provider.get_default_model()),
                64,
                0.2,
            )
            .await
            .ok()
            .and_then(|response| {
                response
                    .content
                    .as_deref()
                    .and_then(|content| content.lines().next())
                    .map(str::trim)
                    .map(|title| title.trim_matches('"').trim())
                    .filter(|title| !title.is_empty())
                    .map(|title| title.chars().take(60).collect::<String>())
            });
        let title = generated
            .clone()
            .unwrap_or_else(|| fallback_title.trim().to_string());
        let session_to_save = {
            let session = self.sessions.get_or_create(session_key.to_string());
            session.set_conversation_title(Some(title.clone()));
            session.set_title_generated(generated.is_some());
            session.set_title_manually_set(false);
            session.clone()
        };

        self.sessions
            .save(&session_to_save)
            .map_err(|e| e.to_string())?;

        Ok((title, generated.is_some(), false))
    }

    /// Re-assemble after runtime mutations so registration remains a phase
    /// boundary even before a subsequent tool call can re-snapshot state.
    pub(crate) async fn rebuild_tools_for_active_phase(&mut self) {
        let surface = self.active_tool_surface.clone();
        let active_mask = self.load_active_mask();
        self.rebuild_tools_for_turn(
            active_mask.as_ref(),
            surface.plan_phase,
            surface.execution_session_id,
            surface.session_key,
            surface.background_task_context,
        );
    }

    pub(super) async fn snapshot_active_plan_runtime(
        &self,
        session_key: &str,
    ) -> Option<PlanRuntimeState> {
        let planning = self.tool_config.planning.as_ref()?;
        planning
            .registry
            .runtime_state_for_session(session_key)
            .await
    }

    /// Schedule a typed Fabric envelope without making the AgentLoop's root
    /// select loop wait for the eventual provider response. The caller gets
    /// the first queued/running admission transition; the existing per-session
    /// worker owns the long-running turn and its cancellation fence.
    fn spawn_typed_channel_turn(
        &self,
        envelope: ChannelEnvelopeV1,
        reply_tx: oneshot::Sender<
            Result<agent_diva_core::bus::SessionAdmissionObservation, String>,
        >,
    ) {
        let session_key = envelope.correlation.session_key.clone();
        let worker_tx = self.session_worker_sender(&session_key);
        tokio::spawn(dispatch_typed_channel_turn(
            self.session_dispatcher.clone(),
            self.bus.clone(),
            worker_tx,
            self.egress_tx.clone(),
            envelope,
            reply_tx,
        ));
    }
}

async fn dispatch_typed_channel_turn(
    dispatcher: super::dispatcher::SessionDispatcher,
    bus: agent_diva_core::bus::AgentEventBus,
    worker_tx: mpsc::UnboundedSender<super::SessionWorkerCommand>,
    egress_tx: Option<mpsc::Sender<agent_diva_core::channel::ChannelCommand>>,
    envelope: ChannelEnvelopeV1,
    reply_tx: oneshot::Sender<Result<agent_diva_core::bus::SessionAdmissionObservation, String>>,
) {
    let (envelope, identity) = match super::prepare_turn_envelope(envelope) {
        Ok(value) => value,
        Err(error) => {
            let _ = reply_tx.send(Err(error));
            return;
        }
    };
    let session_key = envelope.correlation.session_key.clone();
    let observer = super::admission_observer(bus.clone(), envelope.clone(), None, identity.clone());
    let reply_slot = std::sync::Arc::new(std::sync::Mutex::new(Some(reply_tx)));
    let callback = {
        let reply_slot = std::sync::Arc::clone(&reply_slot);
        let callback_envelope = envelope.clone();
        let callback_identity = identity.clone();
        let observer = observer.clone();
        std::sync::Arc::new(
            move |transition: super::dispatcher::SessionDispatchTransition| {
                let admission = match &transition {
                    super::dispatcher::SessionDispatchTransition::Queued { queue_depth } => {
                        Some(agent_diva_core::bus::SessionAdmissionObservation {
                            code: None,
                            phase: agent_diva_core::bus::SessionAdmissionPhase::Queued,
                            session_key: callback_envelope.correlation.session_key.clone(),
                            request_id: callback_identity.request_id.clone(),
                            trace_id: callback_identity.trace_id.clone(),
                            queue_depth: *queue_depth,
                            wait_latency_ms: 0,
                        })
                    }
                    super::dispatcher::SessionDispatchTransition::Running {
                        queue_depth,
                        wait_latency,
                    } => Some(agent_diva_core::bus::SessionAdmissionObservation {
                        code: None,
                        phase: agent_diva_core::bus::SessionAdmissionPhase::Running,
                        session_key: callback_envelope.correlation.session_key.clone(),
                        request_id: callback_identity.request_id.clone(),
                        trace_id: callback_identity.trace_id.clone(),
                        queue_depth: *queue_depth,
                        wait_latency_ms: wait_latency.as_millis().try_into().unwrap_or(u64::MAX),
                    }),
                    super::dispatcher::SessionDispatchTransition::Rejected(_) => None,
                };
                observer(transition);
                if let Some(admission) = admission {
                    if let Some(reply_tx) = reply_slot
                        .lock()
                        .unwrap_or_else(|error| error.into_inner())
                        .take()
                    {
                        let _ = reply_tx.send(Ok(admission));
                    }
                }
            },
        ) as std::sync::Arc<dyn Fn(super::dispatcher::SessionDispatchTransition) + Send + Sync>
    };
    let execution_envelope = envelope.clone();
    let result = dispatcher
        .dispatch_observed(
            session_key,
            identity,
            callback,
            move |cancellation| async move {
                let (worker_reply_tx, worker_reply_rx) = oneshot::channel();
                worker_tx
                    .send(super::SessionWorkerCommand::Execute {
                        envelope: execution_envelope,
                        cancellation,
                        reply_tx: worker_reply_tx,
                    })
                    .map_err(|_| "session worker unavailable".to_string())?;
                worker_reply_rx
                    .await
                    .map_err(|_| "session worker unavailable".to_string())?
            },
        )
        .await;
    match result {
        Ok(Some(command)) => {
            if let Some(egress_tx) = egress_tx {
                if let Err(error) = egress_tx.send(command).await {
                    tracing::error!(%error, "failed to publish typed adapter command");
                }
            }
        }
        Ok(None) => {}
        Err(error) => {
            if let Some(reply_tx) = reply_slot
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .take()
            {
                let _ = reply_tx.send(Err(error.to_string()));
            }
        }
    }
}

#[cfg(test)]
mod typed_channel_tests {
    use super::*;
    use agent_diva_core::channel::{
        ChannelAddress, ChannelDirection, ChannelOrigin, ChannelPayloadV1, ContentPart,
        Correlation, OwnerApprovalPolicy, OwnerExecutionContextV1, OwnerTurnIntent, StreamPhase,
    };
    use crate::agent_loop::prepare_turn_envelope;

    #[test]
    fn typed_message_preserves_opaque_session_and_correlations() {
        let mut correlation = Correlation::new("profile/session-1");
        correlation.request_id = Some("request-1".to_string());
        correlation.trace_id = Some("trace-1".to_string());
        let envelope = ChannelEnvelopeV1::new(
            ChannelDirection::Ingress,
            ChannelAddress::new("neuro-link", "chat-1"),
            correlation,
            ChannelOrigin::OwnerFrontend,
            ChannelPayloadV1::Message {
                parts: vec![
                    ContentPart::Text {
                        text: "hello".to_string(),
                    },
                    ContentPart::Markdown {
                    markdown: " **world**".to_string(),
                    },
                ],
                subject: Some("subject".to_string()),
                locale: Some("zh-CN".to_string()),
                context: Some(agent_diva_core::channel::OwnerTurnContextV1 {
                    intent: OwnerTurnIntent::Agent,
                    approval_policy: Some(OwnerApprovalPolicy::OnFailure),
                    execution: Some(OwnerExecutionContextV1 {
                        plan_id: "plan-1".to_string(),
                        revision: 3,
                        execution_id: Some("execution-1".to_string()),
                    }),
                }),
            },
        );
        let (prepared, identity) = prepare_turn_envelope(envelope).unwrap();
        assert_eq!(prepared.correlation.session_key, "profile/session-1");
        assert_eq!(prepared.rendered_message_text().as_deref(), Some("hello\n **world**"));
        assert_eq!(prepared.correlation.request_id.as_deref(), Some("request-1"));
        assert_eq!(prepared.correlation.trace_id.as_deref(), Some("trace-1"));
        assert_eq!(identity.request_id, "request-1");
        assert_eq!(identity.trace_id, "trace-1");
        match prepared.payload {
            ChannelPayloadV1::Message {
                subject,
                locale,
                context: Some(context),
                ..
            } => {
                assert_eq!(subject.as_deref(), Some("subject"));
                assert_eq!(locale.as_deref(), Some("zh-CN"));
                assert_eq!(context.intent, OwnerTurnIntent::Agent);
                assert_eq!(context.approval_policy, Some(OwnerApprovalPolicy::OnFailure));
                assert_eq!(
                    context.execution.as_ref().and_then(|value| value.execution_id.as_deref()),
                    Some("execution-1")
                );
            }
            payload => panic!("unexpected payload: {payload:?}"),
        }
    }

    #[test]
    fn typed_non_message_payload_is_rejected_before_dispatch() {
        let envelope = ChannelEnvelopeV1::new(
            ChannelDirection::Ingress,
            ChannelAddress::new("neuro-link", "chat-1"),
            Correlation::new("session-1"),
            ChannelOrigin::OwnerFrontend,
            ChannelPayloadV1::Stream {
                phase: StreamPhase::Delta,
                parts: Vec::new(),
            },
        );
        let error = prepare_turn_envelope(envelope).unwrap_err();
        assert!(error.contains("message payload"));
    }
}
