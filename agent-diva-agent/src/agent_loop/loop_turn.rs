#[cfg(test)]
use super::turn::iteration::{contains_internal_protocol, InternalProtocolGuard};
use super::turn::{
    admission::AdmittedTurn,
    finalize::{FinalizationContext, FinalizationPreparation},
    iteration::{IterationBudget, IterationOutcome},
    prompt,
    tool_step::{ToolOrchestrationContext, ToolRunSummary},
};
use super::AgentLoop;
use crate::context_assembly::{serialize_dynamic_sections, ContextSection, PromptSection};
use agent_diva_core::bus::{
    AgentEvent, InboundMessage, OutboundMessage, PlanRuntimeState, PlanRuntimeTodo, PokeEvent,
};
use agent_diva_core::planning::model::{PlanPhase, TodoStatus};
use agent_diva_core::planning::store::PlanningStore;
use agent_diva_core::planning::update_plan::UpdatePlanArgs;
use agent_diva_core::reasoning::ThinkingMode;
use agent_diva_core::session::{ChatMessage, Session, TokenUsage};
use agent_diva_core::token_ledger::budget::check_budget_at_path;
use agent_diva_core::token_ledger::{JsonlTokenLedger, TokenLedgerEntry};
use agent_diva_providers::{ImageUrl, Message, MessageContent, MessageContentPart, ProviderError};
use agent_diva_tooling::ToolDefinitionSet;
use anyhow;
use base64::Engine;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::{debug, error, info, trace, warn};

/// Max size for text attachments to inline (100KB)
const MAX_INLINE_ATTACHMENT_SIZE: u64 = 100 * 1024;

fn read_only_tool_definitions(mut definitions: ToolDefinitionSet) -> ToolDefinitionSet {
    definitions.retain(|definition| {
        definition
            .get("function")
            .and_then(|function| function.get("name"))
            .and_then(|name| name.as_str())
            .is_some_and(crate::mask::ToolPolicy::is_read_only_tool)
    });
    definitions
}

#[derive(Debug, Default, Clone)]
pub(super) struct ProcessedInboundMedia {
    pub(super) prompt_text: String,
    pub(super) image_parts: Vec<MessageContentPart>,
}

pub(super) async fn persist_execution_context(
    planning: &crate::tool_config::PlanningConfig,
    session_key: &str,
    execution: &agent_diva_core::planning::ExecutionSession,
) -> anyhow::Result<()> {
    let context = agent_diva_core::planning::PersistedExecutionContext {
        plan_id: execution.report_id.clone(),
        revision: execution.revision,
        session_key: session_key.to_string(),
        execution_id: execution.id.clone(),
        context_policy: execution.context_policy,
        boundary: execution.boundary.clone(),
        compacted_context: execution.compacted_context.clone(),
        initialization_status: execution.initialization_status,
        initialization_error: execution.initialization_error.clone(),
        created_at: execution.created_at,
        updated_at: execution.updated_at,
    };
    planning
        .store
        .update_execution_context(&context)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))
}

pub(super) fn fallback_session_title(session: &Session) -> Option<String> {
    let first_user_msg = session.messages.iter().find(|msg| msg.role == "user")?;
    let content = first_user_msg.content.trim();
    if content.is_empty() {
        return None;
    }
    Some(content.chars().take(20).collect())
}

fn normalize_generated_title(raw: &str) -> Option<String> {
    let first_line = raw.lines().next()?.trim().trim_matches('"').trim();
    if first_line.is_empty() {
        return None;
    }
    Some(first_line.chars().take(60).collect())
}

pub(super) fn should_generate_session_title(session: &Session) -> bool {
    if session.title_manually_set()
        || session.title_generated()
        || session.conversation_title().is_some()
    {
        return false;
    }
    let has_user = session
        .messages
        .iter()
        .any(|message| message.role == "user" && !message.content.trim().is_empty());
    let has_assistant = session
        .messages
        .iter()
        .any(|message| message.role == "assistant" && !message.content.trim().is_empty());
    has_user && has_assistant
}

fn todo_key(todo: &PlanRuntimeTodo) -> String {
    todo.title.trim().to_ascii_lowercase()
}

pub(super) fn build_current_turn_message(
    text: &str,
    image_parts: &[MessageContentPart],
) -> Message {
    if image_parts.is_empty() {
        return Message::user(text);
    }

    let mut parts = Vec::with_capacity(image_parts.len() + usize::from(!text.trim().is_empty()));
    if !text.trim().is_empty() {
        parts.push(MessageContentPart::Text {
            text: text.to_string(),
        });
    }
    parts.extend(image_parts.iter().cloned());
    Message::user(MessageContent::Parts(parts))
}

impl AgentLoop {
    pub(super) fn emit_agent_event(
        &self,
        msg: &InboundMessage,
        event_tx: Option<&mpsc::UnboundedSender<AgentEvent>>,
        event: AgentEvent,
    ) {
        if let Some(tx) = event_tx {
            let _ = tx.send(event.clone());
        }
        let _ = self
            .bus
            .publish_event(msg.channel.clone(), msg.chat_id.clone(), event);
    }

