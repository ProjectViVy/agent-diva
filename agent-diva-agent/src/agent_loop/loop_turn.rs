use super::{policy_phase_for, AgentLoop};
use crate::compaction::ContextCompactor;
use crate::consolidation;
use crate::context_budget::check_budget;
use crate::mask::ToolPolicy;
use crate::planning::builtin_tool_capability;
use agent_diva_core::audit::{self, AuditEvent};
use agent_diva_core::bus::{
    AgentEvent, InboundMessage, OutboundMessage, PlanRuntimeState, PlanRuntimeTodo, PokeEvent,
};
use agent_diva_core::memory::{PrefetchRequest, PrefetchStatus};
use agent_diva_core::planning::model::{PlanPhase, TodoStatus};
use agent_diva_core::planning::policy::{allows_for_phase, ToolCapability};
use agent_diva_core::planning::{
    normalize_report_markdown, report_validation_issues, resolve_plan_report_body,
    strip_proposed_plan_block, PlanRevisionAuthor,
};
use agent_diva_core::reasoning::ThinkingMode;
use agent_diva_core::security::{check_security, SecurityContext, SecurityDecision};
use agent_diva_core::session::{align_chat_history, ChatMessage, CompactTrigger, Session, TokenUsage};
use agent_diva_core::soul::SoulStateStore;
use agent_diva_core::token_ledger::budget::check_budget_at_path;
use agent_diva_core::token_ledger::{JsonlTokenLedger, TokenLedgerEntry};
use agent_diva_providers::{
    supports_vision_model, ImageUrl, LLMResponse, LLMStreamEvent, Message, MessageContent,
    MessageContentPart, ProviderError,
};
use agent_diva_tools::BackgroundTaskContext;
use anyhow;
use base64::Engine;
use futures::StreamExt;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, trace, warn};

/// Max size for text attachments to inline (100KB)
const MAX_INLINE_ATTACHMENT_SIZE: u64 = 100 * 1024;

#[derive(Debug, Default, Clone)]
struct ProcessedInboundMedia {
    prompt_text: String,
    image_parts: Vec<MessageContentPart>,
}

fn is_plan_mode(msg: &InboundMessage) -> bool {
    msg.metadata
        .get("exec_mode")
        .and_then(|value| value.as_str())
        .is_some_and(|mode| mode.eq_ignore_ascii_case("plan"))
}

