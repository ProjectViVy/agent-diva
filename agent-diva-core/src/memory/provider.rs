//! Provider boundary for Agent-Diva long-memory integration.
//!
//! The trait in this module is owned by `agent-diva-core` so Agent-Diva can
//! depend on a stable domain contract without importing transport-specific
//! CLI, MCP, or HTTP shapes. This matches the consuming-boundary pattern used
//! by Laputa's adapter layer and keeps long-memory ownership outside prompt
//! assembly and loop execution code.

use std::path::PathBuf;

/// Deterministic status for startup wakeup injection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupStatus {
    /// Fresh startup content was produced for the current wakeup.
    Ready,
    /// Startup continuity could not be assembled. `last_usable_wakeup` stays
    /// `None` when no cache is reused, which is the current default policy.
    Degraded {
        reason: String,
        last_usable_wakeup: Option<SystemPromptBlock>,
    },
}

/// Startup wakeup result consumed by prompt assembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemPromptResponse {
    pub status: StartupStatus,
    pub prompt_block: Option<SystemPromptBlock>,
}

impl SystemPromptResponse {
    #[must_use]
    pub fn ready(prompt_block: SystemPromptBlock) -> Self {
        Self {
            status: StartupStatus::Ready,
            prompt_block: Some(prompt_block),
        }
    }

    #[must_use]
    pub fn degraded(reason: impl Into<String>) -> Self {
        let reason = reason.into();
        Self {
            prompt_block: Some(SystemPromptBlock {
                shape: StartupInjectionShape::CompactRenderedMarkdown,
                markdown: render_degraded_startup_markdown(&reason),
            }),
            status: StartupStatus::Degraded {
                reason,
                last_usable_wakeup: None,
            },
        }
    }
}

/// Explicit shape chosen for startup injection into prompt assembly.
///
/// Agent-Diva currently consumes a compact rendered markdown block at the
/// prompt seam instead of a transport envelope or raw backend payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupInjectionShape {
    /// Compact rendered markdown ready for direct prompt inclusion.
    CompactRenderedMarkdown,
}

/// Deterministic status for intent-aware prefetch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrefetchStatus {
    /// No actionable intent was available, so recall was intentionally skipped.
    SkippedNoIntent,
    /// Recall completed and yielded a prompt block or an empty-but-successful result.
    Ready,
    /// Recall was attempted but failed.
    Failed { reason: String },
}

/// Deterministic status for post-turn synchronization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncTurnStatus {
    /// At least one durable write completed successfully.
    Persisted,
    /// No durable write was needed for this turn.
    Noop,
    /// A write was attempted but did not complete successfully.
    Failed { reason: String },
}

/// Deterministic status for session-end shutdown handling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEndStatus {
    /// Shutdown hook ran and triggered work.
    Triggered,
    /// Shutdown hook intentionally performed no work.
    Noop,
    /// This session-end call was already handled and is idempotently ignored.
    AlreadyHandled,
    /// Shutdown work failed.
    Failed { reason: String },
}

/// Minimal provider-facing summary of a wakeup-style state bundle.
///
/// This mirrors the durable, domain-oriented content Agent-Diva needs from a
/// Laputa-style wakeup without depending on Laputa crate types directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WakeupPackSummary {
    pub identity: String,
    pub recent_state: String,
    pub latest_capsule: Option<String>,
    pub key_relations: Vec<String>,
    pub unresolved_threads: Vec<String>,
    pub generated_at: Option<String>,
}

/// Provider-facing representation of a startup-relevant rhythm signal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RhythmTrigger {
    pub name: String,
    pub reason: Option<String>,
}

/// Structured startup support data that can be rendered into the chosen
/// provider-consumable prompt block shape.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StartupContextSnapshot {
    /// Optional `.laputa` state root associated with the current session.
    pub laputa_state_root: Option<PathBuf>,
    /// Optional rendered SOUL projection markdown.
    pub soul_markdown: Option<String>,
    /// Optional rendered WAKEUP projection markdown.
    pub wakeup_markdown: Option<String>,
    /// Optional structured wakeup summary when markdown projections are not
    /// already available.
    pub wakeup_pack: Option<WakeupPackSummary>,
    /// Optional rhythm signals relevant at startup.
    pub rhythm_triggers: Vec<RhythmTrigger>,
    /// Optional fallback block from existing Agent-Diva core outputs.
    pub memory_markdown: Option<String>,
}