    pub(super) async fn emit_planning_runtime_events(
        &self,
        msg: &InboundMessage,
        event_tx: Option<&mpsc::UnboundedSender<AgentEvent>>,
        tool_name: &str,
        before: Option<PlanRuntimeState>,
        after: Option<PlanRuntimeState>,
    ) {
        let Some(after_plan) = after else {
            return;
        };

        match tool_name {
            "todo_write" => {
                let before_by_title = before
                    .as_ref()
                    .map(|plan| {
                        plan.todos
                            .iter()
                            .map(|todo| (todo_key(todo), todo.clone()))
                            .collect::<HashMap<_, _>>()
                    })
                    .unwrap_or_default();

                for todo in &after_plan.todos {
                    let event = match before_by_title.get(&todo_key(todo)) {
                        None => AgentEvent::TodoCreated {
                            plan: after_plan.clone(),
                            todo: todo.clone(),
                        },
                        Some(previous) if previous.status != todo.status => match todo.status {
                            TodoStatus::Completed => AgentEvent::TodoCompleted {
                                plan: after_plan.clone(),
                                todo: todo.clone(),
                            },
                            TodoStatus::Canceled => AgentEvent::TodoCancelled {
                                plan: after_plan.clone(),
                                todo: todo.clone(),
                            },
                            _ => AgentEvent::TodoStepUpdated {
                                plan: after_plan.clone(),
                                todo: todo.clone(),
                            },
                        },
                        _ => continue,
                    };
                    self.emit_agent_event(msg, event_tx, event);
                }
            }
            "plan_submit" | "plan_transition"
                if after_plan.phase == PlanPhase::AwaitingApproval =>
            {
                self.emit_agent_event(
                    msg,
                    event_tx,
                    AgentEvent::PlanReadyForApproval {
                        plan: after_plan.clone(),
                    },
                );
            }
            _ => {}
        }
    }

    /// Emit a `ChatPlanUpdate` event when the `update_plan` tool succeeds.
    ///
    /// This is intentionally a pure event: the tool itself does not persist the
    /// plan, so the handler only broadcasts the parsed arguments to streaming
    /// consumers and the bus.
    pub(super) async fn emit_chat_plan_update(
        &self,
        msg: &InboundMessage,
        event_tx: Option<&mpsc::UnboundedSender<AgentEvent>>,
        tool_name: &str,
        params: &serde_json::Value,
        is_error: bool,
    ) {
        if is_error || tool_name != "update_plan" {
            return;
        }

        match serde_json::from_value::<UpdatePlanArgs>(params.clone()) {
            Ok(args) => {
                self.emit_agent_event(msg, event_tx, AgentEvent::ChatPlanUpdate { args });
            }
            Err(error) => {
                trace!("Failed to deserialize update_plan arguments: {}", error);
            }
        }
    }

    pub(super) async fn generate_session_title_with_llm(
        &self,
        session: &Session,
        model: &str,
    ) -> Option<String> {
        let first_user_message = session
            .messages
            .iter()
            .find(|message| message.role == "user")
            .map(|message| message.content.trim().to_string())?;
        let first_assistant_message = session
            .messages
            .iter()
            .find(|message| message.role == "assistant")
            .map(|message| message.content.trim().to_string())?;
        if first_user_message.is_empty() || first_assistant_message.is_empty() {
            return None;
        }

        let user_prompt = prompt::session_title_user(&first_user_message, &first_assistant_message);
        let response = self
            .provider
            .chat(
                vec![prompt::session_title_system().system(), user_prompt.user()],
                None,
                agent_diva_providers::ToolChoiceMode::Unspecified,
                Some(model.to_string()),
                64,
                0.2,
            )
            .await
            .ok()?;
        normalize_generated_title(response.content.as_deref().unwrap_or_default())
    }

    pub(super) fn enforce_session_token_budget(
        &self,
        session_key: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(limit) = self.session_token_budget_limit {
            check_budget_at_path(&self.token_ledger_data_root, session_key, limit)?;
        }
        Ok(())
    }

    fn append_session_token_usage(
        &self,
        session_key: &str,
        model: &str,
        usage: &TokenUsage,
        provider_usage: &HashMap<String, i64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if usage.total_tokens == 0 {
            return Ok(());
        }

        let ledger = JsonlTokenLedger::new(&self.token_ledger_data_root)?;
        let cache_creation = provider_usage
            .get("cache_creation_input_tokens")
            .map(|value| (*value).max(0) as u32);
        let cache_read = provider_usage
            .get("cache_read_input_tokens")
            .map(|value| (*value).max(0) as u32);
        ledger.append(
            TokenLedgerEntry::from_usage(session_key, model, usage)
                .with_cache_usage(cache_creation, cache_read),
        )?;
        Ok(())
    }

