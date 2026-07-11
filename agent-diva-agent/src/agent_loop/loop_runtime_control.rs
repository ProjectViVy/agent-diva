use super::{policy_phase_for, AgentLoop};
use crate::compaction::ContextCompactor;
use crate::runtime_control::RuntimeControlCommand;
use agent_diva_core::bus::{
    AgentEvent, InboundMessage, PlanApprovalResult, PlanRuntimeState, PlanRuntimeStep,
    PlanRuntimeTodo,
};
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
            RuntimeControlCommand::UpdateMentle {
                mentle,
                builtin_mentle,
            } => {
                self.apply_mentle_config(mentle, builtin_mentle).await;
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
            RuntimeControlCommand::CompactSession {
                session_key,
                reply_tx,
            } => {
                let result = self.handle_compact_session(&session_key).await;
                let _ = reply_tx.send(result);
            }
            RuntimeControlCommand::ApproveActivePlan { request, reply_tx } => {
                let result = self.handle_approve_active_plan(request).await;
                let _ = reply_tx.send(result);
            }
            RuntimeControlCommand::ReturnActivePlanToDraft { reply_tx } => {
                let result = self.handle_return_active_plan_to_draft().await;
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

    async fn handle_approve_active_plan(
        &mut self,
        request: agent_diva_core::planning::ApprovalRequest,
    ) -> Result<PlanApprovalResult, String> {
        let Some(planning) = self.tool_config.planning.as_ref() else {
            return Err("planning runtime is unavailable".to_string());
        };

        let plan_id = planning
            .store
            .get_active_plan()
            .await
            .map_err(|error| error.to_string())?;

        let receipt = planning
            .store
            .approve_plan(&plan_id, &request)
            .await
            .map_err(|error| error.to_string())?;

        let plan = self
            .snapshot_plan_runtime(&plan_id)
            .await
            .ok_or_else(|| "failed to load plan after approval".to_string())?;
        self.rebuild_tools_for_active_phase().await;

        Ok(PlanApprovalResult { plan, receipt })
    }

    async fn handle_return_active_plan_to_draft(&mut self) -> Result<PlanRuntimeState, String> {
        let Some(planning) = self.tool_config.planning.as_ref() else {
            return Err("planning runtime is unavailable".to_string());
        };
        let plan_id = planning
            .store
            .get_active_plan()
            .await
            .map_err(|error| error.to_string())?;
        planning
            .store
            .reopen_plan(&plan_id)
            .await
            .map_err(|error| error.to_string())?;
        let plan = self
            .snapshot_plan_runtime(&plan_id)
            .await
            .ok_or_else(|| "failed to load reopened plan".to_string())?;
        self.rebuild_tools_for_active_phase().await;
        Ok(plan)
    }

    /// Re-assemble after runtime mutations so registration remains a phase
    /// boundary even before a subsequent tool call can re-snapshot state.
    async fn rebuild_tools_for_active_phase(&mut self) {
        let active_mask = self.load_active_mask();
        let active_plan = self.snapshot_active_plan_runtime().await;
        self.rebuild_tools_for_turn(
            active_mask.as_ref(),
            policy_phase_for(active_plan.as_ref(), false),
            None,
            None,
        );
    }

    pub(super) async fn snapshot_active_plan_runtime(&self) -> Option<PlanRuntimeState> {
        let planning = self.tool_config.planning.as_ref()?;
        let plan_id = planning.store.get_active_plan().await.ok()?;
        self.snapshot_plan_runtime(&plan_id).await
    }

    pub(super) async fn snapshot_plan_runtime(
        &self,
        plan_id: &agent_diva_core::planning::ids::PlanId,
    ) -> Option<PlanRuntimeState> {
        let planning = self.tool_config.planning.as_ref()?;
        let plan = planning.store.get_plan(plan_id).await.ok()?;
        let steps = planning.store.get_steps(plan_id).await.ok()?;
        let todos = planning.store.get_todos(plan_id).await.ok()?;
        let revision = planning.store.get_plan_revision(plan_id).await.ok()?;

        Some(PlanRuntimeState {
            plan_id: plan.id.0.clone(),
            revision,
            title: plan.title.clone(),
            goal: plan.goal.clone(),
            phase: plan.phase,
            status: plan.status,
            strategy: plan.strategy.clone(),
            summary: format!("{}: {}", plan.title, plan.goal),
            steps: steps
                .into_iter()
                .map(|step| PlanRuntimeStep {
                    id: step.id,
                    ordinal: step.ordinal,
                    title: step.title,
                    rationale: step.rationale,
                    expected_output: step.expected_output,
                    status: step.status,
                })
                .collect(),
            todos: todos
                .items
                .into_iter()
                .map(|todo| PlanRuntimeTodo {
                    id: todo.id.0,
                    plan_step_id: todo.plan_step_id,
                    title: todo.title,
                    detail: todo.detail,
                    status: todo.status,
                    priority: todo.priority,
                    evidence_ref: todo.evidence_ref,
                    block_reason: todo.block_reason,
                    updated_at: todo.updated_at,
                })
                .collect(),
            created_at: plan.created_at,
            updated_at: plan.updated_at,
        })
    }
}