impl StartupContextSnapshot {
    /// Render structured startup data into the explicit prompt seam shape.
    pub fn into_system_prompt_block(self) -> Option<SystemPromptBlock> {
        let markdown = self.render_compact_markdown();
        if markdown.is_empty() {
            None
        } else {
            Some(SystemPromptBlock {
                shape: StartupInjectionShape::CompactRenderedMarkdown,
                markdown,
            })
        }
    }

    fn render_compact_markdown(&self) -> String {
        let mut sections = Vec::new();

        if let Some(memory_markdown) = trimmed_markdown(self.memory_markdown.as_deref()) {
            sections.push(memory_markdown.to_string());
        }

        if let Some(soul_markdown) = trimmed_markdown(self.soul_markdown.as_deref()) {
            sections.push(format!("## Soul Projection\n{}", soul_markdown));
        }

        if let Some(wakeup_markdown) = trimmed_markdown(self.wakeup_markdown.as_deref()) {
            sections.push(format!("## Wakeup Projection\n{}", wakeup_markdown));
        } else if let Some(wakeup_pack) = &self.wakeup_pack {
            sections.push(render_wakeup_pack_summary(wakeup_pack));
        }

        if !self.rhythm_triggers.is_empty() {
            let triggers = self
                .rhythm_triggers
                .iter()
                .map(|trigger| match trigger.reason.as_deref() {
                    Some(reason) if !reason.trim().is_empty() => {
                        format!("- {} — {}", trigger.name.trim(), reason.trim())
                    }
                    _ => format!("- {}", trigger.name.trim()),
                })
                .collect::<Vec<_>>()
                .join("\n");
            sections.push(format!("## Rhythm Signals\n{}", triggers));
        }

        sections.join("\n\n")
    }
}

fn render_degraded_startup_markdown(reason: &str) -> String {
    format!(
        "## Memory Startup Status\n- status: degraded\n- reason: {}\n- last_usable_wakeup: omitted (no cache reuse)\n",
        reason.trim()
    )
}

fn trimmed_markdown(markdown: Option<&str>) -> Option<&str> {
    let markdown = markdown?.trim();
    if markdown.is_empty() {
        None
    } else {
        Some(markdown)
    }
}

fn render_wakeup_pack_summary(pack: &WakeupPackSummary) -> String {
    let latest_capsule = pack.latest_capsule.as_deref().unwrap_or("None");
    let key_relations = if pack.key_relations.is_empty() {
        "- None".to_string()
    } else {
        pack.key_relations
            .iter()
            .map(|item| format!("- {}", item.trim()))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let unresolved_threads = if pack.unresolved_threads.is_empty() {
        "- None".to_string()
    } else {
        pack.unresolved_threads
            .iter()
            .map(|item| format!("- {}", item.trim()))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let mut rendered = String::from("## Wakeup Summary");
    if let Some(generated_at) = trimmed_markdown(pack.generated_at.as_deref()) {
        rendered.push_str("\nGenerated: ");
        rendered.push_str(generated_at);
    }
    rendered.push_str("\n\n### Identity\n");
    rendered.push_str(pack.identity.trim());
    rendered.push_str("\n\n### Recent State\n");
    rendered.push_str(pack.recent_state.trim());
    rendered.push_str("\n\n### Latest Capsule\n");
    rendered.push_str(latest_capsule.trim());
    rendered.push_str("\n\n### Key Relations\n");
    rendered.push_str(&key_relations);
    rendered.push_str("\n\n### Unresolved Threads\n");
    rendered.push_str(&unresolved_threads);
    rendered
}

/// Input for startup wakeup-style prompt generation.
///
/// This request maps to the `turn_start` side of D-002 and the
/// `laputa_wakeup` / `laputa_project_soul` style boundary described by D-010.
/// Implementations should return only the markdown block Agent-Diva needs to
/// splice into the system prompt, not transport envelopes or backend rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemPromptRequest {
    /// Workspace root for the active agent session.
    pub workspace_root: PathBuf,
}

/// Startup block injected into the Agent-Diva system prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemPromptBlock {
    /// Explicitly chosen startup injection shape consumed by prompt assembly.
    pub shape: StartupInjectionShape,
    /// Markdown content suitable for direct prompt inclusion.
    pub markdown: String,
}