    pub(super) async fn process_inbound_message_inner(
        &mut self,
        msg: InboundMessage,
        event_tx: Option<&mpsc::UnboundedSender<AgentEvent>>,
        trace_id: String,
    ) -> Result<Option<OutboundMessage>, Box<dyn std::error::Error>> {
        trace!(trace_id = %trace_id, step_name = "msg_received", "Message received from {}:{}", msg.channel, msg.sender_id);

        let AdmittedTurn {
            active_mask,
            model: model_to_use,
            scheduled: is_cron_trigger,
            mode,
            execution_start,
            session_key,
            mut active_execution,
            active_execution_id,
            snapshot: mut turn_snapshot,
            plan_guard_active,
            approved_plan_markdown,
            background_task_context,
        } = self.admit_turn(&msg, &trace_id).await?;
        let plan_mode = mode.is_plan();
        let read_only = mode.is_read_only();

        let runtime_context = self
            .prepare_runtime_context(
                &msg,
                &model_to_use,
                execution_start,
                &session_key,
                &mut active_execution,
                plan_guard_active,
                approved_plan_markdown.as_deref(),
                read_only,
                active_mask.as_ref(),
                is_cron_trigger,
                &trace_id,
                event_tx,
            )
            .await?;
        let message_content = runtime_context.message_content;
        let current_turn_message = runtime_context.current_turn_message;
        let mut turn_messages_start = runtime_context.turn_messages_start;
        let dynamic_sections = runtime_context.dynamic_sections;
        let stable_prefix = runtime_context.stable_prefix;
        let _assembly_report = runtime_context.assembly_report;
        let mut messages = runtime_context.messages;

        // Agent loop
        let mut iteration_budget = IterationBudget::default();
        let mut final_content: Option<String> = None;
        let mut final_reasoning: Option<String> = None;
        let mut turn_token_usage: Option<TokenUsage> = None;
        // Codex-style follow-up: after tools, keep sampling until text or budget.
        let mut tool_run_summaries: Vec<ToolRunSummary> = Vec::new();
        let mut stopped_for_plan_approval = false;

        // Allow at most one bonus summary-only iteration after tools exhaust max_iterations.
        while iteration_budget.can_continue(self.max_iterations) {
            self.drain_runtime_control_commands().await;
            if self.is_session_cancelled(&session_key) {
                self.emit_error_event(&msg, event_tx, "Generation stopped by user.");
                return Ok(None);
            }

            let iteration_pass = iteration_budget.begin_pass(self.max_iterations);
            let iteration = iteration_pass.index;
            let summary_only_pass = iteration_pass.summary_only;
            debug!(
                "Agent iteration {}/{}{}",
                iteration,
                self.max_iterations,
                if summary_only_pass {
                    " (summary-only)"
                } else {
                    ""
                }
            );
            trace!(trace_id = %trace_id, loop_index = iteration, step_name = "loop_started", "Agent loop started");

            let event = AgentEvent::IterationStarted {
                index: iteration,
                max_iterations: self.max_iterations,
            };
            if let Some(tx) = event_tx {
                let _ = tx.send(event.clone());
            }
            let _ = self
                .bus
                .publish_event(msg.channel.clone(), msg.chat_id.clone(), event);

            // Call LLM (streaming when provider supports it)
            // For cron-triggered turns, keep normal tools available but hide cron tool
            // to prevent recursive schedule creation loops.
            // Summary-only pass: no tools (Codex-style final text after tool work).
            let tool_defs: ToolDefinitionSet = if summary_only_pass {
                if iteration_budget.take_summary_nudge() {
                    if let Some(nudge) = serialize_dynamic_sections(
                        &[PromptSection::new(
                            ContextSection::PlanGuard,
                            SUMMARY_ONLY_NUDGE,
                        )],
                        self.provider.dynamic_context_transport(),
                    )? {
                        messages.push(nudge);
                    }
                }
                ToolDefinitionSet {
                    definitions: Vec::new(),
                    core_count: 0,
                }
            } else if read_only {
                read_only_tool_definitions(self.tools.get_definition_set())
            } else if msg.channel == "cron" || is_cron_trigger {
                let mut definitions = self.tools.get_definition_set();
                definitions.retain(|def| {
                    def.get("function")
                        .and_then(|f| f.get("name"))
                        .and_then(|n| n.as_str())
                        != Some("cron")
                });
                definitions
            } else {
                self.tools.get_definition_set()
            };
            let (stream, cache_ticket) = self
                .start_model_stream(
                    &mut messages,
                    &tool_defs,
                    summary_only_pass,
                    &session_key,
                    &model_to_use,
                    &msg,
                    &dynamic_sections,
                    &current_turn_message,
                    &mut turn_messages_start,
                    &stable_prefix,
                    event_tx,
                )
                .await?;
            let model_step = match self
                .collect_model_stream(stream, &session_key, &msg, event_tx)
                .await
            {
                Ok(Some(step)) => step,
                Ok(None) => {
                    self.cache_observer.abandon_call(cache_ticket);
                    return Ok(None);
                }
                Err(error) => {
                    self.cache_observer.abandon_call(cache_ticket);
                    return Err(error);
                }
            };
            let response = model_step.response;
            self.cache_observer
                .note_post_call(cache_ticket, &response.usage);
            let protocol_leak_detected = model_step.protocol_leak_detected;

            // Emit TokenUsed if usage data is available
            if let Some(tokens) = response.usage.get("total_tokens") {
                if *tokens > 0 {
                    let provider = model_to_use
                        .split('/')
                        .next()
                        .unwrap_or("unknown")
                        .to_string();
                    let _ = self.bus.publish_poke_event(PokeEvent::TokenUsed {
                        tokens: *tokens as u32,
                        model: model_to_use.clone(),
                        provider,
                    });
                }
            }

            // Accumulate token usage for this turn
            let iter_usage = extract_token_usage(&response.usage);
            if let Err(e) = self.append_session_token_usage(
                &session_key,
                &model_to_use,
                &iter_usage,
                &response.usage,
            ) {
                warn!("Failed to append token ledger entry: {}", e);
            }
            turn_token_usage = Some(match turn_token_usage {
                Some(existing) => TokenUsage {
                    prompt_tokens: existing
                        .prompt_tokens
                        .saturating_add(iter_usage.prompt_tokens),
                    completion_tokens: existing
                        .completion_tokens
                        .saturating_add(iter_usage.completion_tokens),
                    total_tokens: existing
                        .total_tokens
                        .saturating_add(iter_usage.total_tokens),
                },
                None => iter_usage,
            });

            // Emit ReasoningReceived if reasoning content is available
            if let Some(ref reasoning) = response.reasoning_content {
                if !reasoning.is_empty() {
                    let _ = self.bus.publish_poke_event(PokeEvent::ReasoningReceived {
                        content: reasoning.clone(),
                        model: model_to_use.clone(),
                    });
                }
            }

            // Trace intent decision
            let decision_type = if response.has_tool_calls() {
                "tool_use"
            } else {
                "final_response"
            };
            trace!(trace_id = %trace_id, loop_index = iteration, step_name = "intent_decided", decision_type = %decision_type, "Intent decided");

            if protocol_leak_detected {
                warn!(
                    summary_only_pass,
                    "provider response contained an internal tool protocol; suppressing user-visible output"
                );
                final_content = Some(synthesize_iteration_limit_summary(&tool_run_summaries, &[]));
                final_reasoning = None;
                break;
            }

            // Handle tool calls
            if response.has_tool_calls() {
                // On the summary-only bonus pass we disabled tools; if a provider
                // still emits calls, skip execution and fall through to empty final.
                if summary_only_pass {
                    warn!(
                        "summary-only pass received {} unexpected tool call(s); ignoring tools",
                        response.tool_calls.len()
                    );
                    let pending_tools: Vec<&str> = response
                        .tool_calls
                        .iter()
                        .map(|call| call.name.as_str())
                        .collect();
                    final_content = Some(synthesize_iteration_limit_summary(
                        &tool_run_summaries,
                        &pending_tools,
                    ));
                    final_reasoning = None;
                    break;
                }

                info!("LLM requested {} tool calls", response.tool_calls.len());

                // Add assistant message with tool calls
                self.context.add_assistant_message(
                    &mut messages,
                    response.content.clone(),
                    Some(response.tool_calls.clone()),
                    response.reasoning_content.clone(),
                    None,
                );

                // Execute tools. A transition into AwaitingApproval is a
                // lifecycle barrier: do not ask the model for another tool
                // call in the same turn, otherwise it can immediately retry a
                // mutation after receiving a rejected tool result.
                let mut stop_after_tool_call = false;
                let tool_context = ToolOrchestrationContext {
                    message: &msg,
                    event_tx,
                    session_key: &session_key,
                    trace_id: &trace_id,
                    iteration,
                    plan_mode,
                    plan_guard_active,
                    active_mask: active_mask.as_ref(),
                    active_execution_id: active_execution_id.clone(),
                    background_task_context: background_task_context.clone(),
                };
                for tool_call in &response.tool_calls {
                    let Some(result) = self
                        .orchestrate_tool_call(
                            tool_call,
                            &tool_context,
                            &mut turn_snapshot,
                            &mut messages,
                        )
                        .await?
                    else {
                        return Ok(None);
                    };
                    tool_run_summaries.push(result.summary);
                    stop_after_tool_call = result.stop_after_tool_call;
                    if stop_after_tool_call {
                        break;
                    }
                }
                if stop_after_tool_call {
                    // Lifecycle barrier: do not follow up with mutation tools.
                    stopped_for_plan_approval = true;
                    break;
                }
                // Codex-style needs_follow_up: after tools, sample again.
                // If this was the last normal iteration, grant one summary-only bonus.
                if iteration_budget.grant_summary_bonus_if_exhausted(self.max_iterations) {
                    info!(
                        "tool-only at iteration {}/{}; granting one summary-only pass",
                        iteration, self.max_iterations
                    );
                }
                continue;
            } else {
                // No tool calls, we're done
                if response.finish_reason == "error" {
                    let preview = response
                        .content
                        .as_deref()
                        .map(|s| s.chars().take(200).collect::<String>())
                        .unwrap_or_default();
                    error!("LLM returned error finish_reason with content: {}", preview);
                    final_content =
                        Some("Sorry, I encountered an error calling the AI model.".to_string());
                    final_reasoning = None;
                    break;
                }
                // Treat blank content as missing so fallback synthesis can run.
                final_content = response.content.filter(|s| !s.trim().is_empty());
                final_reasoning = response.reasoning_content;
                // Honor thinking mode: Off clears reasoning, Auto/On pass through
                if self.thinking_mode == ThinkingMode::Off {
                    final_reasoning = None;
                }
                break;
            }
        }

        let final_content = final_content.unwrap_or_else(|| {
            resolve_empty_final_content(stopped_for_plan_approval, &tool_run_summaries)
        });
        let iteration_outcome = IterationOutcome {
            content: Some(final_content.clone()),
            reasoning: final_reasoning.clone(),
            token_usage: turn_token_usage.clone(),
            stopped_for_plan_approval,
        };
        debug_assert_eq!(iteration_outcome.resolved_content(), final_content);
        debug_assert_eq!(
            iteration_outcome.stopped_for_plan_approval,
            stopped_for_plan_approval
        );
        let finalization = self
            .prepare_finalization(
                FinalizationPreparation {
                    message: &msg,
                    event_tx,
                    session_key: &session_key,
                    plan_mode,
                    rendered_content: final_content,
                },
                iteration_outcome,
            )
            .await;
        self.finalize_turn(
            FinalizationContext {
                message: msg,
                messages,
                session_key: turn_snapshot.session_key,
                message_content,
                turn_messages_start,
                system_turn: is_cron_trigger || execution_start,
                model: turn_snapshot.model,
                trace_id: turn_snapshot.trace_id,
            },
            finalization,
            event_tx,
        )
        .await
    }

