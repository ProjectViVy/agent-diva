use super::AgentLoop;
use crate::runtime_control::RuntimeControlCommand;
use agent_diva_core::bus::{AgentEvent, InboundMessage};
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TryRecvError;
use tracing::info;

impl AgentLoop {
    pub(super) async fn handle_runtime_control_command(&mut self, cmd: RuntimeControlCommand) {
        match cmd {
            RuntimeControlCommand::UpdateNetwork(network) => {
                self.apply_network_config(network).await;
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
                let session = self.sessions.get(&session_key).cloned();
                let _ = reply_tx.send(session);
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
}