fn fallback_session_title(session: &Session) -> Option<String> {
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

fn should_generate_session_title(session: &Session) -> bool {
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

fn build_current_turn_message(text: &str, image_parts: &[MessageContentPart]) -> Message {
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

fn replace_current_turn_message(messages: &mut Vec<Message>, current_turn_message: Message) {
    if let Some(last) = messages.last_mut() {
        *last = current_turn_message;
    } else {
        messages.push(current_turn_message);
    }
}

impl AgentLoop {
    fn emit_agent_event(
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

    async fn emit_planning_runtime_events(
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

    async fn generate_session_title_with_llm(
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

        let prompt = format!(
            "Generate a concise conversation title.\nReturn only the title with no quotes, no markdown, and no explanation.\nKeep it under 12 words.\n\nFirst user message:\n{}\n\nFirst assistant message:\n{}",
            first_user_message,
            first_assistant_message
        );
        let response = self
            .provider
            .chat(
                vec![
                    Message::system(
                        "You write short chat titles. Output only a short title with no punctuation wrapper.",
                    ),
                    Message::user(prompt),
                ],
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

    fn enforce_session_token_budget(
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
    ) -> Result<(), Box<dyn std::error::Error>> {
        if usage.total_tokens == 0 {
            return Ok(());
        }

        let ledger = JsonlTokenLedger::new(&self.token_ledger_data_root)?;
        ledger.append(TokenLedgerEntry::from_usage(session_key, model, usage))?;
        Ok(())
    }

    pub(super) async fn process_inbound_message_inner(
        &mut self,
        msg: InboundMessage,
        event_tx: Option<&mpsc::UnboundedSender<AgentEvent>>,
        trace_id: String,
    ) -> Result<Option<OutboundMessage>, Box<dyn std::error::Error>> {
        trace!(trace_id = %trace_id, step_name = "msg_received", "Message received from {}:{}", msg.channel, msg.sender_id);

        // Load the active mask first so it can influence model selection and
        // subagent defaults for this turn.
        let active_mask = self.load_active_mask();
        let model_to_use = self.effective_model_for_turn(active_mask.as_ref());
        self.subagent_manager
            .set_current_mask(active_mask.as_ref().map(|m| m.frontmatter.clone()))
            .await;

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
        let plan_mode = is_plan_mode(&msg);
        let mut execution_start = msg
            .metadata
            .get("execution_start")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        // Plan safety is a runtime lifecycle property, not only a UI/request mode.
        // Once a plan is waiting for approval, an agent-mode follow-up must not
        // re-enable mutation tools before the explicit approval transition.
        let active_plan = self.snapshot_active_plan_runtime().await;
        let session_key = format!("{}:{}", msg.channel, msg.chat_id);
        // Hydrate approved report execution (replacement plan runtime).
        let active_execution = match &self.tool_config.planning {
            Some(planning) if !plan_mode => planning
                .registry
                .active_execution_for_session(&session_key)
                .await
                ,
            _ => None,
        };
        // Kickoff turns that begin implementing an approved report.
        if active_execution.is_some() && !plan_mode {
            execution_start = execution_start
                || msg
                    .metadata
                    .get("approved_plan_markdown")
                    .and_then(|value| value.as_str())
                    .is_some()
                || msg.content.contains("Carry out the approved plan")
                || msg.content.contains("开始执行已批准");
        }
        let active_execution_id = active_execution.as_ref().map(|execution| execution.id.clone());
        // An active report execution session must not be blocked by a stale
        // legacy plan phase (e.g. AwaitingApproval left in the old store).
        let policy_phase = if active_execution_id.is_some() && !plan_mode {
            None
        } else {
            policy_phase_for(active_plan.as_ref(), plan_mode)
        };
        let plan_guard_active = policy_phase.is_some();
        let execution_context_policy = active_execution
            .as_ref()
            .map(|execution| execution.context_policy)
            .or_else(|| {
                msg.metadata
                    .get("execution_context_policy")
                    .and_then(|value| value.as_str())
                    .and_then(|policy| match policy {
                        "Clear" | "clear" => {
                            Some(agent_diva_core::planning::ExecutionContextPolicy::Clear)
                        }
                        "Retain" | "retain" => {
                            Some(agent_diva_core::planning::ExecutionContextPolicy::Retain)
                        }
                        "Compact" | "compact" => {
                            Some(agent_diva_core::planning::ExecutionContextPolicy::Compact)
                        }
                        _ => None,
                    })
            });
        let approved_plan_markdown = if let Some(markdown) = msg
            .metadata
            .get("approved_plan_markdown")
            .and_then(|value| value.as_str())
        {
            Some(markdown.to_string())
        } else if let (Some(planning), Some(execution)) =
            (&self.tool_config.planning, active_execution.as_ref())
        {
            planning.registry.execution_markdown(&execution.id).await
        } else {
            None
        };
        let background_task_context = BackgroundTaskContext {
            channel: Some(msg.channel.clone()),
            chat_id: Some(msg.chat_id.clone()),
            session_key: Some(session_key.clone()),
            trace_id: Some(trace_id.clone()),
            parent_run_id: msg
                .metadata
                .get("run_id")
                .or_else(|| msg.metadata.get("parent_run_id"))
                .and_then(|value| value.as_str())
                .map(str::to_string),
            token_budget_limit: self.session_token_budget_limit,
        };
        self.rebuild_tools_for_turn(
            active_mask.as_ref(),
            policy_phase.clone(),
            active_execution_id.clone(),
            Some(background_task_context.clone()),
        );

        // Process attachments first, then run the combined user-visible payload through
        // the security gate before any session or provider work starts.
        let processed_media = if msg.media.is_empty() {
            ProcessedInboundMedia::default()
        } else {
            self.load_attachment_contents(&msg.media).await?
        };
        let message_content = if processed_media.prompt_text.is_empty() {
            msg.content.clone()
        } else {
            format!(
                "{}\n\n[Attachments]\n{}\n[/Attachments]",
                msg.content, processed_media.prompt_text
            )
        };
        let security_context = SecurityContext {
            source_type: "channel".to_string(),
            workspace_id: Some(self.workspace.display().to_string()),
            run_id: None,
            channel_id: Some(msg.channel.clone()),
            tool_name: None,
        };
        let message_content = match check_security(&message_content, &security_context) {
            SecurityDecision::Allow => message_content,
            SecurityDecision::Sanitize { redacted, .. } => redacted,
            SecurityDecision::Block { reason, .. }
            | SecurityDecision::Quarantine { reason, .. } => {
                audit::emit(AuditEvent::ChannelMessageBlocked {
                    channel_id: msg.channel.clone(),
                    reason: reason.clone(),
                });
                return Err(
                    anyhow::anyhow!("Security policy blocked inbound message: {}", reason).into(),
                );
            }
        };
        if !processed_media.image_parts.is_empty() && !supports_vision_model(&model_to_use) {
            return Err(anyhow::anyhow!(
                "Current model `{}` does not support vision input. Switch to a vision-capable model such as `gpt-4o` or `gpt-4.1`.",
                model_to_use
            )
            .into());
        }
        let current_turn_message =
            build_current_turn_message(&message_content, &processed_media.image_parts);

        // Derive prefetch intent from raw user message before it's consumed.
        let prefetch_intent = derive_prefetch_intent(&message_content);
        let prefetch_user_message = message_content.clone();

        // Get or create session
        self.clear_session_cancellation(&session_key);

        // ── Build initial messages with budget-aware compaction ──
        // Phase 1: check budget and decide if compaction is needed (release borrow before .await)
        let (should_compact, budget_report) = {
            let session = self.sessions.get_or_create(&session_key);
            let history = session.get_history(50); // Last 50 messages

            // Budget check against context window limits
            let budget_config = self.tool_config.budget.clone();
            let budget_report = check_budget(&history, &budget_config);

            (budget_report.should_compact, budget_report)
        };

        // Phase 2: run best-effort auto compaction before any provider call.
        // The first provider request for this turn must see the post-compaction
        // session snapshot, not the pre-compaction history captured above.
        let (mut history, _history_len_before_policy, compaction_history, did_compact) =
            if should_compact {
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
                    ContextCompactor::compact(
                        session,
                        &budget_config,
                        provider,
                        &model,
                        CompactTrigger::Auto,
                        &session.compaction_history,
                    )
                    .await
                } else {
                    Err(anyhow::anyhow!("Session not found for compaction"))
                }
            };

            match compact_result {
                Ok(result) => {
                    let session = self.sessions.get_or_create(&session_key);
                    session.last_compacted = result.new_compacted_index;
                    session.compaction_history.push(result.summary);
                    let history = session.get_history(50);
                    let history_len = history.len();
                    (
                        history,
                        history_len,
                        session.compaction_history.clone(),
                        true,
                    )
                }
                Err(e) => {
                    warn!("Compaction failed (non-blocking): {}", e);
                    // Carry forward existing compaction history as fallback
                    let session = self.sessions.get_or_create(&session_key);
                    let history = session.get_history(50);
                    let history_len = history.len();
                    (
                        history,
                        history_len,
                        session.compaction_history.clone(),
                        false,
                    )
                }
            }
        } else {
            // Carry forward any existing compaction history from a previous turn
            let session = self.sessions.get_or_create(&session_key);
            let history = session.get_history(50);
            let history_len = history.len();
            (
                history,
                history_len,
                session.compaction_history.clone(),
                false,
            )
        };

        // Persist compaction state immediately when it just occurred
        if did_compact {
            if let Some(s) = self.sessions.get(&session_key) {
                if let Err(e) = self.sessions.save(s) {
                    error!("Failed to persist compaction state: {}", e);
                }
            }
        }

        if execution_start
            && matches!(
                execution_context_policy,
                Some(agent_diva_core::planning::ExecutionContextPolicy::Clear)
            )
        {
            history.clear();
        } else if execution_start
            && matches!(
                execution_context_policy,
                Some(agent_diva_core::planning::ExecutionContextPolicy::Compact)
            )
            && history.len() > 6
        {
            // Lightweight compact: keep the latest few turns plus the plan system note later.
            // Always re-align after tail truncation so we never open with an orphan
            // `tool` message (DeepSeek/OpenAI reject that shape with HTTP 400).
            let keep = 6.min(history.len());
            history = align_chat_history(history.split_off(history.len() - keep));
        }

        let mut messages = self.context.build_messages(
            history,
            message_content.clone(),
            Some(&msg.channel),
            Some(&msg.chat_id),
            &compaction_history,
        );
        if plan_guard_active {
            messages.insert(
                1,
                agent_diva_providers::Message::system(
                    "You are in Plan mode until the user leaves it. Explore with read-only tools only: do not modify files, run mutating shell commands, call planning/TODO tools, or begin implementation.\n\n\
When you have enough information for a complete plan, end the turn with exactly one line-oriented XML block (tags alone on their lines, tags untranslated):\n\
<proposed_plan>\n\
# short title\n\
## 目标\n\
...\n\
## 范围\n\
...\n\
## 计划步骤\n\
...\n\
## 风险与假设\n\
...\n\
## 验证方法\n\
...\n\
</proposed_plan>\n\n\
Preferred Markdown sections inside the block: 目标, 范围, 计划步骤, 风险与假设, 验证方法. Put any preface outside the tags. At most one <proposed_plan> per turn; revisions must be a full replacement. Do not ask whether to implement — the user uses the approval UI.",
                ),
            );
        }
        if let Some(markdown) = approved_plan_markdown.as_deref() {
            messages.insert(
                1,
                agent_diva_providers::Message::system(format!(
                    "You are implementing this approved plan. It remains authoritative throughout execution. Do not re-plan; execute and report results.\n\n{markdown}"
                )),
            );
        }
        // Drop any history-shaped orphans before the first provider call.
        crate::context::ContextBuilder::sanitize_messages_for_provider(&mut messages);
        // Prefix (system / plan notes / history / current user) ends here.
        // Agent-loop assistant/tool messages are appended after this index.
        let turn_messages_start = messages.len();
        if let Some(mask) = active_mask.as_ref() {
            if let Some(first) = messages.first_mut() {
                *first = agent_diva_providers::Message::system(
                    self.context.build_system_prompt(Some(mask)),
                );
            }
        }
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
        replace_current_turn_message(&mut messages, current_turn_message.clone());

        // Agent loop
        let mut iteration = 0;
        let mut final_content: Option<String> = None;
        let mut final_reasoning: Option<String> = None;
        let mut soul_files_changed: HashSet<String> = HashSet::new();
        let mut turn_token_usage: Option<TokenUsage> = None;
        // Codex-style follow-up: after tools, keep sampling until text or budget.
        let mut tool_run_summaries: Vec<ToolRunSummary> = Vec::new();
        let mut stopped_for_plan_approval = false;
        // One extra text-only sampling pass after tools hit max_iterations.
        let mut summary_bonus_remaining: usize = 0;
        let mut summary_only_nudge_injected = false;

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

        // Allow at most one bonus summary-only iteration after tools exhaust max_iterations.
        while iteration < self.max_iterations + summary_bonus_remaining {
            self.drain_runtime_control_commands().await;
            if self.is_session_cancelled(&session_key) {
                self.emit_error_event(&msg, event_tx, "Generation stopped by user.");
                return Ok(None);
            }

            iteration += 1;
            let summary_only_pass = iteration > self.max_iterations;
            if summary_only_pass {
                // Consume the single bonus pass budget.
                summary_bonus_remaining = 0;
            }
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
            let tool_defs: Vec<serde_json::Value> = if summary_only_pass {
                if !summary_only_nudge_injected {
                    messages.push(agent_diva_providers::Message::system(SUMMARY_ONLY_NUDGE));
                    summary_only_nudge_injected = true;
                }
                Vec::new()
            } else if msg.channel == "cron" || is_cron_trigger {
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
                self.enforce_session_token_budget(&session_key)?;
                let tool_defs_for_call = if !tool_defs.is_empty() {
                    Some(tool_defs.clone())
                } else {
                    None
                };
                crate::context::ContextBuilder::sanitize_messages_for_provider(&mut messages);
                match self
                    .provider
                    .chat_stream(
                        messages.clone(),
                        tool_defs_for_call,
                        if summary_only_pass { agent_diva_providers::ToolChoiceMode::Disabled } else if tool_defs.is_empty() { agent_diva_providers::ToolChoiceMode::Unspecified } else { agent_diva_providers::ToolChoiceMode::Auto },
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
                                        session,
                                        &budget_config,
                                        provider,
                                        &model,
                                        CompactTrigger::Reactive,
                                        &session.compaction_history,
                                    )
                                    .await
                                } else {
                                    Err(anyhow::anyhow!(
                                        "Session not found for reactive compaction"
                                    ))
                                }
                            };

                            // Phase 2: apply result and get updated history
                            let (reactive_history, reactive_compaction_history) =
                                match compact_result {
                                    Ok(result) => {
                                        let session = self.sessions.get_or_create(&session_key);
                                        session.last_compacted = result.new_compacted_index;
                                        session.compaction_history.push(result.summary);
                                        let history = session.get_history(50);
                                        (history, session.compaction_history.clone())
                                    }
                                    Err(e) => {
                                        warn!("Reactive compaction failed (non-blocking): {}", e);
                                        // Fallback: use existing session state without updating
                                        let session = self.sessions.get_or_create(&session_key);
                                        let history = session.get_history(50);
                                        (history, session.compaction_history.clone())
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

                            // Rebuild messages with new compaction history
                            messages = self.context.build_messages(
                                reactive_history,
                                message_content.clone(),
                                Some(&msg.channel),
                                Some(&msg.chat_id),
                                &reactive_compaction_history,
                            );
                            if let Some(markdown) = approved_plan_markdown.as_deref() {
                                messages.insert(
                                    1,
                                    agent_diva_providers::Message::system(format!(
                                        "You are implementing this approved plan. It remains authoritative throughout execution. Do not re-plan; execute and report results.\n\n{markdown}"
                                    )),
                                );
                            }
                            if is_cron_trigger {
                                let current_message = messages.pop();
                                messages.push(agent_diva_providers::Message::system(
                                    "This turn is triggered automatically by a scheduled cron job, not by a real-time user input. Do not schedule new reminders/jobs from this turn unless explicitly required by prior task design.",
                                ));
                                if let Some(current_message) = current_message {
                                    messages.push(current_message);
                                }
                            }
                            replace_current_turn_message(
                                &mut messages,
                                current_turn_message.clone(),
                            );
                            crate::context::ContextBuilder::sanitize_messages_for_provider(
                                &mut messages,
                            );

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
            let mut output_guard = InternalProtocolGuard::default();
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
                        if let Some(safe_delta) = output_guard.push(delta) {
                            let event = AgentEvent::AssistantDelta { text: safe_delta };
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

            let protocol_leak_detected = output_guard.detected()
                || response
                    .content
                    .as_deref()
                    .is_some_and(contains_internal_protocol);

            // `push` retains a short suffix to detect protocol markers split
            // across provider chunks. Once the response is known to be safe,
            // publish that suffix so the streamed UI receives the full reply.
            if !protocol_leak_detected {
                if let Some(safe_tail) = output_guard.finish() {
                    let event = AgentEvent::AssistantDelta { text: safe_tail };
                    if let Some(tx) = event_tx {
                        let _ = tx.send(event.clone());
                    }
                    let _ = self
                        .bus
                        .publish_event(msg.channel.clone(), msg.chat_id.clone(), event);
                }
            }

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
            if let Err(e) =
                self.append_session_token_usage(&session_key, &model_to_use, &iter_usage)
            {
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
                    let planning_before = self.snapshot_active_plan_runtime().await;
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

                    let (result, is_error) = match serde_json::to_value(&tool_call.arguments) {
                        Ok(mut params_value) => {
                            let read_only_rejected = active_mask
                                .as_ref()
                                .is_some_and(ToolPolicy::is_read_only_mode)
                                && !ToolPolicy::is_read_only_tool(&tool_call.name);
                            // Active report execution must not be re-gated by a
                            // stale legacy plan phase mid-turn.
                            let policy_phase = if active_execution_id.is_some() && !plan_mode {
                                None
                            } else {
                                policy_phase_for(planning_before.as_ref(), plan_mode)
                            };
                            let capability = builtin_tool_capability(&tool_call.name);
                            let plan_mode_rejected = policy_phase
                                .as_ref()
                                .is_some_and(|phase| !allows_for_phase(phase, capability))
                                || (plan_guard_active
                                    && planning_before.is_none()
                                    && !matches!(
                                        capability,
                                        ToolCapability::Inspect | ToolCapability::PlanningRecord
                                    ));

                            if read_only_rejected {
                                (
                                    format!(
                                        "Error: tool '{}' is disabled in reviewer read-only mode",
                                        tool_call.name
                                    ),
                                    true,
                                )
                            } else if plan_mode_rejected {
                                (format!(
                                    "Error: tool '{}' is denied by the active plan capability policy.",
                                    tool_call.name,
                                ), true)
                            } else {
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
                                    ("Error: cron tool is disabled during cron-triggered execution to prevent recursive scheduling".to_string(), true)
                                } else {
                                    match self.tools.execute(&tool_call.name, params_value).await {
                                        Ok(text) => (text, false),
                                        Err(e) => (format!("Error: {}", e), true),
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            warn!(
                                "Failed to serialize arguments for tool '{}' (call_id: {}): {}",
                                tool_call.name, tool_call.id, e
                            );
                            (
                                format!(
                                    "Error: failed to serialize arguments for tool '{}': {}",
                                    tool_call.name, e
                                ),
                                true,
                            )
                        }
                    };
                    if self.notify_on_soul_change && !is_error {
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

                    let planning_after = if !is_error {
                        self.snapshot_active_plan_runtime().await
                    } else {
                        None
                    };

                    let event = AgentEvent::ToolCallFinished {
                        name: tool_call.name.clone(),
                        is_error,
                        result: result.clone(),
                        call_id: tool_call.id.clone(),
                    };
                    if let Some(tx) = event_tx {
                        let _ = tx.send(event.clone());
                    }
                    let _ = self
                        .bus
                        .publish_event(msg.channel.clone(), msg.chat_id.clone(), event);
                    if !is_error {
                        self.emit_planning_runtime_events(
                            &msg,
                            event_tx,
                            &tool_call.name,
                            planning_before.clone(),
                            planning_after.clone(),
                        )
                        .await;
                        if planning_before
                            .as_ref()
                            .map(|plan| (plan.phase.clone(), plan.revision))
                            != planning_after
                                .as_ref()
                                .map(|plan| (plan.phase.clone(), plan.revision))
                        {
                            let rebuild_phase = if active_execution_id.is_some() && !plan_mode {
                                None
                            } else {
                                policy_phase_for(planning_after.as_ref(), plan_mode)
                            };
                            self.rebuild_tools_for_turn(
                                active_mask.as_ref(),
                                rebuild_phase,
                                active_execution_id.clone(),
                                Some(background_task_context.clone()),
                            );
                        }
                        if matches!(tool_call.name.as_str(), "plan_submit" | "plan_transition")
                            && planning_after
                                .as_ref()
                                .is_some_and(|plan| plan.phase == PlanPhase::AwaitingApproval)
                        {
                            stop_after_tool_call = true;
                        }
                    }
                    tool_run_summaries.push(ToolRunSummary {
                        name: tool_call.name.clone(),
                        ok: !is_error,
                        detail: truncate_for_tool_summary(&result, 120),
                    });
                    self.context.add_tool_result(
                        &mut messages,
                        tool_call.id.clone(),
                        tool_call.name.clone(),
                        result,
                    );
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
                if iteration >= self.max_iterations && summary_bonus_remaining == 0 {
                    summary_bonus_remaining = 1;
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
                final_content = response
                    .content
                    .filter(|s| !s.trim().is_empty());
                final_reasoning = response.reasoning_content;
                // Honor thinking mode: Off clears reasoning, Auto/On pass through
                if self.thinking_mode == ThinkingMode::Off {
                    final_reasoning = None;
                }
                break;
            }
        }

        let mut final_content = final_content.unwrap_or_else(|| {
            resolve_empty_final_content(stopped_for_plan_approval, &tool_run_summaries)
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

        // Plan mode: demux a proposed plan into a durable report artifact so the
        // GUI can show PlanApprovalCard. Prefer <proposed_plan> tags (Codex-style);
        // fall back to freeform plan-like text. Section completeness is soft.
        if plan_mode {
            if let Some(planning) = &self.tool_config.planning {
                if let Some(extracted) = resolve_plan_report_body(&final_content) {
                    let markdown = normalize_report_markdown(&extracted.markdown);
                    let title = markdown
                        .lines()
                        .find_map(|line| line.trim().strip_prefix("# "))
                        .unwrap_or("Plan report");
                    let soft_issues = report_validation_issues(&markdown);
                    match planning
                        .registry
                        .create_report(&session_key, title, &markdown, PlanRevisionAuthor::Agent)
                        .await
                    {
                        Ok(report) => {
                            // Keep chat bubble short; the approval card owns the plan body.
                            if extracted.tagged {
                                final_content = strip_proposed_plan_block(&final_content);
                            } else {
                                // Freeform plans are the whole reply — do not leave a
                                // second incomplete surface in the message list.
                                final_content.clear();
                            }
                            if final_content.trim().is_empty() {
                                final_content =
                                    "已生成计划报告，请在下方审批卡片中查看并批准。"
                                        .to_string();
                            }
                            if !soft_issues.is_empty() {
                                let missing: Vec<String> =
                                    soft_issues.iter().map(|issue| issue.to_string()).collect();
                                final_content.push_str(&format!(
                                    "\n\n> 计划已提交审批，但章节仍不完整（{}）。可直接批准，或点编辑继续完善。",
                                    missing.join("；")
                                ));
                            }
                            self.emit_agent_event(
                                &msg,
                                event_tx,
                                AgentEvent::PlanReportReadyForApproval { report },
                            );
                        }
                        Err(error) => warn!(%error, "failed to persist plan report"),
                    }
                }
            }
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
            let user_role = if is_cron_trigger || execution_start {
                "system"
            } else {
                "user"
            };
            save_turn(
                session,
                &messages,
                turn_messages_start,
                user_role,
                &message_content,
                &final_content,
                turn_token_usage,
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

        let title_update = if let Some(session) = self.sessions.get(&session_key) {
            if should_generate_session_title(session) {
                let fallback = fallback_session_title(session);
                let generated = self
                    .generate_session_title_with_llm(session, &model_to_use)
                    .await;
                generated
                    .clone()
                    .or(fallback)
                    .map(|title| (title, generated.is_some()))
            } else {
                None
            }
        } else {
            None
        };

        if let Some((title, generated)) = title_update {
            let session = self.sessions.get_or_create(&session_key);
            session.set_conversation_title(Some(title));
            session.set_title_generated(generated);
            if !session.title_manually_set() {
                session.set_title_manually_set(false);
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

fn changed_soul_file(
    tool_name: &str,
    arguments: &HashMap<String, serde_json::Value>,
    _result: &str,
) -> Option<&'static str> {
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
#[allow(clippy::too_many_arguments)]
fn save_turn(
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

/// Compact record of a tool execution for empty-final synthesis.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ToolRunSummary {
    name: String,
    ok: bool,
    detail: String,
}

const SUMMARY_ONLY_NUDGE: &str = "You already executed tools in this turn. Based on the tool results above, write a concise final reply for the user in their language. Do not call any tools.";

const INTERNAL_PROTOCOL_MARKERS: &[&str] = &[
    "<｜DSML｜tool_calls>",
    "<｜DSML｜invoke",
    "<｜DSML｜parameter",
    "<｜｜DSML｜｜tool_calls>",
    "<｜｜DSML｜｜invoke",
    "<｜｜DSML｜｜parameter",
    "<tool_calls>",
    "<function_calls>",
];

/// Prevent internal tool protocols from reaching either streaming or final UI output.
/// A short suffix is withheld so a marker split over stream chunks cannot leak.
#[derive(Default)]
struct InternalProtocolGuard {
    pending: String,
    detected: bool,
}

impl InternalProtocolGuard {
    fn push(&mut self, delta: String) -> Option<String> {
        if self.detected {
            return None;
        }
        self.pending.push_str(&delta);
        if contains_internal_protocol(&self.pending) {
            self.detected = true;
            self.pending.clear();
            return None;
        }

        const RETAINED_CHARS: usize = 32;
        let chars: Vec<char> = self.pending.chars().collect();
        if chars.len() <= RETAINED_CHARS {
            return None;
        }
        let split_at = chars.len() - RETAINED_CHARS;
        let safe: String = chars[..split_at].iter().collect();
        self.pending = chars[split_at..].iter().collect();
        Some(safe)
    }

    fn detected(&self) -> bool {
        self.detected
    }

    /// Returns the withheld safe suffix after the provider stream ends.
    fn finish(&mut self) -> Option<String> {
        if self.detected || self.pending.is_empty() {
            return None;
        }
        Some(std::mem::take(&mut self.pending))
    }
}

fn contains_internal_protocol(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    INTERNAL_PROTOCOL_MARKERS
        .iter()
        .any(|marker| lower.contains(&marker.to_ascii_lowercase()))
}

fn synthesize_iteration_limit_summary(summaries: &[ToolRunSummary], pending_tools: &[&str]) -> String {
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
const FALLBACK_PLAN_APPROVAL_ZH: &str =
    "计划已提交审批，请在下方审批卡片中查看并批准。";

fn truncate_for_tool_summary(text: &str, max_chars: usize) -> String {
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
        assert!(contains_internal_protocol("<｜｜DSML｜｜invoke name=\"write_file\">"));
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
    fn test_changed_soul_file_ignores_non_write_tools() {
        let args = HashMap::from([(
            "path".to_string(),
            serde_json::Value::String("SOUL.md".to_string()),
        )]);
        // Non-write_file/edit_file tools should return None regardless of result
        assert_eq!(
            changed_soul_file("list_dir", &args, "Successfully listed"),
            None
        );
        assert_eq!(changed_soul_file("read_file", &args, "content"), None);
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
