use anyhow;
use super::AgentLoop;
use crate::compaction::ContextCompactor;
use crate::consolidation;
use crate::context_budget::check_budget;
use agent_diva_core::bus::{AgentEvent, InboundMessage, OutboundMessage};
use agent_diva_core::memory::{PrefetchRequest, PrefetchStatus};
use agent_diva_core::reasoning::ThinkingMode;
use agent_diva_core::session::{ChatMessage, CompactTrigger};
use agent_diva_core::soul::SoulStateStore;
use agent_diva_providers::{LLMResponse, LLMStreamEvent, ProviderError};
use futures::StreamExt;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, trace, warn};

/// Max size for text attachments to inline (100KB)
const MAX_INLINE_ATTACHMENT_SIZE: u64 = 100 * 1024;

impl AgentLoop {
    pub(super) async fn process_inbound_message_inner(
        &mut self,
        msg: InboundMessage,
        event_tx: Option<&mpsc::UnboundedSender<AgentEvent>>,
        trace_id: String,
    ) -> Result<Option<OutboundMessage>, Box<dyn std::error::Error>> {
        trace!(trace_id = %trace_id, step_name = "msg_received", "Message received from {}:{}", msg.channel, msg.sender_id);

        // Use the default model from the current provider
        let model_to_use = self.provider.get_default_model();

        let preview = if msg.content.chars().count() > 80 {
            format!("{}...", msg.content.chars().take(80).collect::<String>())
        } else {
            msg.content.clone()
        };
        info!(
            "Processing message from {}:{}: {} (model: {})",
            msg.channel, msg.sender_id, preview, model_to_use
        );

        let is_cron_trigger = msg.sender_id == "cron" || msg.metadata.contains_key("cron_job_id");

        // Process attachments: load text file contents and append to message
        let message_content = if !msg.media.is_empty() {
            match self.load_attachment_contents(&msg.media).await {
                Ok(attachment_text) if !attachment_text.is_empty() => {
                    format!(
                        "{}\n\n[Attachments]\n{}\n[/Attachments]",
                        msg.content, attachment_text
                    )
                }
                _ => msg.content.clone(),
            }
        } else {
            msg.content.clone()
        };

        // Derive prefetch intent from raw user message before it's consumed.
        let prefetch_intent = derive_prefetch_intent(&message_content);
        let prefetch_user_message = message_content.clone();

        // Get or create session
        let session_key = format!("{}:{}", msg.channel, msg.chat_id);
        self.clear_session_cancellation(&session_key);

        // ── Build initial messages with budget-aware compaction ──
        // Phase 1: check budget and decide if compaction is needed (release borrow before .await)
        let (history, history_len, should_compact, budget_report) = {
            let session = self.sessions.get_or_create(&session_key);
            let history = session.get_history(50); // Last 50 messages
            let history_len = history.len();

            // Budget check against context window limits
            let budget_config = self.tool_config.budget.clone();
            let budget_report = check_budget(&history, &budget_config);

            (history, history_len, budget_report.should_compact, budget_report)
        };

        // Phase 2: call async compact() if needed, then update session
        let (compaction_clone, did_compact) = if should_compact {
            info!(
                "Compaction triggered — budget pressure {:.1}% ({} tokens used of ~{} history budget)",
                budget_report.pressure_ratio * 100.0,
                budget_report.history_estimated,
                budget_report.total_estimated.saturating_sub(budget_report.system_estimated),
            );

            let provider = self.provider.clone();
            let model = self.model.clone();
            let budget_config = self.tool_config.budget.clone();

            // Use immutable get() to avoid holding &mut across .await
            let compact_result = {
                if let Some(session) = self.sessions.get(&session_key) {
                    ContextCompactor::compact(session, &budget_config, provider, &model, CompactTrigger::Auto).await
                } else {
                    Err(anyhow::anyhow!("Session not found for compaction"))
                }
            };

            match compact_result {
                Ok(result) => {
                    let session = self.sessions.get_or_create(&session_key);
                    session.last_compacted = result.new_compacted_index;
                    session.compaction = Some(result.summary.clone());
                    (Some(result.summary), true)
                }
                Err(e) => {
                    warn!("Compaction failed (non-blocking): {}", e);
                    // Carry forward any existing compaction as fallback
                    let session = self.sessions.get_or_create(&session_key);
                    (session.compaction.clone(), false)
                }
            }
        } else {
            // Carry forward any existing compaction from a previous turn
            let session = self.sessions.get_or_create(&session_key);
            (session.compaction.clone(), false)
        };

        // Persist compaction state immediately when it just occurred
        if did_compact {
            if let Some(s) = self.sessions.get(&session_key) {
                if let Err(e) = self.sessions.save(s) {
                    error!("Failed to persist compaction state: {}", e);
                }
            }
        }

        let mut messages = self.context.build_messages(
            history,
            message_content.clone(),
            Some(&msg.channel),
            Some(&msg.chat_id),
            compaction_clone.as_ref(),
        );
        if is_cron_trigger {
            // Make trigger origin explicit so the model does not treat it as a fresh user request.
            let current_message = messages.pop();
            messages.push(agent_diva_providers::Message::system(
                "This turn is triggered automatically by a scheduled cron job, not by a real-time user input. Do not schedule new reminders/jobs from this turn unless explicitly required by prior task design.",
            ));
            if let Some(current_message) = current_message {
                messages.push(current_message);
            }
        }

        // Agent loop
        let mut iteration = 0;
        let mut final_content: Option<String> = None;
        let mut final_reasoning: Option<String> = None;
        let mut soul_files_changed: HashSet<String> = HashSet::new();

        // Intent-aware prefetch: run recall search before the first LLM call
        // when the user message provides a workable intent string.
        if !prefetch_intent.is_empty() {
            let prefetch_result = self
                .memory_provider
                .prefetch(PrefetchRequest {
                    workspace_root: self.workspace.clone(),
                    intent: prefetch_intent,
                    current_room: Some(msg.channel.clone()),
                    user_message: Some(prefetch_user_message.clone()),
                })
                .await;
            match prefetch_result {
                Ok(response) => match response.status {
                    PrefetchStatus::Failed { reason } => {
                        warn!("Prefetch recall failed (non-fatal): {}", reason);
                    }
                    _ => {
                        if let Some(block) = response.prompt_block {
                            // Inject recall results as an additional system message
                            // right after the main system prompt.
                            messages.insert(1, agent_diva_providers::Message::system(block));
                            trace!(trace_id = %trace_id, step_name = "prefetch_injected", "Prefetch recall injected into turn context");
                        } else {
                            trace!(trace_id = %trace_id, step_name = "prefetch_skipped", "Prefetch skipped or empty");
                        }
                    }
                },
                Err(e) => {
                    warn!("Prefetch recall failed (non-fatal): {}", e);
                }
            }
        }

        while iteration < self.max_iterations {
            self.drain_runtime_control_commands().await;
            if self.is_session_cancelled(&session_key) {
                self.emit_error_event(&msg, event_tx, "Generation stopped by user.");
                return Ok(None);
            }

            iteration += 1;
            debug!("Agent iteration {}/{}", iteration, self.max_iterations);
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
            let tool_defs = if msg.channel == "cron" || is_cron_trigger {
                self.tools
                    .get_definitions()
                    .into_iter()
                    .filter(|def| {
                        def.get("function")
                            .and_then(|f| f.get("name"))
                            .and_then(|n| n.as_str())
                            != Some("cron")
                    })
                    .collect()
            } else {
                self.tools.get_definitions()
            };
            // Call LLM with reactive context-overflow safety net.
            // On context_length_exceeded, perform emergency compaction and retry once.
            let mut reactive_retry_attempted = false;
            let mut stream = loop {
                let tool_defs_for_call = if !tool_defs.is_empty() {
                    Some(tool_defs.clone())
                } else {
                    None
                };
                match self
                    .provider
                    .chat_stream(
                        messages.clone(),
                        tool_defs_for_call,
                        Some(model_to_use.clone()),
                        4096,
                        0.7,
                    )
                    .await
                {
                    Ok(s) => break s,
                    Err(e) => {
                        if !reactive_retry_attempted && is_context_overflow_error(&e) {
                            warn!(
                                "Context overflow detected from provider: {}. Triggering reactive compaction...",
                                e
                            );
                            reactive_retry_attempted = true;

                            // --- Reactive compaction ---
                            let provider = self.provider.clone();
                            let model = self.model.clone();
                            let budget_config = self.tool_config.budget.clone();

                            // Phase 1: call compact() with immutable session ref
                            let compact_result = {
                                if let Some(session) = self.sessions.get(&session_key) {
                                    ContextCompactor::compact(
                                        session, &budget_config, provider, &model,
                                        CompactTrigger::Reactive,
                                    ).await
                                } else {
                                    Err(anyhow::anyhow!("Session not found for reactive compaction"))
                                }
                            };

                            // Phase 2: apply result and get updated history
                            let (reactive_history, reactive_compaction) = match compact_result {
                                Ok(result) => {
                                    let session = self.sessions.get_or_create(&session_key);
                                    session.last_compacted = result.new_compacted_index;
                                    session.compaction = Some(result.summary.clone());
                                    let history = session.get_history(50);
                                    (history, result.summary)
                                }
                                Err(e) => {
                                    warn!("Reactive compaction failed (non-blocking): {}", e);
                                    // Fallback: use existing session state without updating
                                    let session = self.sessions.get_or_create(&session_key);
                                    let history = session.get_history(50);
                                    let fallback = session.compaction.clone().unwrap_or_else(|| {
                                        agent_diva_core::session::CompactSummary {
                                            schema_version: 1,
                                            compact_id: String::new(),
                                            created_at: String::new(),
                                            trigger: CompactTrigger::Reactive,
                                            source_range: agent_diva_core::session::CompactionRange {
                                                start_index: 0,
                                                end_index: 0,
                                            },
                                            kept_recent_count: 0,
                                            pre_compact_message_count: 0,
                                            pre_compact_estimated_tokens: 0,
                                            summary: String::new(),
                                            quality_score: None,
                                            retry_count: 0,
                                        }
                                    });
                                    (history, fallback)
                                }
                            };

                            // Persist compaction state
                            if let Some(s) = self.sessions.get(&session_key) {
                                if let Err(persist_err) = self.sessions.save(s) {
                                    error!(
                                        "Failed to persist reactive compaction state: {}",
                                        persist_err
                                    );
                                }
                            }

                            // Rebuild messages with new compaction summary
                            messages = self.context.build_messages(
                                reactive_history,
                                message_content.clone(),
                                Some(&msg.channel),
                                Some(&msg.chat_id),
                                Some(&reactive_compaction),
                            );
                            if is_cron_trigger {
                                let current_message = messages.pop();
                                messages.push(agent_diva_providers::Message::system(
                                    "This turn is triggered automatically by a scheduled cron job, not by a real-time user input. Do not schedule new reminders/jobs from this turn unless explicitly required by prior task design.",
                                ));
                                if let Some(current_message) = current_message {
                                    messages.push(current_message);
                                }
                            }

                            info!("Reactive compaction complete, retrying provider call...");
                            continue; // retry once
                        } else {
                            return Err(e.into());
                        }
                    }
                }
            };
            let mut streamed_content = String::new();
            let mut streamed_reasoning = String::new();
            let mut response: Option<LLMResponse> = None;
            loop {
                self.drain_runtime_control_commands().await;
                if self.is_session_cancelled(&session_key) {
                    self.emit_error_event(&msg, event_tx, "Generation stopped by user.");
                    return Ok(None);
                }

                let stream_event =
                    match tokio::time::timeout(Duration::from_millis(250), stream.next()).await {
                        Ok(Some(event)) => event,
                        Ok(None) => break,
                        Err(_) => continue,
                    };

                match stream_event? {
                    LLMStreamEvent::TextDelta(delta) => {
                        streamed_content.push_str(&delta);
                        let event = AgentEvent::AssistantDelta { text: delta };
                        if let Some(tx) = event_tx {
                            let _ = tx.send(event.clone());
                        }
                        let _ =
                            self.bus
                                .publish_event(msg.channel.clone(), msg.chat_id.clone(), event);
                    }
                    LLMStreamEvent::ReasoningDelta(delta) => {
                        debug!("Stream ReasoningDelta: {:?}", delta);
                        streamed_reasoning.push_str(&delta);
                        let event = AgentEvent::ReasoningDelta { text: delta };
                        if let Some(tx) = event_tx {
                            let _ = tx.send(event.clone());
                        }
                        let _ =
                            self.bus
                                .publish_event(msg.channel.clone(), msg.chat_id.clone(), event);
                    }
                    LLMStreamEvent::ToolCallDelta {
                        name,
                        arguments_delta,
                        ..
                    } => {
                        if let Some(delta) = arguments_delta {
                            let event = AgentEvent::ToolCallDelta {
                                name,
                                args_delta: delta,
                            };
                            if let Some(tx) = event_tx {
                                let _ = tx.send(event.clone());
                            }
                            let _ = self.bus.publish_event(
                                msg.channel.clone(),
                                msg.chat_id.clone(),
                                event,
                            );
                        }
                    }
                    LLMStreamEvent::Completed(done) => {
                        response = Some(done);
                        break;
                    }
                }
            }
            let response = response.unwrap_or_else(|| LLMResponse {
                content: if streamed_content.is_empty() {
                    None
                } else {
                    Some(streamed_content)
                },
                tool_calls: Vec::new(),
                finish_reason: "stop".to_string(),
                usage: std::collections::HashMap::new(),
                reasoning_content: if streamed_reasoning.is_empty() {
                    None
                } else {
                    Some(streamed_reasoning)
                },
            });

            // Trace intent decision
            let decision_type = if response.has_tool_calls() {
                "tool_use"
            } else {
                "final_response"
            };
            trace!(trace_id = %trace_id, loop_index = iteration, step_name = "intent_decided", decision_type = %decision_type, "Intent decided");

            // Handle tool calls
            if response.has_tool_calls() {
                info!("LLM requested {} tool calls", response.tool_calls.len());

                // Add assistant message with tool calls
                self.context.add_assistant_message(
                    &mut messages,
                    response.content.clone(),
                    Some(response.tool_calls.clone()),
                    response.reasoning_content.clone(),
                    None,
                );

                // Execute tools
                for tool_call in &response.tool_calls {
                    self.drain_runtime_control_commands().await;
                    if self.is_session_cancelled(&session_key) {
                        self.emit_error_event(&msg, event_tx, "Generation stopped by user.");
                        return Ok(None);
                    }

                    trace!(trace_id = %trace_id, loop_index = iteration, step_name = "tool_invoked", tool_name = %tool_call.name, "Tool invoked");

                    let args_str = serde_json::to_string(&tool_call.arguments).unwrap_or_default();
                    let preview = if args_str.chars().count() > 200 {
                        format!("{}...", args_str.chars().take(200).collect::<String>())
                    } else {
                        args_str.clone()
                    };
                    info!("Tool call: {}({})", tool_call.name, preview);
                    let event = AgentEvent::ToolCallStarted {
                        name: tool_call.name.clone(),
                        args_preview: preview.clone(),
                        call_id: tool_call.id.clone(),
                    };
                    if let Some(tx) = event_tx {
                        let _ = tx.send(event.clone());
                    }
                    let _ = self
                        .bus
                        .publish_event(msg.channel.clone(), msg.chat_id.clone(), event);

                    let result = match serde_json::to_value(&tool_call.arguments) {
                        Ok(mut params_value) => {
                            if tool_call.name == "cron" {
                                if let Some(params_obj) = params_value.as_object_mut() {
                                    params_obj.insert(
                                        "context_channel".to_string(),
                                        serde_json::Value::String(msg.channel.clone()),
                                    );
                                    params_obj.insert(
                                        "context_chat_id".to_string(),
                                        serde_json::Value::String(msg.chat_id.clone()),
                                    );
                                    if msg.channel == "cron" || is_cron_trigger {
                                        params_obj.insert(
                                            "_in_cron_context".to_string(),
                                            serde_json::Value::Bool(true),
                                        );
                                    }
                                }
                            }
                            if is_cron_trigger && tool_call.name == "cron" {
                                "Error: cron tool is disabled during cron-triggered execution to prevent recursive scheduling".to_string()
                            } else {
                                self.tools.execute(&tool_call.name, params_value).await
                            }
                        }
                        Err(e) => {
                            warn!(
                                "Failed to serialize arguments for tool '{}' (call_id: {}): {}",
                                tool_call.name, tool_call.id, e
                            );
                            format!(
                                "Error: failed to serialize arguments for tool '{}': {}",
                                tool_call.name, e
                            )
                        }
                    };
                    if self.notify_on_soul_change {
                        if let Some(changed_file) =
                            changed_soul_file(&tool_call.name, &tool_call.arguments, &result)
                        {
                            if changed_file == "BOOTSTRAP.md" {
                                let _ =
                                    SoulStateStore::new(&self.workspace).mark_bootstrap_completed();
                            }
                            soul_files_changed.insert(changed_file.to_string());
                        }
                    }

                    trace!(trace_id = %trace_id, loop_index = iteration, step_name = "tool_completed", tool_name = %tool_call.name, "Tool completed");

                    let event = AgentEvent::ToolCallFinished {
                        name: tool_call.name.clone(),
                        is_error: result.starts_with("Error"),
                        result: result.clone(),
                        call_id: tool_call.id.clone(),
                    };
                    if let Some(tx) = event_tx {
                        let _ = tx.send(event.clone());
                    }
                    let _ = self
                        .bus
                        .publish_event(msg.channel.clone(), msg.chat_id.clone(), event);
                    self.context.add_tool_result(
                        &mut messages,
                        tool_call.id.clone(),
                        tool_call.name.clone(),
                        result,
                    );
                }
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
                final_content = response.content;
                final_reasoning = response.reasoning_content;
                // Honor thinking mode: Off clears reasoning, Auto/On pass through
                if self.thinking_mode == ThinkingMode::Off {
                    final_reasoning = None;
                }
                break;
            }
        }

        let mut final_content = final_content.unwrap_or_else(|| {
            "I've completed processing but have no response to give.".to_string()
        });
        if self.notify_on_soul_change && !soul_files_changed.is_empty() {
            let frequent_hint = self.is_frequent_soul_change_turn();
            let notice = format_soul_transparency_notice(
                &soul_files_changed,
                self.soul_governance.boundary_confirmation_hint,
                frequent_hint,
            );
            final_content.push_str(&notice);
        }

        trace!(trace_id = %trace_id, step_name = "response_generated", "Response generated");

        // Log response preview - use char indices to handle multi-byte UTF-8 characters safely
        let preview = if final_content.chars().count() > 120 {
            format!("{}...", final_content.chars().take(120).collect::<String>())
        } else {
            final_content.clone()
        };
        info!("Response to {}:{}: {}", msg.channel, msg.sender_id, preview);
        let event = AgentEvent::FinalResponse {
            content: final_content.clone(),
        };
        if let Some(tx) = event_tx {
            let _ = tx.send(event.clone());
        }
        let _ = self
            .bus
            .publish_event(msg.channel.clone(), msg.chat_id.clone(), event);

        // Save complete turn to session
        {
            let session = self.sessions.get_or_create(&session_key);
            let user_role = if is_cron_trigger { "system" } else { "user" };
            save_turn(
                session,
                &messages,
                history_len,
                user_role,
                &msg.content,
                &final_content,
            );
        }

        // Run memory consolidation if threshold reached
        {
            let session = self.sessions.get_or_create(&session_key);
            if consolidation::should_consolidate(session, self.memory_window) {
                if let Err(e) = consolidation::consolidate(
                    session,
                    &self.provider,
                    &model_to_use,
                    &self.workspace,
                    &*self.memory_provider,
                    self.memory_window,
                )
                .await
                {
                    error!("Memory consolidation failed: {}", e);
                }
            }
        }

        // Persist session to disk
        if let Some(session) = self.sessions.get(&session_key) {
            if let Err(e) = self.sessions.save(session) {
                error!("Failed to save session: {}", e);
            }
        }

        // Extract reply_to from metadata if available (critical for platforms like QQ)
        let reply_to = msg
            .metadata
            .get("message_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        trace!(trace_id = %trace_id, step_name = "msg_sent_to_channel", "Returning response to channel/manager");
        // Also trace sent to manager as requested, which is effectively this return
        trace!(trace_id = %trace_id, step_name = "msg_sent_to_manager", "Returning response to manager");

        Ok(Some(OutboundMessage {
            channel: msg.channel,
            chat_id: msg.chat_id,
            content: final_content,
            reply_to,
            media: vec![],
            reasoning_content: final_reasoning,
            metadata: msg.metadata,
        }))
    }

    /// Load and format attachment contents for inclusion in the message.
    /// Only text files under MAX_INLINE_ATTACHMENT_SIZE are inlined.
    /// For other files, adds a placeholder telling AI to use read_file tool.
    async fn load_attachment_contents(
        &self,
        file_ids: &[String],
    ) -> Result<String, Box<dyn std::error::Error>> {
        let storage_path = dirs::data_local_dir()
            .map(|p| p.join("agent-diva").join("files"))
            .unwrap_or_else(|| PathBuf::from(".agent-diva/files"));
        info!("Loading attachments from: {}", storage_path.display());
        info!("File IDs to load: {:?}", file_ids);
        let mut parts = Vec::new();

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

                    if is_text && size <= MAX_INLINE_ATTACHMENT_SIZE {
                        match self.file_manager.read(&handle).await {
                            Ok(bytes) => match String::from_utf8(bytes) {
                                Ok(content) => {
                                    parts.push(format!(
                                        "--- {} ---\n{}\n---",
                                        handle.metadata.name, content
                                    ));
                                }
                                Err(_) => {
                                    parts.push(format!(
                                        "[File: {} ({} bytes, binary)]",
                                        handle.metadata.name, size
                                    ));
                                }
                            },
                            Err(e) => {
                                warn!("Failed to read file {}: {}", file_id, e);
                                parts.push(format!(
                                    "[File: {} (error reading: {})]",
                                    handle.metadata.name, e
                                ));
                            }
                        }
                    } else {
                        // Non-text or too large - tell AI to use tool
                        parts.push(format!(
                            "[File: {} ({} bytes, {}) - Use read_file tool to access]",
                            handle.metadata.name, size, mime_type
                        ));
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to get file handle for {}: {}. Storage path: {}",
                        file_id,
                        e,
                        storage_path.display()
                    );
                    parts.push(format!("[Attachment: {} (not found - {})]", file_id, e));
                }
            }
        }

        Ok(parts.join("\n\n"))
    }
}

