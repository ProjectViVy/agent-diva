use super::AgentLoop;
use crate::consolidation;
use agent_diva_core::bus::{AgentEvent, InboundMessage, OutboundMessage, RunTelemetrySnapshotV0};
use agent_diva_providers::Message;
use agent_diva_swarm::{
    execute_full_swarm_convergence_loop, format_light_path_stop_for_user,
    load_swarm_prelude_config_from_workspace, resolve_execution_tier,
    sanitize_tool_summary_for_process_event, ConvergencePolicy, ExecutionTier, LightPathStopReason,
    PreludeInputSource, ProcessEventPipeline, ProcessEventV0, SwarmHandoffCheckpointV0,
    SwarmPreludeConfig, LIGHT_PATH_MAX_INTERNAL_STEPS, LIGHT_PATH_MAX_WALL_MS,
};
use agent_diva_core::session::ChatMessage;
use agent_diva_core::PersonSeamVisibility;
use agent_diva_core::soul::SoulStateStore;
use agent_diva_providers::{LLMResponse, LLMStreamEvent};
use futures::StreamExt;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, error, info, trace, warn};

struct SwarmRouteGuard {
    route: Arc<Mutex<Option<(String, String)>>>,
}

impl SwarmRouteGuard {
    fn arm(route: Arc<Mutex<Option<(String, String)>>>, channel: String, chat_id: String) -> Self {
        *route.lock().unwrap_or_else(|e| e.into_inner()) = Some((channel, chat_id));
        Self { route }
    }
}

