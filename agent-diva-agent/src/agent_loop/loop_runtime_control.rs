use super::AgentLoop;
use crate::compaction::ContextCompactor;
use crate::runtime_control::RuntimeControlCommand;
use agent_diva_core::bus::{AgentEvent, InboundMessage, PlanRuntimeState};
use agent_diva_core::session::CompactTrigger;
use agent_diva_providers::Message;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TryRecvError;
use tracing::{info, warn};

impl AgentLoop {
    pub(super) async fn handle_runtime_control_command(&mut self, cmd: RuntimeControlCommand) {
        match cmd {
            RuntimeControlCommand::UpdateNetwork(network) => {
                self.apply_network_config(network).await;
                self.rebuild_tools_for_active_phase().await;
            }
            RuntimeControlCommand::UpdateMcp { servers } => {
                self.apply_mcp_config(servers).await;
                self.rebuild_tools_for_active_phase().await;
            }
            RuntimeControlCommand::StopSession { session_key } => {
                self.cancelled_sessions.insert(session_key);
            }
            RuntimeControlCommand::ResetSession { session_key } => {
                agent_diva_laputa::release_frozen_core_session(&self.workspace, &session_key);
                self.context.reset_session_cache(&session_key);
                if let Some(planning) = self.tool_config.planning.as_ref() {
                    planning.registry.discard_session(&session_key).await;
                }
                if let Err(e) = self.sessions.archive_and_reset(&session_key) {
                    tracing::error!("Failed to archive and reset session: {}", e);
                } else {
                    info!("Archived and reset session: {}", session_key);
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
                let result = self
                    .sessions
                    .delete(&session_key)
                    .map_err(|e| e.to_string());
                if result.is_ok() {
                    agent_diva_laputa::release_frozen_core_session(&self.workspace, &session_key);
                    self.context.end_session_cache(&session_key);
                    if let Some(planning) = self.tool_config.planning.as_ref() {
                        planning.registry.discard_session(&session_key).await;
                    }
                }
                match &result {
                    Ok(deleted) => {
                        info!(
                            session_key = %session_key,
                            deleted = *deleted,
                            "Runtime delete session completed"
                        );
                    }
                    Err(err) => {
                        tracing::error!(
                            session_key = %session_key,
                            error = %err,
                            "Runtime delete session failed"
                        );
                    }
                }
                let _ = reply_tx.send(result);
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
    }

    pub(super) fn clear_session_cancellation(&mut self, session_key: &str) {
        self.cancelled_sessions.remove(session_key);
    }

    pub(super) fn emit_error_event(
        &self,
        msg: &InboundMessage,
        event_tx: Option<&mpsc::UnboundedSender<AgentEvent>>,
        message: impl Into<String>,
    ) {
        let event = AgentEvent::Error {
            message: message.into(),
        };
        if let Some(tx) = event_tx {
            let _ = tx.send(event.clone());
        }
        let _ = self
            .bus
            .publish_event(msg.channel.clone(), msg.chat_id.clone(), event);
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
            let history = session.get_history(50);
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
                let prior = session.compaction_history.clone();
                ContextCompactor::compact(
                    session,
                    &budget_config,
                    provider,
                    &model,
                    CompactTrigger::Manual,
                    &prior,
                )
                .await
            } else {
                return Err("session disappeared during compaction".to_string());
            }
        };

        match compact_result {
            Ok(result) => {
                // Check if there was actually anything to compact
                if result.summary.summary.is_empty()
                    && result.summary.pre_compact_message_count == 0
                {
                    return Ok("nothing to compact — session is already lean".to_string());
                }

                // Persist compaction state — push to history chain
                {
                    let session = self.sessions.get_or_create(session_key);
                    session.last_compacted = result.new_compacted_index;
                    session.compaction_history.push(result.summary.clone());
                }
                if let Some(s) = self.sessions.get(session_key) {
                    if let Err(e) = self.sessions.save(s) {
                        warn!("Failed to persist compaction state: {}", e);
                    }
                }

                info!(
                    "Manual compaction complete: {} msgs → {} chars summary",
                    result.summary.pre_compact_message_count,
                    result.summary.summary.len()
                );

                Ok(format!(
                    "compact done — {} messages compressed, ~{} tokens saved\nsummary: {}",
                    result.summary.pre_compact_message_count,
                    result.summary.pre_compact_estimated_tokens,
                    result.summary.summary
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
            None,
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
}