fn changed_soul_file(
    tool_name: &str,
    arguments: &HashMap<String, serde_json::Value>,
    result: &str,
) -> Option<&'static str> {
    if result.starts_with("Error") || result.starts_with("Warning") {
        return None;
    }
    if tool_name != "write_file" && tool_name != "edit_file" {
        return None;
    }

    let path = arguments.get("path").and_then(|v| v.as_str())?;
    let file_name = Path::new(path).file_name()?.to_string_lossy();

    ["SOUL.md", "IDENTITY.md", "USER.md", "BOOTSTRAP.md"]
        .into_iter()
        .find(|name| file_name.eq_ignore_ascii_case(name))
}

fn format_soul_transparency_notice(
    changed_files: &HashSet<String>,
    boundary_confirmation_hint: bool,
    frequent_hint: bool,
) -> String {
    let mut changed_files = changed_files.iter().cloned().collect::<Vec<_>>();
    changed_files.sort();
    let mut notice =
        "\n\nTransparency notice: I updated soul identity files this turn.".to_string();
    notice.push_str("\n- Updated files: ");
    notice.push_str(&changed_files.join(", "));
    notice.push_str(
        "\n- Reason: to keep identity, boundaries, and behavior guidance aligned with this conversation.",
    );
    if boundary_confirmation_hint && changed_files.iter().any(|f| f == "SOUL.md") {
        notice.push_str(
            "\n- Suggestion: if boundary-related rules changed in SOUL.md, please confirm they match your expectations.",
        );
    }
    if frequent_hint {
        notice.push_str(
            "\n- Governance hint: soul files changed frequently in a short window; consider consolidating updates for stability.",
        );
    }
    notice
}