impl Drop for SwarmRouteGuard {
    fn drop(&mut self) {
        *self.route.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
}

struct ProcessEventFlushGuard {
    pipe: Option<std::sync::Arc<agent_diva_swarm::ProcessEventPipeline>>,
}

impl Drop for ProcessEventFlushGuard {
    fn drop(&mut self) {
        if let Some(p) = self.pipe.take() {
            p.flush_pending();
        }
    }
}

impl AgentLoop {
    pub(super) async fn process_inbound_message_inner(
        &mut self,
        msg: InboundMessage,
        event_tx: Option<&mpsc::UnboundedSender<AgentEvent>>,
        trace_id: String,
    ) -> Result<Option<OutboundMessage>, Box<dyn std::error::Error>> {
        let _process_event_flush = ProcessEventFlushGuard {
            pipe: self.process_event_pipeline.clone(),
        };

        let _swarm_route_guard = self.process_event_pipeline.as_ref().map(|_| {
            SwarmRouteGuard::arm(
                self.swarm_process_route.clone(),
                msg.channel.clone(),
                msg.chat_id.clone(),
            )
        });

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

        // Get or create session
        let session_key = format!("{}:{}", msg.channel, msg.chat_id);
        self.clear_session_cancellation(&session_key);
        let session = self.sessions.get_or_create(&session_key);

        let is_cron_trigger = msg.sender_id == "cron" || msg.metadata.contains_key("cron_job_id");

        // Build initial messages
        let history = session.get_history(50); // Last 50 messages
        let history_len = history.len();
        let mut messages = self.context.build_messages(
            history,
            msg.content.clone(),
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

        let execution_tier = if is_cron_trigger {
            ExecutionTier::Light
        } else if let Some(pipe) = &self.process_event_pipeline {
            let cortex_enabled = pipe.cortex_runtime().snapshot().enabled;
            let explicit_full_swarm = msg
                .metadata
                .get("explicit_full_swarm")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            resolve_execution_tier(&msg.content, cortex_enabled, explicit_full_swarm)
        } else {
            ExecutionTier::Light
        };

        let mut prelude_llm_calls: u32 = 0;
        let mut prelude_swarm_phase_events: u32 = 0;
        if execution_tier == ExecutionTier::FullSwarm {
            if let Some(pipe) = &self.process_event_pipeline {
                match run_swarm_deliberation_prelude(
                    &self.provider,
                    &model_to_use,
                    &msg.content,
                    Some(pipe),
                    &self.workspace,
                )
                .await
                {
                    Ok((maybe_summary, llm, phases)) => {
                        prelude_llm_calls = llm;
                        prelude_swarm_phase_events = phases;
                        if let Some(prelude) = maybe_summary {
                            if let Some(pos) = messages.iter().rposition(|m| m.role == "user") {
                                messages.insert(pos, Message::system(prelude));
                            }
                        }
                    }
                    Err(e) => {
                        prelude_llm_calls = e.llm_calls;
                        prelude_swarm_phase_events = e.phase_events;
                        if let Some(cp) = e.checkpoint.as_ref() {
                            match serde_json::to_string(cp) {
                                Ok(checkpoint_json) => warn!(
                                    target: "agent_diva_agent::prelude",
                                    checkpoint_json = %checkpoint_json,
                                    error = %e,
                                    "Swarm deliberation prelude skipped; last successful handoff checkpoint"
                                ),
                                Err(_) => warn!(
                                    target: "agent_diva_agent::prelude",
                                    error = %e,
                                    "Swarm deliberation prelude skipped (checkpoint serialize failed)"
                                ),
                            }
                        } else {
                            warn!(
                                target: "agent_diva_agent::prelude",
                                error = %e,
                                "Swarm deliberation prelude skipped (no successful prelude step)"
                            );
                        }
                    }
                }
            }
        }

        // Agent loop
        let mut iteration = 0;
        let mut final_content: Option<String> = None;
        let mut final_reasoning: Option<String> = None;
        let mut soul_files_changed: HashSet<String> = HashSet::new();
        let light_turn_start = Instant::now();

        'agent_turn: while iteration < self.max_iterations {
            self.drain_runtime_control_commands().await;
            if self.is_session_cancelled(&session_key) {
                self.emit_error_event(&msg, event_tx, "Generation stopped by user.");
                return Ok(None);
            }

            iteration += 1;

            if execution_tier == ExecutionTier::Light {
                if iteration > LIGHT_PATH_MAX_INTERNAL_STEPS as usize {
                    let reason = LightPathStopReason::InternalStepLimit {
                        steps_used: LIGHT_PATH_MAX_INTERNAL_STEPS,
                    };
                    warn!(
                        trace_id = %trace_id,
                        iteration,
                        limit = LIGHT_PATH_MAX_INTERNAL_STEPS,
                        "Light path internal step budget exceeded"
                    );
                    final_content = Some(format_light_path_stop_for_user(reason));
                    break 'agent_turn;
                }
                let elapsed_ms = light_turn_start.elapsed().as_millis() as u64;
                if elapsed_ms >= LIGHT_PATH_MAX_WALL_MS {
                    let reason = LightPathStopReason::WallClockTimeout { elapsed_ms };
                    warn!(
                        trace_id = %trace_id,
                        elapsed_ms,
                        limit_ms = LIGHT_PATH_MAX_WALL_MS,
                        "Light path wall-clock budget exceeded"
                    );
                    final_content = Some(format_light_path_stop_for_user(reason));
                    break 'agent_turn;
                }
            }

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

            if let Some(pip) = &self.process_event_pipeline {
                pip.try_emit(ProcessEventV0::swarm_phase_changed(
                    format!("agent_iteration_{iteration}"),
                    format!("Agent iteration {iteration}/{}", self.max_iterations),
                ));
            }

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
            let tool_defs: Vec<_> = if execution_tier == ExecutionTier::FullSwarm {
                tool_defs
                    .into_iter()
                    .filter(|def| {
                        def.get("function")
                            .and_then(|f| f.get("name"))
                            .and_then(|n| n.as_str())
                            != Some("spawn")
                    })
                    .collect()
            } else {
                tool_defs
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

                if execution_tier == ExecutionTier::Light {
                    let elapsed_ms = light_turn_start.elapsed().as_millis() as u64;
                    if elapsed_ms >= LIGHT_PATH_MAX_WALL_MS {
                        let reason = LightPathStopReason::WallClockTimeout { elapsed_ms };
                        warn!(
                            trace_id = %trace_id,
                            elapsed_ms,
                            limit_ms = LIGHT_PATH_MAX_WALL_MS,
                            "Light path wall-clock budget exceeded during model stream"
                        );
                        final_content = Some(format_light_path_stop_for_user(reason));
                        break 'agent_turn;
                    }
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

                    if let Some(pip) = &self.process_event_pipeline {
                        let process_start_msg =
                            sanitize_tool_summary_for_process_event(preview.as_str(), 96);
                        pip.try_emit(ProcessEventV0::tool_call_started(
                            tool_call.name.clone(),
                            process_start_msg,
                            tool_call.id.clone(),
                        ));
                    }

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

                    if let Some(pip) = &self.process_event_pipeline {
                        let summary =
                            sanitize_tool_summary_for_process_event(result.as_str(), 96);
                        pip.try_emit(ProcessEventV0::tool_call_finished(
                            tool_call.name.clone(),
                            summary,
                            tool_call.id.clone(),
                        ));
                    }

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

        let full_swarm_convergence_rounds =
            if execution_tier == ExecutionTier::FullSwarm {
                if let Some(pip) = &self.process_event_pipeline {
                    let (_stop, rounds) = execute_full_swarm_convergence_loop(
                        &ConvergencePolicy::default(),
                        Some(pip.as_ref()),
                        |_| true,
                    );
                    Some(rounds)
                } else {
                    None
                }
            } else {
                None
            };

        let hit_iteration_cap = final_content.is_none();
        let iterations_u32 = u32::try_from(iteration).unwrap_or(u32::MAX);
        let main_loop_swarm_phase_events = self
            .process_event_pipeline
            .as_ref()
            .map(|_| iterations_u32)
            .unwrap_or(0);
        let telemetry = RunTelemetrySnapshotV0::from_live_agent_turn(
            prelude_llm_calls,
            prelude_swarm_phase_events,
            iterations_u32,
            main_loop_swarm_phase_events,
            hit_iteration_cap,
            full_swarm_convergence_rounds,
        );
        let telemetry_event = AgentEvent::RunTelemetry(telemetry);
        if let Some(tx) = event_tx {
            let _ = tx.send(telemetry_event.clone());
        }
        let _ = self.bus.publish_event(
            msg.channel.clone(),
            msg.chat_id.clone(),
            telemetry_event,
        );

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
                msg.person_seam,
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
}

/// Error from [`run_swarm_deliberation_prelude`] after partial progress (e.g. N-th role `chat` failed).
/// Carries telemetry-aligned counts so `RunTelemetry` matches `swarm_phase_changed` already emitted.
///
/// When at least one prelude `chat` succeeded, [`Self::checkpoint`] holds the last successful step
/// ([`SwarmHandoffCheckpointV0`], Story 5.3).
#[derive(Debug)]
struct PreludeRunError {
    llm_calls: u32,
    phase_events: u32,
    checkpoint: Option<SwarmHandoffCheckpointV0>,
    source: Box<dyn std::error::Error + Send + Sync>,
}

impl std::fmt::Display for PreludeRunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.source)
    }
}

impl std::error::Error for PreludeRunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.source()
    }
}