/// Input for optional intent-aware recall during a live turn.
///
/// This represents the D-010 `laputa_recall_intent(intent, current_room)`
/// shape at the Agent-Diva boundary. The contract stays domain-oriented: it
/// carries the inferred intent and room context, not CLI flags or route names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefetchRequest {
    /// Workspace root for the active agent session.
    pub workspace_root: PathBuf,
    /// Intent inferred from the current turn.
    pub intent: String,
    /// Optional current room or topic context.
    pub current_room: Option<String>,
    /// Optional user message or distilled query text.
    pub user_message: Option<String>,
}

/// Optional memory material returned for mid-turn recall.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefetchResponse {
    /// Deterministic prefetch outcome for consumers.
    pub status: PrefetchStatus,
    /// Markdown block that can be injected into turn context when available.
    pub prompt_block: Option<String>,
}

impl Default for PrefetchResponse {
    fn default() -> Self {
        Self {
            status: PrefetchStatus::SkippedNoIntent,
            prompt_block: None,
        }
    }
}

/// Input for post-successful-turn synchronization.
///
/// This is the Agent-Diva side of D-002 `sync_turn(events)` and checklist
/// items 5-7. The first contract version is intentionally small: it can carry
/// distilled memory updates and a persisted history/evidence entry without
/// leaking backend-specific write commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncTurnRequest {
    /// Workspace root for the active agent session.
    pub workspace_root: PathBuf,
    /// Optional full replacement or refreshed long-memory markdown.
    pub memory_update_markdown: Option<String>,
    /// Optional history/evidence line derived from the completed turn.
    pub history_entry: Option<String>,
}

/// Result of turn synchronization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncTurnResponse {
    /// Deterministic sync outcome for consumers.
    pub status: SyncTurnStatus,
}

impl Default for SyncTurnResponse {
    fn default() -> Self {
        Self {
            status: SyncTurnStatus::Noop,
        }
    }
}

/// Input for session shutdown rhythm handling.
///
/// This maps to the `on_session_end()` hook requested by checklist item 5.6,
/// which is intentionally broader than a specific scheduler or transport call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionEndRequest {
    /// Workspace root for the active agent session.
    pub workspace_root: PathBuf,
    /// Optional Agent-Diva session identifier.
    pub session_id: Option<String>,
}

/// Result of session shutdown handling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionEndResponse {
    /// Deterministic shutdown outcome for consumers.
    pub status: SessionEndStatus,
}

impl Default for SessionEndResponse {
    fn default() -> Self {
        Self {
            status: SessionEndStatus::Noop,
        }
    }
}

/// Isolation layer between Agent-Diva and any long-memory backend.
///
/// Contract rules:
/// - `system_prompt_block()` is synchronous because prompt assembly in
///   `ContextBuilder::build_system_prompt()` is synchronous today.
/// - `prefetch()`, `sync_turn()`, and `on_session_end()` are async because
///   live-turn recall, post-turn persistence, and shutdown rhythm work may
///   require I/O and already sit on async paths in Agent-Diva.
/// - All request/response types are Agent-Diva-owned domain structs; do not
///   leak MCP schemas, CLI arguments, HTTP routes, or backend model types.
#[async_trait::async_trait]
pub trait MemoryProvider: Send + Sync {
    /// Build the startup memory block for system prompt assembly.
    fn system_prompt_block(
        &self,
        request: &SystemPromptRequest,
    ) -> crate::Result<SystemPromptResponse>;

    /// Perform optional intent-aware prefetch for a live turn.
    async fn prefetch(&self, request: PrefetchRequest) -> crate::Result<PrefetchResponse>;

    /// Persist evidence after a successful turn completes.
    async fn sync_turn(&self, request: SyncTurnRequest) -> crate::Result<SyncTurnResponse>;

    /// Trigger shutdown/session-end rhythm work if needed.
    async fn on_session_end(&self, request: SessionEndRequest)
        -> crate::Result<SessionEndResponse>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyProvider;

    #[async_trait::async_trait]
    impl MemoryProvider for DummyProvider {
        fn system_prompt_block(
            &self,
            request: &SystemPromptRequest,
        ) -> crate::Result<SystemPromptResponse> {
            Ok(SystemPromptResponse::ready(SystemPromptBlock {
                shape: StartupInjectionShape::CompactRenderedMarkdown,
                markdown: format!("workspace={}", request.workspace_root.display()),
            }))
        }

        async fn prefetch(&self, request: PrefetchRequest) -> crate::Result<PrefetchResponse> {
            Ok(PrefetchResponse {
                status: PrefetchStatus::Ready,
                prompt_block: Some(format!(
                    "intent={} room={}",
                    request.intent,
                    request.current_room.unwrap_or_else(|| "none".to_string())
                )),
            })
        }