/// Save all messages from the current turn to the session
fn save_turn(
    session: &mut agent_diva_core::session::Session,
    messages: &[agent_diva_providers::Message],
    history_len: usize,
    user_role: &str,
    user_content: &str,
    final_content: &str,
) {
    // Save trigger message; cron-triggered turns are not real-time user input.
    session.add_message(user_role, user_content);

    // Skip system prompt (1) + history (history_len) + current user message (1)
    let turn_start = 1 + history_len + 1;
    if turn_start < messages.len() {
        for m in &messages[turn_start..] {
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
                        &m.content.to_text_lossy(),
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
    if messages.len() <= turn_start || messages.last().map(|m| m.role.as_str()) != Some("assistant")
    {
        let mut final_msg = ChatMessage::new("assistant", final_content);
        if let Some(last) = messages.last() {
            final_msg.reasoning_content = last.reasoning_content.clone();
            final_msg.thinking_blocks = last.thinking_blocks.clone();
        }
        session.add_full_message(final_msg);
    }
}

/// Derive a lightweight recall intent from the user message.
///
/// Returns an empty string when the message is too short or lacks any
/// action/recall-indicating words, so `prefetch` is gated on intent
/// availability without requiring a full intent classifier.
fn derive_prefetch_intent(message: &str) -> String {
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
fn is_context_overflow_error(err: &ProviderError) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_changed_soul_file_detects_successful_updates() {
        let args = HashMap::from([(
            "path".to_string(),
            serde_json::Value::String("memory/../SOUL.md".to_string()),
        )]);
        let result = "Successfully wrote 12 bytes";
        assert_eq!(
            changed_soul_file("write_file", &args, result),
            Some("SOUL.md")
        );

        let args = HashMap::from([(
            "path".to_string(),
            serde_json::Value::String("IDENTITY.md".to_string()),
        )]);
        assert_eq!(
            changed_soul_file("edit_file", &args, "Successfully edited"),
            Some("IDENTITY.md")
        );
    }

    #[test]
    fn test_changed_soul_file_ignores_errors_and_other_tools() {
        let args = HashMap::from([(
            "path".to_string(),
            serde_json::Value::String("SOUL.md".to_string()),
        )]);
        assert_eq!(
            changed_soul_file("write_file", &args, "Error writing file: denied"),
            None
        );
        assert_eq!(
            changed_soul_file("list_dir", &args, "Successfully listed"),
            None
        );
    }

    #[test]
    fn test_changed_soul_file_ignores_non_soul_paths() {
        let args = HashMap::from([(
            "path".to_string(),
            serde_json::Value::String("README.md".to_string()),
        )]);
        assert_eq!(
            changed_soul_file("write_file", &args, "Successfully wrote"),
            None
        );
    }

    #[test]
    fn test_format_soul_transparency_notice_lists_sorted_files_and_hints() {
        let files = HashSet::from([
            "USER.md".to_string(),
            "SOUL.md".to_string(),
            "IDENTITY.md".to_string(),
        ]);
        let notice = format_soul_transparency_notice(&files, true, true);
        assert!(notice.contains("IDENTITY.md, SOUL.md, USER.md"));
        assert!(notice.contains("Suggestion: if boundary-related rules changed in SOUL.md"));
        assert!(notice.contains("Governance hint: soul files changed frequently"));
    }

    #[test]
    fn test_format_soul_transparency_notice_without_optional_hints() {
        let files = HashSet::from(["USER.md".to_string()]);
        let notice = format_soul_transparency_notice(&files, true, false);
        assert!(!notice.contains("Suggestion: if boundary-related rules changed in SOUL.md"));
        assert!(!notice.contains("Governance hint:"));
    }

    // ── Reactive compact: context-overflow detection ──────────────

    #[test]
    fn test_is_context_overflow_detects_known_patterns() {
        assert!(is_context_overflow_error(
            &ProviderError::ApiError("context_length_exceeded".into())
        ));
        assert!(is_context_overflow_error(
            &ProviderError::ApiError("prompt_too_long".into())
        ));
        assert!(is_context_overflow_error(
            &ProviderError::InvalidResponse("maximum context length exceeded".into())
        ));
        assert!(is_context_overflow_error(
            &ProviderError::ApiError("token limit reached".into())
        ));
        assert!(is_context_overflow_error(
            &ProviderError::ApiError("too many tokens in request".into())
        ));
        assert!(is_context_overflow_error(
            &ProviderError::ApiError("input length exceeded maximum".into())
        ));
        assert!(is_context_overflow_error(
            &ProviderError::ApiError("context window exceeded".into())
        ));
    }

    #[test]
    fn test_is_context_overflow_ignores_unrelated_errors() {
        assert!(!is_context_overflow_error(
            &ProviderError::ApiError("rate limit exceeded".into())
        ));
        assert!(!is_context_overflow_error(
            &ProviderError::ApiError("invalid api key".into())
        ));
        assert!(!is_context_overflow_error(
            &ProviderError::JsonError(serde_json::from_str::<serde_json::Value>("invalid").unwrap_err())
        ));
        assert!(!is_context_overflow_error(
            &ProviderError::ConfigError("missing config".into())
        ));
    }
}