    /// Load and format attachment contents for inclusion in the message.
    /// Only text files under MAX_INLINE_ATTACHMENT_SIZE are inlined.
    /// For other files, adds a placeholder telling AI to use read_file tool.
    pub(super) async fn load_attachment_contents(
        &self,
        file_ids: &[String],
    ) -> Result<ProcessedInboundMedia, Box<dyn std::error::Error>> {
        let storage_path = dirs::data_local_dir()
            .map(|p| p.join("agent-diva").join("files"))
            .unwrap_or_else(|| PathBuf::from(".agent-diva/files"));
        info!("Loading attachments from: {}", storage_path.display());
        info!("File IDs to load: {:?}", file_ids);
        let mut result = ProcessedInboundMedia::default();

        for file_id in file_ids {
            match self.file_manager.get(file_id).await {
                Ok(handle) => {
                    let size = handle.metadata.size;
                    let mime_type = handle
                        .metadata
                        .mime_type
                        .as_deref()
                        .unwrap_or("application/octet-stream");
                    let is_text = mime_type.starts_with("text/")
                        || mime_type == "application/json"
                        || mime_type == "application/javascript"
                        || mime_type == "application/typescript"
                        || mime_type == "application/x-yaml"
                        || mime_type == "application/xml";
                    let is_image = mime_type.starts_with("image/") || handle.is_image();

                    if is_text && size <= MAX_INLINE_ATTACHMENT_SIZE {
                        match self.file_manager.read(&handle).await {
                            Ok(bytes) => match String::from_utf8(bytes) {
                                Ok(content) => {
                                    result.prompt_text.push_str(&format!(
                                        "--- {} ---\n{}\n---",
                                        handle.metadata.name, content
                                    ));
                                    result.prompt_text.push_str("\n\n");
                                }
                                Err(_) => {
                                    result.prompt_text.push_str(&format!(
                                        "[File: {} ({} bytes, binary)]",
                                        handle.metadata.name, size
                                    ));
                                    result.prompt_text.push_str("\n\n");
                                }
                            },
                            Err(e) => {
                                warn!("Failed to read file {}: {}", file_id, e);
                                result.prompt_text.push_str(&format!(
                                    "[File: {} (error reading: {})]",
                                    handle.metadata.name, e
                                ));
                                result.prompt_text.push_str("\n\n");
                            }
                        }
                    } else if is_image {
                        match self.file_manager.read(&handle).await {
                            Ok(bytes) => {
                                let data_uri = format!(
                                    "data:{};base64,{}",
                                    mime_type,
                                    base64::engine::general_purpose::STANDARD.encode(bytes)
                                );
                                result.image_parts.push(MessageContentPart::ImageUrl {
                                    image_url: ImageUrl { url: data_uri },
                                });
                                result.prompt_text.push_str(&format!(
                                    "[Image: {} ({} bytes, {})]",
                                    handle.metadata.name, size, mime_type
                                ));
                                result.prompt_text.push_str("\n\n");
                            }
                            Err(e) => {
                                warn!("Failed to read image file {}: {}", file_id, e);
                                result.prompt_text.push_str(&format!(
                                    "[Image: {} (error reading: {})]",
                                    handle.metadata.name, e
                                ));
                                result.prompt_text.push_str("\n\n");
                            }
                        }
                    } else {
                        // Non-text or too large - tell AI to use tool
                        result.prompt_text.push_str(&format!(
                            "[File: {} ({} bytes, {}) - Use read_file tool to access]",
                            handle.metadata.name, size, mime_type
                        ));
                        result.prompt_text.push_str("\n\n");
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to get file handle for {}: {}. Storage path: {}",
                        file_id,
                        e,
                        storage_path.display()
                    );
                    result
                        .prompt_text
                        .push_str(&format!("[Attachment: {} (not found - {})]", file_id, e));
                    result.prompt_text.push_str("\n\n");
                }
            }
        }

        result.prompt_text = result.prompt_text.trim().to_string();
        Ok(result)
    }
}