/// 蜂群全层路径上、主 ReAct 循环之前：按工作区 [`load_swarm_prelude_config_from_workspace`] 串行多角色对话，
/// 摘要注入系统上下文；**不**经 `spawn` 子代理工具。无配置文件时与阶段 A 两角色行为一致。
///
/// 每个角色的 `swarm_phase_changed` 在对应 `chat` **成功之后** 发射，避免失败 turn 上出现「有相位、无 LLM」与遥测不一致。
async fn run_swarm_deliberation_prelude(
    provider: &std::sync::Arc<dyn agent_diva_providers::LLMProvider>,
    model: &str,
    user_text: &str,
    pip: Option<&std::sync::Arc<ProcessEventPipeline>>,
    workspace: &Path,
) -> Result<(Option<String>, u32, u32), PreludeRunError> {
    let mut llm_calls: u32 = 0;
    let mut phase_events: u32 = 0;

    let cfg = load_swarm_prelude_config_from_workspace(workspace);
    if !cfg.enabled {
        return Ok((None, 0, 0));
    }

    if cfg.max_prelude_rounds == 0 {
        if let Some(p) = pip {
            p.try_emit(ProcessEventV0::swarm_phase_changed(
                "swarm_prelude_round_cap".to_string(),
                "max_prelude_rounds 为 0，蜂群序曲已跳过".to_string(),
            ));
            phase_events = phase_events.saturating_add(1);
        }
        return Ok((None, 0, phase_events));
    }

    let roles: Vec<_> = if cfg.roles.is_empty() {
        SwarmPreludeConfig::default().roles
    } else {
        cfg.roles.clone()
    };

    let max_steps = cfg.max_prelude_rounds as usize;
    let mut previous_output = String::new();
    let mut sections: Vec<(String, String)> = Vec::new();
    let mut last_checkpoint: Option<SwarmHandoffCheckpointV0> = None;

    for (i, role) in roles.iter().enumerate() {
        if i >= max_steps {
            if let Some(p) = pip {
                p.try_emit(ProcessEventV0::swarm_phase_changed(
                    "swarm_prelude_round_cap".to_string(),
                    format!(
                        "蜂群序曲已达 max_prelude_rounds={}，后续角色已跳过",
                        cfg.max_prelude_rounds
                    ),
                ));
                phase_events = phase_events.saturating_add(1);
            }
            break;
        }

        let user_payload = match role.input {
            PreludeInputSource::OriginalUser => user_text.to_string(),
            PreludeInputSource::PreviousOutput => {
                if previous_output.is_empty() {
                    user_text.to_string()
                } else {
                    previous_output.clone()
                }
            }
        };

        let tok = role.max_tokens.min(i32::MAX as u32) as i32;
        let resp = provider
            .chat(
                vec![
                    Message::system(role.system_prompt.clone()),
                    Message::user(user_payload),
                ],
                None,
                Some(model.to_string()),
                tok,
                role.temperature,
            )
            .await
            .map_err(|source| PreludeRunError {
                llm_calls,
                phase_events,
                checkpoint: last_checkpoint.clone(),
                source: source.into(),
            })?;
        llm_calls = llm_calls.saturating_add(1);
        if let Some(p) = pip {
            p.try_emit(ProcessEventV0::swarm_phase_changed(
                role.phase_id.clone(),
                role.phase_label.clone(),
            ));
            phase_events = phase_events.saturating_add(1);
        }
        previous_output = resp.content.unwrap_or_default();
        last_checkpoint = Some(SwarmHandoffCheckpointV0::from_successful_role_output(
            i as u32,
            role.phase_id.as_str(),
            &previous_output,
        ));

        if let Some(title) = &role.summary_section_title {
            sections.push((title.clone(), previous_output.clone()));
        } else if !previous_output.is_empty() {
            sections.push((String::new(), previous_output.clone()));
        }
    }

    if cfg.merge_phase.enabled {
        if let Some(p) = pip {
            p.try_emit(ProcessEventV0::swarm_phase_changed(
                cfg.merge_phase.phase_id.clone(),
                cfg.merge_phase.phase_label.clone(),
            ));
            phase_events = phase_events.saturating_add(1);
        }
    }

    if sections.is_empty() {
        return Ok((None, llm_calls, phase_events));
    }

    let mut body = String::new();
    for (title, content) in sections {
        if !body.is_empty() {
            body.push_str("\n\n");
        }
        if title.is_empty() {
            body.push_str(&content);
        } else {
            body.push_str(&title);
            body.push('\n');
            body.push_str(&content);
        }
    }

    Ok((
        Some(format!("{}\n{}", cfg.summary_preamble, body)),
        llm_calls,
        phase_events,
    ))
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
    trigger_person_seam: Option<PersonSeamVisibility>,
) {
    // Save trigger message; cron-triggered turns are not real-time user input.
    let mut trigger = ChatMessage::new(user_role, user_content);
    trigger.person_seam = trigger_person_seam;
    session.add_full_message(trigger);

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

    /// Story 6.6 / SWARM-MIG-02: subagent inbound uses Internal; persisted transcript must not surface raw payload.
    #[test]
    fn save_turn_subagent_trigger_internal_excluded_from_get_history() {
        use agent_diva_core::session::Session;

        let mut session = Session::new("gui:chat-1");
        let messages: Vec<Message> = vec![];
        save_turn(
            &mut session,
            &messages,
            0,
            "user",
            "[Subagent raw result blob]",
            "Here is a brief summary for you.",
            Some(PersonSeamVisibility::Internal),
        );

        assert_eq!(session.messages.len(), 2);
        assert_eq!(
            session.messages[0].person_seam,
            Some(PersonSeamVisibility::Internal)
        );
        assert_eq!(session.messages[0].content, "[Subagent raw result blob]");

        let hist = session.get_history(50);
        assert_eq!(hist.len(), 1);
        assert_eq!(hist[0].role, "assistant");
        assert_eq!(hist[0].content, "Here is a brief summary for you.");
    }

    #[tokio::test]
    async fn swarm_prelude_disabled_skips_llm() {
        use agent_diva_providers::{LLMProvider, LLMResponse, Message, ProviderResult};
        use async_trait::async_trait;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use tempfile::tempdir;

        struct CountingProvider {
            calls: Arc<AtomicUsize>,
        }
        #[async_trait]
        impl LLMProvider for CountingProvider {
            async fn chat(
                &self,
                _messages: Vec<Message>,
                _tools: Option<Vec<serde_json::Value>>,
                _model: Option<String>,
                _max_tokens: i32,
                _temperature: f64,
            ) -> ProviderResult<LLMResponse> {
                self.calls.fetch_add(1, Ordering::SeqCst);
                Ok(LLMResponse {
                    content: Some("ok".into()),
                    tool_calls: vec![],
                    finish_reason: "stop".into(),
                    usage: Default::default(),
                    reasoning_content: None,
                })
            }

            fn get_default_model(&self) -> String {
                "m".into()
            }
        }

        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join(agent_diva_swarm::SWARM_PRELUDE_FILE_TOML),
            "schema_version = 1\nenabled = false\n",
        )
        .unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let prov: Arc<dyn LLMProvider> = Arc::new(CountingProvider {
            calls: calls.clone(),
        });
        let out = run_swarm_deliberation_prelude(&prov, "m", "user", None, dir.path())
            .await
            .unwrap();
        assert!(out.0.is_none());
        assert_eq!(out.1, 0);
        assert_eq!(out.2, 0);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn swarm_prelude_no_file_matches_phase_a_two_llm_calls() {
        use agent_diva_providers::{LLMProvider, LLMResponse, Message, ProviderResult};
        use async_trait::async_trait;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use tempfile::tempdir;

        struct CountingProvider {
            calls: Arc<AtomicUsize>,
        }
        #[async_trait]
        impl LLMProvider for CountingProvider {
            async fn chat(
                &self,
                _messages: Vec<Message>,
                _tools: Option<Vec<serde_json::Value>>,
                _model: Option<String>,
                _max_tokens: i32,
                _temperature: f64,
            ) -> ProviderResult<LLMResponse> {
                self.calls.fetch_add(1, Ordering::SeqCst);
                Ok(LLMResponse {
                    content: Some("chunk".into()),
                    tool_calls: vec![],
                    finish_reason: "stop".into(),
                    usage: Default::default(),
                    reasoning_content: None,
                })
            }

            fn get_default_model(&self) -> String {
                "m".into()
            }
        }

        let dir = tempdir().unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let prov: Arc<dyn LLMProvider> = Arc::new(CountingProvider {
            calls: calls.clone(),
        });
        let out = run_swarm_deliberation_prelude(&prov, "m", "hello", None, dir.path())
            .await
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(out.1, 2, "prelude_llm_calls");
        assert_eq!(out.2, 0, "no process pipe → no swarm_phase_changed count");
        let text = out.0.expect("prelude");
        assert!(text.contains("【规划摘要】"));
        assert!(text.contains("【批评与补充】"));
        assert!(text.contains("chunk"));
    }

    #[tokio::test]
    async fn swarm_prelude_max_rounds_one_single_llm_call() {
        use agent_diva_providers::{LLMProvider, LLMResponse, Message, ProviderResult};
        use async_trait::async_trait;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use tempfile::tempdir;

        struct CountingProvider {
            calls: Arc<AtomicUsize>,
        }
        #[async_trait]
        impl LLMProvider for CountingProvider {
            async fn chat(
                &self,
                _messages: Vec<Message>,
                _tools: Option<Vec<serde_json::Value>>,
                _model: Option<String>,
                _max_tokens: i32,
                _temperature: f64,
            ) -> ProviderResult<LLMResponse> {
                self.calls.fetch_add(1, Ordering::SeqCst);
                Ok(LLMResponse {
                    content: Some("only".into()),
                    tool_calls: vec![],
                    finish_reason: "stop".into(),
                    usage: Default::default(),
                    reasoning_content: None,
                })
            }

            fn get_default_model(&self) -> String {
                "m".into()
            }
        }

        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join(agent_diva_swarm::SWARM_PRELUDE_FILE_TOML),
            "schema_version = 1\nenabled = true\nmax_prelude_rounds = 1\n",
        )
        .unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let prov: Arc<dyn LLMProvider> = Arc::new(CountingProvider {
            calls: calls.clone(),
        });
        let out = run_swarm_deliberation_prelude(&prov, "m", "u", None, dir.path())
            .await
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(out.1, 1);
        assert_eq!(out.2, 0);
    }

    /// Story 5.1 / code review：带 `ProcessEventPipeline` 时校验触顶与 merge 仍发射（产品选项 1）。
    #[tokio::test]
    async fn swarm_prelude_round_cap_emits_phases_when_pipeline_attached() {
        use agent_diva_providers::{LLMProvider, LLMResponse, Message, ProviderResult};
        use agent_diva_swarm::{
            recorder_sink, CortexRuntime, ProcessEventNameV0, ProcessEventPipeline,
            ProcessEventThrottleConfig,
        };
        use async_trait::async_trait;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use tempfile::tempdir;

        struct CountingProvider {
            calls: Arc<AtomicUsize>,
        }
        #[async_trait]
        impl LLMProvider for CountingProvider {
            async fn chat(
                &self,
                _messages: Vec<Message>,
                _tools: Option<Vec<serde_json::Value>>,
                _model: Option<String>,
                _max_tokens: i32,
                _temperature: f64,
            ) -> ProviderResult<LLMResponse> {
                self.calls.fetch_add(1, Ordering::SeqCst);
                Ok(LLMResponse {
                    content: Some("only".into()),
                    tool_calls: vec![],
                    finish_reason: "stop".into(),
                    usage: Default::default(),
                    reasoning_content: None,
                })
            }

            fn get_default_model(&self) -> String {
                "m".into()
            }
        }

        let cortex = Arc::new(CortexRuntime::new());
        let (rec, sink) = recorder_sink();
        let pipe = Arc::new(ProcessEventPipeline::new(
            cortex,
            sink,
            ProcessEventThrottleConfig::default(),
        ));

        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join(agent_diva_swarm::SWARM_PRELUDE_FILE_TOML),
            "schema_version = 1\nenabled = true\nmax_prelude_rounds = 1\n",
        )
        .unwrap();

        let calls = Arc::new(AtomicUsize::new(0));
        let prov: Arc<dyn LLMProvider> = Arc::new(CountingProvider {
            calls: calls.clone(),
        });
        let out = run_swarm_deliberation_prelude(&prov, "m", "u", Some(&pipe), dir.path())
            .await
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(out.1, 1);
        assert!(out.0.is_some(), "expected prelude summary text");

        pipe.flush_pending();
        let phase_ids: Vec<_> = rec
            .snapshot()
            .into_iter()
            .filter(|e| e.name == ProcessEventNameV0::SwarmPhaseChanged)
            .filter_map(|e| e.phase_id.clone())
            .collect();

        assert_eq!(
            phase_ids,
            vec![
                "swarm_peer_planner".to_string(),
                "swarm_prelude_round_cap".to_string(),
                "swarm_peer_merge".to_string(),
            ],
            "planner → round cap → merge (README: product semantics option 1)"
        );
    }

    /// Story 5.2 review：序曲中途 `chat` 失败时，已发射相位与 `prelude_llm_calls` / `phase_events` 一致。
    #[tokio::test]
    async fn swarm_prelude_second_chat_failure_partial_counts_match_pipeline() {
        use agent_diva_providers::{LLMProvider, LLMResponse, Message, ProviderError, ProviderResult};
        use agent_diva_swarm::{
            recorder_sink, CortexRuntime, ProcessEventNameV0, ProcessEventPipeline,
            ProcessEventThrottleConfig,
        };
        use async_trait::async_trait;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use tempfile::tempdir;

        struct FailOnSecond {
            calls: Arc<AtomicUsize>,
        }
        #[async_trait]
        impl LLMProvider for FailOnSecond {
            async fn chat(
                &self,
                _messages: Vec<Message>,
                _tools: Option<Vec<serde_json::Value>>,
                _model: Option<String>,
                _max_tokens: i32,
                _temperature: f64,
            ) -> ProviderResult<LLMResponse> {
                let n = self.calls.fetch_add(1, Ordering::SeqCst);
                if n == 0 {
                    Ok(LLMResponse {
                        content: Some("first".into()),
                        tool_calls: vec![],
                        finish_reason: "stop".into(),
                        usage: Default::default(),
                        reasoning_content: None,
                    })
                } else {
                    Err(ProviderError::ApiError("boom".into()))
                }
            }

            fn get_default_model(&self) -> String {
                "m".into()
            }
        }

        let cortex = Arc::new(CortexRuntime::new());
        let (rec, sink) = recorder_sink();
        let pipe = Arc::new(ProcessEventPipeline::new(
            cortex,
            sink,
            ProcessEventThrottleConfig::default(),
        ));

        let dir = tempdir().unwrap();
        // Default file absent → phase-A two roles; both attempted, second fails.
        let calls = Arc::new(AtomicUsize::new(0));
        let prov: Arc<dyn LLMProvider> = Arc::new(FailOnSecond {
            calls: calls.clone(),
        });
        let err = run_swarm_deliberation_prelude(&prov, "m", "hello", Some(&pipe), dir.path())
            .await
            .expect_err("second role chat fails");
        assert_eq!(err.llm_calls, 1);
        assert_eq!(err.phase_events, 1);

        let cp = err
            .checkpoint
            .as_ref()
            .expect("Story 5.3: last successful prelude step must yield a handoff checkpoint");
        assert_eq!(cp.role_id, "swarm_peer_planner");
        assert_eq!(cp.prelude_round_index, 0);
        assert!(
            cp.summary_preview.contains("first"),
            "preview should reflect first role output: {:?}",
            cp.summary_preview
        );
        assert_eq!(cp.schema_version, agent_diva_swarm::HANDOFF_CHECKPOINT_SCHEMA_VERSION_V0);

        pipe.flush_pending();
        let phase_count = rec
            .snapshot()
            .into_iter()
            .filter(|e| e.name == ProcessEventNameV0::SwarmPhaseChanged)
            .count();
        assert_eq!(phase_count, 1, "only successful first role emits phase");
    }
}
