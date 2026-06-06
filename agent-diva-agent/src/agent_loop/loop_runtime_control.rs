use super::AgentLoop;
use crate::compaction::ContextCompactor;
use crate::runtime_control::RuntimeControlCommand;
use agent_diva_core::bus::{AgentEvent, InboundMessage};
use agent_diva_core::session::CompactTrigger;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TryRecvError;
use tracing::{info, warn};

impl AgentLoop {
    pub(super) async fn handle_runtime_control_command(&mut self, cmd: RuntimeControlCommand) {
        match cmd {
            RuntimeControlCommand::UpdateNetwork(network) => {
                self.apply_network_config(network).await;
            }
            RuntimeControlCommand::UpdateMentle {
                mentle,
                builtin_mentle,
            } => {
                self.apply_mentle_config(mentle, builtin_mentle).await;
            }
            RuntimeControlCommand::UpdateMcp { servers } => {
                self.apply_mcp_config(servers).await;
            }
            RuntimeControlCommand::StopSession { session_key } => {
                self.cancelled_sessions.insert(session_key);
            }
            RuntimeControlCommand::ResetSession { session_key } => {
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
            RuntimeControlCommand::SetThinking { mode } => {
                self.thinking_mode = mode;
                info!("Thinking mode set to: {:?}", mode);
            }
            RuntimeControlCommand::CompactSession {
                session_key,
                reply_tx,
            } => {
                let result = self.handle_compact_session(&session_key).await;
                let _ = reply_tx.send(result);
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
}