        async fn sync_turn(&self, request: SyncTurnRequest) -> crate::Result<SyncTurnResponse> {
            Ok(SyncTurnResponse {
                status: if request.memory_update_markdown.is_some()
                    || request.history_entry.is_some()
                {
                    SyncTurnStatus::Persisted
                } else {
                    SyncTurnStatus::Noop
                },
            })
        }

        async fn on_session_end(
            &self,
            request: SessionEndRequest,
        ) -> crate::Result<SessionEndResponse> {
            Ok(SessionEndResponse {
                status: if request.session_id.is_some() {
                    SessionEndStatus::Triggered
                } else {
                    SessionEndStatus::Noop
                },
            })
        }
    }

    #[tokio::test]
    async fn test_memory_provider_contract_is_domain_only() {
        let provider = DummyProvider;
        let prompt = provider
            .system_prompt_block(&SystemPromptRequest {
                workspace_root: PathBuf::from("/tmp/diva"),
            })
            .unwrap()
            .prompt_block
            .expect("dummy provider should return a prompt block");
        assert!(prompt.markdown.contains("workspace=/tmp/diva"));
        assert_eq!(prompt.shape, StartupInjectionShape::CompactRenderedMarkdown);

        let prefetch = provider
            .prefetch(PrefetchRequest {
                workspace_root: PathBuf::from("/tmp/diva"),
                intent: "recall-project-status".to_string(),
                current_room: Some("roadmap".to_string()),
                user_message: Some("what changed?".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(prefetch.status, PrefetchStatus::Ready);
        assert_eq!(
            prefetch.prompt_block.as_deref(),
            Some("intent=recall-project-status room=roadmap")
        );

        let sync = provider
            .sync_turn(SyncTurnRequest {
                workspace_root: PathBuf::from("/tmp/diva"),
                memory_update_markdown: Some("updated".to_string()),
                history_entry: None,
            })
            .await
            .unwrap();
        assert_eq!(sync.status, SyncTurnStatus::Persisted);

        let shutdown = provider
            .on_session_end(SessionEndRequest {
                workspace_root: PathBuf::from("/tmp/diva"),
                session_id: Some("session-42".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(shutdown.status, SessionEndStatus::Triggered);
    }

    #[test]
    fn test_startup_context_snapshot_renders_compact_markdown() {
        let block = StartupContextSnapshot {
            laputa_state_root: Some(PathBuf::from("/tmp/diva/.laputa")),
            soul_markdown: Some("# Identity\n\nGenerated soul".to_string()),
            wakeup_markdown: None,
            wakeup_pack: Some(WakeupPackSummary {
                identity: "You are Diva.".to_string(),
                recent_state: "- roadmap: Hot (heat: 5)".to_string(),
                latest_capsule: Some("Weekly review complete.".to_string()),
                key_relations: vec!["maintainer <-> roadmap".to_string()],
                unresolved_threads: vec!["ship provider boundary".to_string()],
                generated_at: Some("2026-05-08 10:00 UTC".to_string()),
            }),
            rhythm_triggers: vec![RhythmTrigger {
                name: "weekly".to_string(),
                reason: Some("capsule due".to_string()),
            }],
            memory_markdown: Some("## Long-term Memory\nExisting durable memory".to_string()),
        }
        .into_system_prompt_block()
        .expect("startup context should render a prompt block");

        assert_eq!(block.shape, StartupInjectionShape::CompactRenderedMarkdown);
        assert!(block.markdown.contains("## Long-term Memory"));
        assert!(block.markdown.contains("## Soul Projection"));
        assert!(block.markdown.contains("## Wakeup Summary"));
        assert!(block.markdown.contains("## Rhythm Signals"));
        assert!(block.markdown.contains("weekly — capsule due"));
    }

    #[test]
    fn test_degraded_startup_explicitly_omits_cached_wakeup() {
        let response = SystemPromptResponse::degraded("wakeup generation failed");

        match response.status {
            StartupStatus::Degraded {
                reason,
                last_usable_wakeup,
            } => {
                assert_eq!(reason, "wakeup generation failed");
                assert!(last_usable_wakeup.is_none());
            }
            other => panic!("expected degraded startup, got {other:?}"),
        }

        let block = response
            .prompt_block
            .expect("degraded startup should still emit an explicit prompt block");
        assert!(block.markdown.contains("status: degraded"));
        assert!(block.markdown.contains("last_usable_wakeup: omitted"));
    }
}
