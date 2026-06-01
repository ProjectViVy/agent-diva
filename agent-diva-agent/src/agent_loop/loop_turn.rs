use super::AgentLoop;
use crate::consolidation;
use agent_diva_core::attachment::FileAttachmentRef;
use agent_diva_core::bus::{AgentEvent, InboundMessage, OutboundMessage};
use agent_diva_core::memory::PrefetchRequest;
use agent_diva_core::session::ChatMessage;
use agent_diva_core::soul::SoulStateStore;
use agent_diva_files::FileManager;
use agent_diva_providers::{
    ImageFile, LLMResponse, LLMStreamEvent, MessageContent, MessageContentPart,
};
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

        // Process attachments: keep images as structured parts and inline text attachments.
        let message_content =
            assemble_current_message_content(&self.file_manager, &msg.content, &msg.media).await;

        // Derive prefetch intent from raw user message before it's consumed.
        let prefetch_user_message = message_content.to_text_lossy();
        let prefetch_intent = derive_prefetch_intent(&prefetch_user_message);

        // Get or create session
        let session_key = format!("{}:{}", msg.channel, msg.chat_id);
        self.clear_session_cancellation(&session_key);
        let session = self.sessions.get_or_create(&session_key);

        // Build initial messages
        let history = session.get_history(50); // Last 50 messages
        let history_len = history.len();
        let mut messages = self.context.build_messages_with_content(
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
                Ok(response) if response.prompt_block.is_some() => {
                    let block = response.prompt_block.unwrap();
                    // Inject recall results as an additional system message
                    // right after the main system prompt.
                    messages.insert(1, agent_diva_providers::Message::system(block));
                    trace!(trace_id = %trace_id, step_name = "prefetch_injected", "Prefetch recall injected into turn context");
                }
                Ok(_) => {
                    trace!(trace_id = %trace_id, step_name = "prefetch_skipped", "Prefetch skipped or empty");
                }
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
            let user_attachments = resolve_attachment_refs(&self.file_manager, &msg.media).await;
            let session = self.sessions.get_or_create(&session_key);
            let user_role = if is_cron_trigger { "system" } else { "user" };
            save_turn(
                session,
                &messages,
                history_len,
                user_role,
                &msg.content,
                user_attachments,
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
}

/// Build the current user message content from prompt text and attachment file IDs.
///
/// Image attachments become structured image parts; text and non-image attachments
/// keep the legacy inline/placeholder text behavior.
async fn assemble_current_message_content(
    file_manager: &FileManager,
    user_content: &str,
    file_ids: &[String],
) -> MessageContent {
    if file_ids.is_empty() {
        return MessageContent::Text(user_content.to_string());
    }

    let storage_path = dirs::data_local_dir()
        .map(|p| p.join("agent-diva").join("files"))
        .unwrap_or_else(|| PathBuf::from(".agent-diva/files"));
    info!("Loading attachments from: {}", storage_path.display());
    info!("File IDs to load: {:?}", file_ids);

    let mut attachment_text_parts = Vec::new();
    let mut image_parts = Vec::new();

    for file_id in file_ids {
        match file_manager.get(file_id).await {
            Ok(handle) => {
                let size = handle.metadata.size;
                let mime_type = handle
                    .metadata
                    .mime_type
                    .as_deref()
                    .unwrap_or("application/octet-stream");

                if mime_type.starts_with("image/") {
                    image_parts.push(MessageContentPart::ImageFile {
                        image_file: ImageFile {
                            file_id: handle.id.clone(),
                        },
                    });
                    continue;
                }

                if is_inline_text_mime(mime_type) && size <= MAX_INLINE_ATTACHMENT_SIZE {
                    match file_manager.read(&handle).await {
                        Ok(bytes) => match String::from_utf8(bytes) {
                            Ok(content) => {
                                attachment_text_parts.push(format!(
                                    "--- {} ---\n{}\n---",
                                    handle.metadata.name, content
                                ));
                            }
                            Err(_) => {
                                attachment_text_parts.push(format!(
                                    "[File: {} ({} bytes, binary)]",
                                    handle.metadata.name, size
                                ));
                            }
                        },
                        Err(e) => {
                            warn!("Failed to read file {}: {}", file_id, e);
                            attachment_text_parts.push(format!(
                                "[File: {} (error reading: {})]",
                                handle.metadata.name, e
                            ));
                        }
                    }
                } else {
                    attachment_text_parts.push(format!(
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
                attachment_text_parts
                    .push(format!("[Attachment: {} (not found - {})]", file_id, e));
            }
        }
    }

    let text_content = if attachment_text_parts.is_empty() {
        user_content.to_string()
    } else {
        format!(
            "{}\n\n[Attachments]\n{}\n[/Attachments]",
            user_content,
            attachment_text_parts.join("\n\n")
        )
    };

    if image_parts.is_empty() {
        MessageContent::Text(text_content)
    } else {
        let mut parts = Vec::with_capacity(image_parts.len() + 1);
        parts.push(MessageContentPart::Text { text: text_content });
        parts.extend(image_parts);
        MessageContent::Parts(parts)
    }
}

fn is_inline_text_mime(mime_type: &str) -> bool {
    mime_type.starts_with("text/")
        || mime_type == "application/json"
        || mime_type == "application/javascript"
        || mime_type == "application/typescript"
        || mime_type == "application/x-yaml"
        || mime_type == "application/xml"
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
    user_attachments: Option<Vec<FileAttachmentRef>>,
    final_content: &str,
) {
    // Save trigger message; cron-triggered turns are not real-time user input.
    match user_attachments {
        Some(attachments) => {
            session.add_full_message(ChatMessage::with_attachments(
                user_role,
                user_content,
                attachments,
            ));
        }
        None => session.add_message(user_role, user_content),
    }

    // Skip system prompt (1) + history (history_len) + current user message (1)
    let turn_start = 1 + history_len + 1;
    if turn_start < messages.len() {
        for m in &messages[turn_start..] {
            match m.role.as_str() {
                "assistant" => {
                    let content = m.content.to_text_lossy();
                    if content.trim().is_empty()
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
                        content,
                        None,
                        tool_calls_json,
                        None,
                    );
                    msg.reasoning_content = m.reasoning_content.clone();
                    msg.thinking_blocks = m.thinking_blocks.clone();
                    session.add_full_message(msg);
                }
                "tool" => {
                    let text_content = m.content.to_text_lossy();
                    let content = if text_content.chars().count() > 500 {
                        format!("{}...", text_content.chars().take(500).collect::<String>())
                    } else {
                        text_content
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

async fn resolve_attachment_refs(
    file_manager: &FileManager,
    file_ids: &[String],
) -> Option<Vec<FileAttachmentRef>> {
    if file_ids.is_empty() {
        return None;
    }

    let mut attachments = Vec::new();
    for file_id in file_ids {
        match file_manager.get(file_id).await {
            Ok(handle) => attachments.push(FileAttachmentRef::from_handle(&handle)),
            Err(e) => {
                warn!(
                    "Failed to resolve attachment metadata for {} while saving session: {}",
                    file_id, e
                );
            }
        }
    }

    if attachments.is_empty() {
        None
    } else {
        Some(attachments)
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

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_files::handle::FileMetadata;
    use agent_diva_files::FileConfig;

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

    #[test]
    fn test_save_turn_attaches_metadata_to_user_message_only() {
        let mut session = agent_diva_core::session::Session::new("gui:chat");
        let messages = vec![agent_diva_providers::Message::system("system")];
        let attachments = vec![FileAttachmentRef {
            file_id: "sha256:image123".to_string(),
            filename: "image.png".to_string(),
            mime_type: Some("image/png".to_string()),
            size: 4096,
        }];

        save_turn(
            &mut session,
            &messages,
            0,
            "user",
            "see attached",
            Some(attachments.clone()),
            "done",
        );

        assert_eq!(session.messages.len(), 2);
        assert_eq!(session.messages[0].role, "user");
        assert_eq!(session.messages[0].attachments, Some(attachments));
        assert_eq!(session.messages[1].role, "assistant");
        assert_eq!(session.messages[1].attachments, None);
    }

    #[tokio::test]
    async fn test_resolve_attachment_refs_reads_metadata_without_bytes() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_manager = FileManager::new(FileConfig::with_path(temp_dir.path()))
            .await
            .unwrap();
        let handle = file_manager
            .store(
                b"not persisted in session",
                FileMetadata {
                    name: "image.png".to_string(),
                    size: 24,
                    mime_type: Some("image/png".to_string()),
                    source: Some("gui".to_string()),
                    created_at: chrono::Utc::now(),
                    last_accessed_at: None,
                    preview: Some("preview should not be copied".to_string()),
                },
            )
            .await
            .unwrap();

        let refs = resolve_attachment_refs(&file_manager, &[handle.id.clone()])
            .await
            .unwrap();

        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].file_id, handle.id);
        assert_eq!(refs[0].filename, "image.png");
        assert_eq!(refs[0].mime_type, Some("image/png".to_string()));
        assert_eq!(refs[0].size, 24);

        let json = serde_json::to_string(&refs).unwrap();
        assert!(!json.contains("not persisted in session"));
        assert!(!json.contains("preview should not be copied"));
        assert!(!json.contains("base64"));
        assert!(!json.contains("bytes"));
    }

    #[tokio::test]
    async fn test_resolve_attachment_refs_skips_missing_files() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_manager = FileManager::new(FileConfig::with_path(temp_dir.path()))
            .await
            .unwrap();

        let refs = resolve_attachment_refs(&file_manager, &["sha256:missing".to_string()]).await;

        assert_eq!(refs, None);
    }

    #[tokio::test]
    async fn test_assemble_current_message_content_image_becomes_part() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_manager = FileManager::new(FileConfig::with_path(temp_dir.path()))
            .await
            .unwrap();
        let handle = file_manager
            .store(
                b"png bytes",
                FileMetadata {
                    name: "photo.png".to_string(),
                    size: 9,
                    mime_type: Some("image/png".to_string()),
                    source: Some("gui".to_string()),
                    created_at: chrono::Utc::now(),
                    last_accessed_at: None,
                    preview: None,
                },
            )
            .await
            .unwrap();

        let content =
            assemble_current_message_content(&file_manager, "describe this", &[handle.id.clone()])
                .await;

        match content {
            MessageContent::Parts(parts) => {
                assert_eq!(parts.len(), 2);
                assert_eq!(
                    parts[0],
                    MessageContentPart::Text {
                        text: "describe this".to_string()
                    }
                );
                assert_eq!(
                    parts[1],
                    MessageContentPart::ImageFile {
                        image_file: ImageFile { file_id: handle.id }
                    }
                );
            }
            other => panic!("expected structured parts, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_assemble_current_message_content_text_attachment_stays_text() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_manager = FileManager::new(FileConfig::with_path(temp_dir.path()))
            .await
            .unwrap();
        let handle = file_manager
            .store(
                b"hello from file",
                FileMetadata {
                    name: "note.txt".to_string(),
                    size: 15,
                    mime_type: Some("text/plain".to_string()),
                    source: Some("gui".to_string()),
                    created_at: chrono::Utc::now(),
                    last_accessed_at: None,
                    preview: None,
                },
            )
            .await
            .unwrap();

        let content =
            assemble_current_message_content(&file_manager, "read this", &[handle.id]).await;

        let text = content
            .as_text()
            .expect("text-only attachment should stay text");
        assert!(text.contains("read this"));
        assert!(text.contains("[Attachments]"));
        assert!(text.contains("--- note.txt ---"));
        assert!(text.contains("hello from file"));
        assert!(text.contains("[/Attachments]"));
    }

    #[tokio::test]
    async fn test_assemble_current_message_content_binary_attachment_keeps_placeholder() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_manager = FileManager::new(FileConfig::with_path(temp_dir.path()))
            .await
            .unwrap();
        let handle = file_manager
            .store(
                b"%PDF-1.7",
                FileMetadata {
                    name: "doc.pdf".to_string(),
                    size: 8,
                    mime_type: Some("application/pdf".to_string()),
                    source: Some("gui".to_string()),
                    created_at: chrono::Utc::now(),
                    last_accessed_at: None,
                    preview: None,
                },
            )
            .await
            .unwrap();

        let content =
            assemble_current_message_content(&file_manager, "inspect", &[handle.id]).await;

        let text = content
            .as_text()
            .expect("binary attachment should stay text");
        assert!(text.contains("doc.pdf"));
        assert!(text.contains("application/pdf"));
        assert!(text.contains("Use read_file tool to access"));
    }

    #[tokio::test]
    async fn test_assemble_current_message_content_missing_file_keeps_error_text() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_manager = FileManager::new(FileConfig::with_path(temp_dir.path()))
            .await
            .unwrap();

        let content = assemble_current_message_content(
            &file_manager,
            "check missing",
            &["sha256:missing".to_string()],
        )
        .await;

        let text = content
            .as_text()
            .expect("missing attachment should stay text");
        assert!(text.contains("check missing"));
        assert!(text.contains("[Attachment: sha256:missing (not found -"));
    }

    #[tokio::test]
    async fn test_assemble_current_message_content_mixed_attachments_share_user_message() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_manager = FileManager::new(FileConfig::with_path(temp_dir.path()))
            .await
            .unwrap();
        let text_handle = file_manager
            .store(
                b"alpha",
                FileMetadata {
                    name: "a.txt".to_string(),
                    size: 5,
                    mime_type: Some("text/plain".to_string()),
                    source: Some("gui".to_string()),
                    created_at: chrono::Utc::now(),
                    last_accessed_at: None,
                    preview: None,
                },
            )
            .await
            .unwrap();
        let image_handle = file_manager
            .store(
                b"image",
                FileMetadata {
                    name: "a.webp".to_string(),
                    size: 5,
                    mime_type: Some("image/webp".to_string()),
                    source: Some("gui".to_string()),
                    created_at: chrono::Utc::now(),
                    last_accessed_at: None,
                    preview: None,
                },
            )
            .await
            .unwrap();
        let binary_handle = file_manager
            .store(
                b"zip",
                FileMetadata {
                    name: "a.zip".to_string(),
                    size: 3,
                    mime_type: Some("application/zip".to_string()),
                    source: Some("gui".to_string()),
                    created_at: chrono::Utc::now(),
                    last_accessed_at: None,
                    preview: None,
                },
            )
            .await
            .unwrap();

        let content = assemble_current_message_content(
            &file_manager,
            "mixed",
            &[text_handle.id, image_handle.id.clone(), binary_handle.id],
        )
        .await;

        match content {
            MessageContent::Parts(parts) => {
                assert_eq!(parts.len(), 2);
                match &parts[0] {
                    MessageContentPart::Text { text } => {
                        assert!(text.contains("mixed"));
                        assert!(text.contains("--- a.txt ---"));
                        assert!(text.contains("alpha"));
                        assert!(text.contains("a.zip"));
                        assert!(text.contains("Use read_file tool to access"));
                        assert!(!text.contains("a.webp"));
                    }
                    other => panic!("expected text part first, got {:?}", other),
                }
                assert_eq!(
                    parts[1],
                    MessageContentPart::ImageFile {
                        image_file: ImageFile {
                            file_id: image_handle.id
                        }
                    }
                );
            }
            other => panic!("expected structured parts, got {:?}", other),
        }
    }
}
