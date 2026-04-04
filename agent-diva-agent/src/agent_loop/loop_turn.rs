use super::AgentLoop;
use crate::consolidation;
use agent_diva_core::bus::{AgentEvent, InboundMessage, OutboundMessage};
use agent_diva_core::session::ChatMessage;
use agent_diva_core::soul::SoulStateStore;
use agent_diva_providers::{LLMResponse, LLMStreamEvent};
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
                    format!("{}\n\n[Attachments]\n{}\n[/Attachments]", msg.content, attachment_text)
                }
                _ => msg.content.clone(),
            }
        } else {
            msg.content.clone()
        };

        // Get or create session
        let session_key = format!("{}:{}", msg.channel, msg.chat_id);
        self.clear_session_cancellation(&session_key);
        let session = self.sessions.get_or_create(&session_key);

        // Build initial messages
        let history = session.get_history(50); // Last 50 messages
        let history_len = history.len();
        let mut messages = self.context.build_messages(
            history,
            message_content,
            Some(&msg.channel),
            Some(&msg.chat_id),
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
            let mut stream = self
                .provider
                .chat_stream(
                    messages.clone(),
                    if !tool_defs.is_empty() {
                        Some(tool_defs)
                    } else {
                        None
                    },
                    Some(model_to_use.clone()),
                    4096,
                    0.7,
                )
                .await?;
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
                let memory_manager = agent_diva_core::memory::MemoryManager::new(&self.workspace);
                if let Err(e) = consolidation::consolidate(
                    session,
                    &self.provider,
                    &model_to_use,
                    &memory_manager,
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
    async fn load_attachment_contents(&self, file_ids: &[String]) -> Result<String, Box<dyn std::error::Error>> {
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
                    let mime_type = handle.metadata.mime_type.as_deref().unwrap_or("application/octet-stream");
                    let is_text = mime_type.starts_with("text/")
                        || mime_type == "application/json"
                        || mime_type == "application/javascript"
                        || mime_type == "application/typescript"
                        || mime_type == "application/x-yaml"
                        || mime_type == "application/xml";

                    if is_text && size <= MAX_INLINE_ATTACHMENT_SIZE {
                        match self.file_manager.read(&handle).await {
                            Ok(bytes) => {
                                match String::from_utf8(bytes) {
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
                                }
                            }
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
                    warn!("Failed to get file handle for {}: {}. Storage path: {}", file_id, e, storage_path.display());
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
                    if m.content.trim().is_empty()
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
                        &m.content,
                        None,
                        tool_calls_json,
                        None,
                    );
                    msg.reasoning_content = m.reasoning_content.clone();
                    msg.thinking_blocks = m.thinking_blocks.clone();
                    session.add_full_message(msg);
                }
                "tool" => {
                    let content = if m.content.chars().count() > 500 {
                        format!("{}...", m.content.chars().take(500).collect::<String>())
                    } else {
                        m.content.clone()
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