/// Save all messages from the current turn to the session
#[allow(clippy::too_many_arguments)]
pub(super) fn save_turn(
    session: &mut agent_diva_core::session::Session,
    messages: &[agent_diva_providers::Message],
    turn_messages_start: usize,
    user_role: &str,
    user_content: &str,
    final_content: &str,
    turn_token_usage: Option<TokenUsage>,
) {
    // Save trigger message; cron-triggered turns are not real-time user input.
    session.add_message(user_role, user_content);

    // `turn_messages_start` is the message count after system/history/current-user
    // prefix construction (including injected plan notes). Only assistant/tool
    // messages appended by the agent loop are persisted from `messages`.
    if turn_messages_start < messages.len() {
        for m in &messages[turn_messages_start..] {
            match m.role.as_str() {
                "assistant" => {
                    if m.content.to_text_lossy().trim().is_empty()
                        && m.tool_calls
                            .as_ref()
                            .map(|calls| calls.is_empty())
                            .unwrap_or(true)
                    {
                        // Skip empty assistant messages to avoid polluting session history.
                        continue;
                    }
                    let tool_calls_json = m.tool_calls.as_ref().map(|calls| {
                        calls
                            .iter()
                            .filter_map(|tc| serde_json::to_value(tc).ok())
                            .collect::<Vec<_>>()
                    });
                    let mut msg = ChatMessage::with_tool_metadata(
                        "assistant",
                        m.content.to_text_lossy(),
                        None,
                        tool_calls_json,
                        None,
                    );
                    msg.reasoning_content = m.reasoning_content.clone();
                    msg.thinking_blocks = m.thinking_blocks.clone();
                    session.add_full_message(msg);
                }
                "tool" => {
                    let content = if m.content.to_text_lossy().chars().count() > 500 {
                        format!(
                            "{}...",
                            m.content
                                .to_text_lossy()
                                .chars()
                                .take(500)
                                .collect::<String>()
                        )
                    } else {
                        m.content.to_text_lossy()
                    };
                    session.add_full_message(ChatMessage::with_tool_metadata(
                        "tool",
                        content,
                        m.tool_call_id.clone(),
                        None,
                        m.name.clone(),
                    ));
                }
                _ => {}
            }
        }
    }

    // Save the final assistant response if not already captured
    if messages.len() <= turn_messages_start
        || messages.last().map(|m| m.role.as_str()) != Some("assistant")
    {
        let mut final_msg = ChatMessage::new("assistant", final_content);
        if let Some(last) = messages.last() {
            final_msg.reasoning_content = last.reasoning_content.clone();
            final_msg.thinking_blocks = last.thinking_blocks.clone();
        }
        final_msg.token_usage = turn_token_usage;
        session.add_full_message(final_msg);
    } else {
        // The last message was an assistant message already added above.
        // Attach token usage to it if provided.
        if turn_token_usage.is_some() {
            if let Some(last) = session.messages.last_mut() {
                last.token_usage = turn_token_usage;
            }
        }
    }
}

/// Extract token usage from the LLM response usage map.
///
/// Converts the provider's `HashMap<String, i64>` into a structured `TokenUsage`.
/// Missing fields default to 0. Negative values are clamped to 0.
fn extract_token_usage(usage: &std::collections::HashMap<String, i64>) -> TokenUsage {
    let prompt = usage.get("prompt_tokens").copied().unwrap_or(0).max(0) as u32;
    let completion = usage.get("completion_tokens").copied().unwrap_or(0).max(0) as u32;
    let total = usage.get("total_tokens").copied().unwrap_or(0).max(0) as u32;
    TokenUsage {
        prompt_tokens: prompt,
        completion_tokens: completion,
        total_tokens: total,
    }
}

/// Derive a lightweight recall intent from the user message.
///
/// Returns an empty string when the message is too short or lacks any
/// action/recall-indicating words, so `prefetch` is gated on intent
/// availability without requiring a full intent classifier.
pub(super) fn derive_prefetch_intent(message: &str) -> String {
    let trimmed = message.trim();
    if trimmed.len() < 4 {
        return String::new();
    }

    let lower = trimmed.to_lowercase();
    let recall_words = [
        "recall",
        "remember",
        "summarize",
        "summary",
        "review",
        "history",
        "memory",
        "previous",
        "last",
        "recent",
        "what",
        "how",
        "why",
        "when",
        "where",
        "who",
        "list",
        "find",
        "search",
        "look",
        "check",
    ];

    let has_recall_signal = recall_words.iter().any(|word| lower.contains(word));

    if has_recall_signal {
        // Use a truncated version as the search intent.
        let limit = trimmed.chars().count().min(120);
        let chars: String = trimmed.chars().take(limit).collect();
        chars
    } else {
        String::new()
    }
}

/// Check whether a provider error indicates a context-length overflow.
///
/// Matches against known error patterns from various LLM providers
/// (DeepSeek, OpenAI, Anthropic, etc.) that signal the request exceeded
/// the model's maximum context window.
pub(super) fn is_context_overflow_error(err: &ProviderError) -> bool {
    let msg = err.to_string().to_lowercase();
    msg.contains("context_length_exceeded")
        || msg.contains("prompt_too_long")
        || msg.contains("maximum context length")
        || msg.contains("context length")
        || msg.contains("token limit")
        || msg.contains("too many tokens")
        || msg.contains("input length exceeded")
        || msg.contains("max tokens")
        || msg.contains("context window")
}

/// Compact record of a tool execution for empty-final synthesis.
const SUMMARY_ONLY_NUDGE: &str = "You already executed tools in this turn. Based on the tool results above, write a concise final reply for the user in their language. Do not call any tools.";

fn synthesize_iteration_limit_summary(
    summaries: &[ToolRunSummary],
    pending_tools: &[&str],
) -> String {
    let mut summary = format!(
        "任务已达到最大工具迭代次数。已执行 {} 个工具调用。",
        summaries.len()
    );
    if !pending_tools.is_empty() {
        summary.push_str(&format!("未执行的工具请求：{}。", pending_tools.join("、")));
    }
    summary.push_str("未展示内部工具协议，请在继续任务前检查当前结果。");
    summary
}

const FALLBACK_EMPTY_REPLY_ZH: &str = "本轮处理已完成，但未生成可读回复。";
const FALLBACK_PLAN_APPROVAL_ZH: &str = "计划已提交审批，请在下方审批卡片中查看并批准。";

pub(super) fn truncate_for_tool_summary(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let mut out: String = trimmed.chars().take(max_chars.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// User-facing Chinese summary when the model ends after tools without text.
fn synthesize_tool_turn_summary(summaries: &[ToolRunSummary]) -> String {
    if summaries.is_empty() {
        return "已执行工具调用，但模型未返回文字总结。".to_string();
    }
    let total = summaries.len();
    let ok = summaries.iter().filter(|s| s.ok).count();
    let fail = total.saturating_sub(ok);
    let mut parts = vec![format!(
        "已完成 {total} 个工具调用（成功 {ok}，失败 {fail}），但模型未返回文字总结。"
    )];
    // Show at most the last few tools — the final verification is usually last.
    let tail: Vec<&ToolRunSummary> = summaries.iter().rev().take(3).collect();
    for item in tail.into_iter().rev() {
        let status = if item.ok { "成功" } else { "失败" };
        if item.detail.is_empty() {
            parts.push(format!("- `{}`：{}", item.name, status));
        } else {
            parts.push(format!("- `{}`：{} — {}", item.name, status, item.detail));
        }
    }
    parts.join("\n")
}

fn resolve_empty_final_content(
    stopped_for_plan_approval: bool,
    tool_summaries: &[ToolRunSummary],
) -> String {
    if stopped_for_plan_approval {
        return FALLBACK_PLAN_APPROVAL_ZH.to_string();
    }
    if !tool_summaries.is_empty() {
        return synthesize_tool_turn_summary(tool_summaries);
    }
    FALLBACK_EMPTY_REPLY_ZH.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ask_mode_exposes_only_the_read_only_tool_allowlist() {
        let names = [
            "read_file",
            "list_dir",
            "read_attachment",
            "web_search",
            "web_fetch",
            "exec",
            "write_file",
            "edit_file",
            "cron",
            "spawn",
        ];
        let definitions = names
            .into_iter()
            .map(|name| serde_json::json!({"function": {"name": name}}))
            .collect::<Vec<_>>();

        let filtered = read_only_tool_definitions(ToolDefinitionSet {
            core_count: definitions.len(),
            definitions,
        });
        let actual = filtered
            .definitions
            .iter()
            .filter_map(|definition| definition["function"]["name"].as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            vec![
                "read_file",
                "list_dir",
                "read_attachment",
                "web_search",
                "web_fetch"
            ]
        );
    }

    #[test]
    fn synthesize_tool_turn_summary_lists_recent_tools() {
        let summaries = vec![
            ToolRunSummary {
                name: "read_file".into(),
                ok: true,
                detail: "ok".into(),
            },
            ToolRunSummary {
                name: "exec".into(),
                ok: true,
                detail: "rc: 0".into(),
            },
        ];
        let text = synthesize_tool_turn_summary(&summaries);
        assert!(text.contains("2 个工具"));
        assert!(text.contains("`exec`"));
        assert!(text.contains("成功"));
        assert!(!text.contains("I've completed processing"));
    }

    #[test]
    fn protocol_guard_blocks_dsml_split_across_stream_chunks() {
        let mut guard = InternalProtocolGuard::default();
        assert_eq!(guard.push("普通回复 <｜｜DS".into()), None);
        assert_eq!(guard.push("ML｜｜tool_calls>".into()), None);
        assert!(guard.detected());
    }

    #[test]
    fn protocol_guard_flushes_safe_suffix_at_end_of_stream() {
        let reply = "x".repeat(40);
        let mut guard = InternalProtocolGuard::default();
        assert_eq!(guard.push(reply), Some("x".repeat(8)));
        assert_eq!(guard.finish(), Some("x".repeat(32)));
        assert_eq!(guard.finish(), None);
    }

    #[test]
    fn internal_protocol_detection_covers_dsml_and_xml() {
        assert!(contains_internal_protocol(
            "<｜｜DSML｜｜invoke name=\"write_file\">"
        ));
        assert!(contains_internal_protocol("<tool_calls><invoke>"));
        assert!(!contains_internal_protocol("正常的用户可见回复"));
    }

    #[test]
    fn iteration_limit_summary_names_unexecuted_tools() {
        let summary = synthesize_iteration_limit_summary(&[], &["write_file"]);
        assert!(summary.contains("write_file"));
        assert!(summary.contains("最大工具迭代次数"));
    }

    #[test]
    fn resolve_empty_final_prefers_plan_approval_over_tools() {
        let summaries = vec![ToolRunSummary {
            name: "plan_submit".into(),
            ok: true,
            detail: String::new(),
        }];
        let text = resolve_empty_final_content(true, &summaries);
        assert!(text.contains("审批"));
        assert!(!text.contains("I've completed processing"));
    }

    #[test]
    fn resolve_empty_final_uses_tool_synthesis() {
        let summaries = vec![ToolRunSummary {
            name: "exec".into(),
            ok: true,
            detail: "stdout: hello".into(),
        }];
        let text = resolve_empty_final_content(false, &summaries);
        assert!(text.contains("exec"));
        assert!(!text.contains("I've completed processing"));
    }

    #[test]
    fn resolve_empty_final_generic_when_no_tools() {
        let text = resolve_empty_final_content(false, &[]);
        assert_eq!(text, FALLBACK_EMPTY_REPLY_ZH);
    }

    #[test]
    fn truncate_for_tool_summary_limits_chars() {
        let long = "a".repeat(200);
        let out = truncate_for_tool_summary(&long, 50);
        assert!(out.chars().count() <= 50);
        assert!(out.ends_with('…'));
    }

    #[test]
    fn test_derive_prefetch_intent_is_empty_for_non_question() {
        assert!(derive_prefetch_intent("the sky is blue").is_empty());
        assert!(derive_prefetch_intent("ok").is_empty());
        assert!(derive_prefetch_intent("").is_empty());
    }

    #[test]
    fn test_derive_prefetch_intent_has_value_for_action_words() {
        assert!(!derive_prefetch_intent("recall all projects").is_empty());
        assert!(!derive_prefetch_intent("what is the provider boundary?").is_empty());
        assert!(!derive_prefetch_intent("summarize the last meeting").is_empty());
    }

    #[test]
    fn build_current_turn_message_keeps_text_only_shape_without_images() {
        let message = build_current_turn_message("hello", &[]);
        assert_eq!(message.content, MessageContent::Text("hello".to_string()));
    }

    #[test]
    fn build_current_turn_message_combines_text_and_image_parts() {
        let images = vec![MessageContentPart::ImageUrl {
            image_url: ImageUrl {
                url: "data:image/png;base64,AAAA".to_string(),
            },
        }];

        let message = build_current_turn_message("describe this", &images);

        match message.content {
            MessageContent::Parts(parts) => {
                assert_eq!(parts.len(), 2);
                assert_eq!(
                    parts[0],
                    MessageContentPart::Text {
                        text: "describe this".to_string()
                    }
                );
                assert!(matches!(parts[1], MessageContentPart::ImageUrl { .. }));
            }
            other => panic!("expected structured parts, got {other:?}"),
        }
    }

    // ── Reactive compact: context-overflow detection ──────────────

    #[test]
    fn test_is_context_overflow_detects_known_patterns() {
        assert!(is_context_overflow_error(&ProviderError::ApiError(
            "context_length_exceeded".into()
        )));
        assert!(is_context_overflow_error(&ProviderError::ApiError(
            "prompt_too_long".into()
        )));
        assert!(is_context_overflow_error(&ProviderError::InvalidResponse(
            "maximum context length exceeded".into()
        )));
        assert!(is_context_overflow_error(&ProviderError::ApiError(
            "token limit reached".into()
        )));
        assert!(is_context_overflow_error(&ProviderError::ApiError(
            "too many tokens in request".into()
        )));
        assert!(is_context_overflow_error(&ProviderError::ApiError(
            "input length exceeded maximum".into()
        )));
        assert!(is_context_overflow_error(&ProviderError::ApiError(
            "context window exceeded".into()
        )));
    }

    #[test]
    fn test_is_context_overflow_ignores_unrelated_errors() {
        assert!(!is_context_overflow_error(&ProviderError::ApiError(
            "rate limit exceeded".into()
        )));
        assert!(!is_context_overflow_error(&ProviderError::ApiError(
            "invalid api key".into()
        )));
        assert!(!is_context_overflow_error(&ProviderError::JsonError(
            serde_json::from_str::<serde_json::Value>("invalid").unwrap_err()
        )));
        assert!(!is_context_overflow_error(&ProviderError::ConfigError(
            "missing config".into()
        )));
    }

    // ── Token usage extraction ──────────────────────────────────────

    #[test]
    fn test_extract_token_usage_all_fields() {
        let mut usage = HashMap::new();
        usage.insert("prompt_tokens".to_string(), 100);
        usage.insert("completion_tokens".to_string(), 50);
        usage.insert("total_tokens".to_string(), 150);
        let result = extract_token_usage(&usage);
        assert_eq!(result.prompt_tokens, 100);
        assert_eq!(result.completion_tokens, 50);
        assert_eq!(result.total_tokens, 150);
    }

    #[test]
    fn test_extract_token_usage_missing_fields() {
        let usage: HashMap<String, i64> = HashMap::new();
        let result = extract_token_usage(&usage);
        assert_eq!(result.prompt_tokens, 0);
        assert_eq!(result.completion_tokens, 0);
        assert_eq!(result.total_tokens, 0);
    }

    #[test]
    fn test_extract_token_usage_partial_fields() {
        let mut usage = HashMap::new();
        usage.insert("total_tokens".to_string(), 200);
        let result = extract_token_usage(&usage);
        assert_eq!(result.prompt_tokens, 0);
        assert_eq!(result.completion_tokens, 0);
        assert_eq!(result.total_tokens, 200);
    }

    #[test]
    fn title_generation_english() {
        let mut session = Session::new("test:1");
        session.add_message("user", "Hello Agent Diva, this is my first message");
        session.add_message("assistant", "Hi there! How can I help you?");

        let title = fallback_session_title(&session);
        assert_eq!(title, Some("Hello Agent Diva, th".to_string()));
    }

    #[test]
    fn title_generation_chinese() {
        let mut session = Session::new("test:2");
        session.add_message("user", "你好，这是我第一次使用Agent Diva，请多关照");
        session.add_message("assistant", "你好！很高兴为你服务。");

        let title = fallback_session_title(&session);
        assert_eq!(title, Some("你好，这是我第一次使用Agent Div".to_string()));
    }

    #[test]
    fn title_generation_emoji() {
        let mut session = Session::new("test:3");
        session.add_message("user", "👋 Hello! 你好！😊");
        session.add_message("assistant", "Hello! Welcome!");

        let title = fallback_session_title(&session);
        assert_eq!(title, Some("👋 Hello! 你好！😊".to_string()));
    }

    #[test]
    fn title_generation_empty_content() {
        let mut session = Session::new("test:4");
        session.add_message("user", "  ");
        session.add_message("assistant", "Empty message response");

        let title = fallback_session_title(&session);
        assert_eq!(title, None);
    }

    #[test]
    fn title_generation_long_message() {
        let mut session = Session::new("test:5");
        session.add_message(
            "user",
            "This is a very long message that should be truncated to exactly twenty characters by the generate session title function",
        );
        session.add_message("assistant", "Indeed it is.");

        let title = fallback_session_title(&session);
        assert_eq!(title, Some("This is a very long ".to_string()));
    }

    #[test]
    fn title_generation_only_assistant() {
        let mut session = Session::new("test:6");
        session.add_message("assistant", "Hello, I'm an AI");

        let title = fallback_session_title(&session);
        assert_eq!(title, None);
    }

    #[test]
    fn should_not_generate_title_again_once_fallback_title_exists() {
        let mut session = Session::new("test:7");
        session.add_message("user", "Need help with release automation");
        session.add_message("assistant", "I can help with release automation.");
        session.set_conversation_title(Some("Need help with rele".to_string()));
        session.set_title_generated(false);
        session.set_title_manually_set(false);

        assert!(!should_generate_session_title(&session));
    }

    #[test]
    fn test_extract_token_usage_clamps_negative() {
        let mut usage = HashMap::new();
        usage.insert("prompt_tokens".to_string(), -10);
        usage.insert("completion_tokens".to_string(), -5);
        usage.insert("total_tokens".to_string(), -1);
        let result = extract_token_usage(&usage);
        assert_eq!(result.prompt_tokens, 0);
        assert_eq!(result.completion_tokens, 0);
        assert_eq!(result.total_tokens, 0);
    }
}
