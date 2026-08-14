//! Context builder for assembling prompts

use crate::context_assembly::{
    render_stable_prefix, serialize_dynamic_sections, CacheBreakReason, ContextSection,
    PromptSection, StablePrefixSnapshot,
};
use crate::mask::MaskFile;
use crate::mask::MaskPromptComposer;
use crate::memory_boundary::default_memory_provider;
use crate::skills::SkillsLoader;
use agent_diva_core::memory::{
    MemoryProvider, StartupInjectionShape, StartupStatus, SystemPromptBlock, SystemPromptRequest,
    SystemPromptResponse,
};
use agent_diva_laputa::{capture_frozen_core_for_session, DEFAULT_FROZEN_CORE_BUDGET};
use agent_diva_providers::{DynamicContextTransport, Message};
use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tracing::warn;

mod prompt;

const DEFAULT_AGENT_NAME: &str = "agent-diva";
const DEFAULT_AGENT_EMOJI: &str = "🐈";
const DEFAULT_AGENT_ROLE: &str = "helpful AI assistant";
static CONTEXT_BUILDER_ID: AtomicU64 = AtomicU64::new(1);

/// Character budget for workspace markdown files injected into the prompt
/// (e.g. AGENTS.md).
const WORKSPACE_MD_MAX_CHARS: usize = 4000;

#[derive(Default)]
struct StableContextCache {
    sessions: HashMap<String, SessionStableCache>,
}

struct SessionStableCache {
    sections: BTreeMap<ContextSection, PromptSection>,
    rendered: String,
    mask: Option<MaskFile>,
    memory_revision: u64,
    prefix_version: u64,
    pending_invalidations: BTreeMap<ContextSection, CacheBreakReason>,
}

/// Builds the context for LLM requests
pub struct ContextBuilder {
    workspace: PathBuf,
    persona_root: PathBuf,
    skills_loader: SkillsLoader,
    memory_provider: Arc<dyn MemoryProvider>,
    default_session_key: String,
    stable_cache: Mutex<StableContextCache>,
}

impl ContextBuilder {
    /// Create a new context builder
    pub fn new(workspace: PathBuf) -> Self {
        let skills_loader = SkillsLoader::new(&workspace, None);
        let memory_provider = default_memory_provider(&workspace);
        Self {
            persona_root: workspace.clone(),
            workspace,
            skills_loader,
            memory_provider,
            default_session_key: next_builder_session_key(),
            stable_cache: Mutex::new(StableContextCache::default()),
        }
    }

    /// Create a new context builder with skills
    pub fn with_skills(workspace: PathBuf, builtin_skills_dir: Option<PathBuf>) -> Self {
        let skills_loader = SkillsLoader::new(&workspace, builtin_skills_dir);
        let memory_provider = default_memory_provider(&workspace);
        Self {
            persona_root: workspace.clone(),
            workspace,
            skills_loader,
            memory_provider,
            default_session_key: next_builder_session_key(),
            stable_cache: Mutex::new(StableContextCache::default()),
        }
    }

    /// Point Persona/Frozen Core reads at the machine-wide config root.
    pub fn with_persona_root(mut self, persona_root: PathBuf) -> Self {
        self.persona_root = persona_root;
        self
    }

    /// Override the memory provider boundary used for prompt assembly.
    pub fn with_memory_provider(mut self, memory_provider: Arc<dyn MemoryProvider>) -> Self {
        self.clear_session_caches();
        self.memory_provider = memory_provider;
        self
    }

    /// Build system prompt from workspace files and memory.
    ///
    /// When a non-default `mask` is provided, its body is injected at the top
    /// of the prompt (before the identity header) — 方案 A placement.
    pub fn build_system_prompt(&self, mask: Option<&MaskFile>) -> String {
        self.build_system_prompt_for_session(mask, &self.default_session_key)
    }

    /// Build a prompt against the immutable Frozen Core capture for one
    /// concrete session key.
    pub fn build_system_prompt_for_session(
        &self,
        mask: Option<&MaskFile>,
        session_key: &str,
    ) -> String {
        self.stable_prefix_snapshot_for_session(mask, session_key)
            .rendered
    }

    /// Build typed stable-prefix sections for one concrete session.
    pub fn build_prompt_sections_for_session(
        &self,
        mask: Option<&MaskFile>,
        session_key: &str,
    ) -> Vec<PromptSection> {
        self.stable_prefix_snapshot_for_session(mask, session_key)
            .sections
    }

    /// Return the current stable-prefix cache snapshot for one session.
    pub fn stable_prefix_snapshot_for_session(
        &self,
        mask: Option<&MaskFile>,
        session_key: &str,
    ) -> StablePrefixSnapshot {
        let memory_request = SystemPromptRequest {
            workspace_root: self.workspace.clone(),
        };
        let memory_revision = self.memory_provider.system_prompt_revision(&memory_request);
        let mut stable_cache = self
            .stable_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        match stable_cache.sessions.entry(session_key.to_string()) {
            Entry::Vacant(entry) => {
                let sections = self.build_all_stable_sections(mask, session_key);
                let rendered = render_stable_sections(&sections);
                entry.insert(SessionStableCache {
                    sections: sections
                        .iter()
                        .cloned()
                        .map(|section| (section.section, section))
                        .collect(),
                    rendered: rendered.clone(),
                    mask: mask.cloned(),
                    memory_revision: self.memory_provider.system_prompt_revision(&memory_request),
                    prefix_version: 1,
                    pending_invalidations: BTreeMap::new(),
                });
                StablePrefixSnapshot {
                    sections,
                    rendered,
                    prefix_version: 1,
                }
            }
            Entry::Occupied(mut entry) => {
                let cache = entry.get_mut();
                if cache.mask.as_ref() != mask {
                    cache.pending_invalidations.insert(
                        ContextSection::MaskAndIdentity,
                        CacheBreakReason::MaskChanged,
                    );
                }
                if cache.memory_revision != memory_revision {
                    cache.pending_invalidations.insert(
                        ContextSection::MemoryPolicyAndIndex,
                        CacheBreakReason::L1HotRefresh,
                    );
                }

                if !cache.pending_invalidations.is_empty() {
                    for section in cache.sections.values_mut() {
                        section.cache_break_reason = None;
                    }
                    let invalidations = std::mem::take(&mut cache.pending_invalidations);
                    for (section, reason) in invalidations {
                        if let Some(rebuilt) = self.build_stable_section(section, mask, session_key)
                        {
                            cache
                                .sections
                                .insert(section, rebuilt.with_cache_break_reason(reason));
                        }
                    }
                    cache.mask = mask.cloned();
                    cache.memory_revision =
                        self.memory_provider.system_prompt_revision(&memory_request);
                    cache.prefix_version = cache.prefix_version.saturating_add(1);
                    let sections = cache.sections.values().cloned().collect::<Vec<_>>();
                    cache.rendered = render_stable_sections(&sections);
                }

                let sections = cache.sections.values().cloned().collect::<Vec<_>>();
                StablePrefixSnapshot {
                    rendered: cache.rendered.clone(),
                    sections,
                    prefix_version: cache.prefix_version,
                }
            }
        }
    }

    /// Re-read AGENTS.md for an already-captured session on its next assembly.
    pub fn invalidate_agent_rules(&self, session_key: &str) {
        self.invalidate_cached_section(
            session_key,
            ContextSection::AgentRulesAndSkills,
            CacheBreakReason::AgentRulesReload,
        );
    }

    /// Re-read the skill catalog for an already-captured session on its next assembly.
    pub fn invalidate_skills(&self, session_key: &str) {
        self.invalidate_cached_section(
            session_key,
            ContextSection::AgentRulesAndSkills,
            CacheBreakReason::SkillsReload,
        );
    }

    /// Re-read the workspace skill catalog for every Session already cached by
    /// this builder on its next context assembly. The invalidation is lazy:
    /// no prompt is rebuilt and no Session history is touched here.
    pub fn invalidate_skills_for_all_sessions(&self) -> usize {
        let mut stable_cache = self
            .stable_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut invalidated = 0;
        for cache in stable_cache.sessions.values_mut() {
            cache.pending_invalidations.insert(
                ContextSection::AgentRulesAndSkills,
                CacheBreakReason::SkillsReload,
            );
            invalidated += 1;
        }
        invalidated
    }

    /// Mark every stable section for recapture after a session reset.
    pub fn reset_session_cache(&self, session_key: &str) {
        let mut stable_cache = self
            .stable_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(cache) = stable_cache.sessions.get_mut(session_key) else {
            return;
        };
        for section in [
            ContextSection::MaskAndIdentity,
            ContextSection::FrozenCore,
            ContextSection::AgentRulesAndSkills,
            ContextSection::MemoryPolicyAndIndex,
        ] {
            cache
                .pending_invalidations
                .insert(section, CacheBreakReason::SessionReset);
        }
    }

    /// Drop all stable state owned by a completed or deleted session.
    pub fn end_session_cache(&self, session_key: &str) {
        let mut stable_cache = self
            .stable_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        stable_cache.sessions.remove(session_key);
    }

    /// Drop all stable section snapshots owned by this builder.
    pub fn clear_session_caches(&self) {
        let mut stable_cache = self
            .stable_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        stable_cache.sessions.clear();
    }

    fn invalidate_cached_section(
        &self,
        session_key: &str,
        section: ContextSection,
        reason: CacheBreakReason,
    ) {
        let mut stable_cache = self
            .stable_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(cache) = stable_cache.sessions.get_mut(session_key) {
            cache.pending_invalidations.insert(section, reason);
        }
    }

    fn build_all_stable_sections(
        &self,
        mask: Option<&MaskFile>,
        session_key: &str,
    ) -> Vec<PromptSection> {
        [
            ContextSection::MaskAndIdentity,
            ContextSection::FrozenCore,
            ContextSection::AgentRulesAndSkills,
            ContextSection::MemoryPolicyAndIndex,
        ]
        .into_iter()
        .filter_map(|section| self.build_stable_section(section, mask, session_key))
        .collect()
    }

    fn build_stable_section(
        &self,
        section: ContextSection,
        mask: Option<&MaskFile>,
        session_key: &str,
    ) -> Option<PromptSection> {
        match section {
            ContextSection::MaskAndIdentity => Some(self.build_mask_and_identity_section(mask)),
            ContextSection::FrozenCore => Some(self.build_frozen_core_section(session_key)),
            ContextSection::AgentRulesAndSkills => {
                Some(self.build_agent_rules_and_skills_section())
            }
            ContextSection::MemoryPolicyAndIndex => {
                Some(self.build_memory_policy_and_index_section())
            }
            _ => None,
        }
    }

    fn build_mask_and_identity_section(&self, mask: Option<&MaskFile>) -> PromptSection {
        let workspace_path = self.workspace.display();
        let identity_header = self.load_identity_header();

        let mut identity = String::new();

        // 方案 A: mask prompt at the very top when active.
        if let Some(mask_prompt) = MaskPromptComposer::compose(mask) {
            identity.push_str(&mask_prompt);
            identity.push_str("\n\n");
        }

        identity.push_str(&format!(
            r#"{identity_header}

You have access to tools that allow you to:
- Read, write, and edit files
- Execute shell commands
- Search the web and fetch web pages
- Send messages to users on chat channels
- Ask the user structured questions and wait for their answer (ask_user)
- Schedule reminders and recurring jobs (cron)
- Track the current task with the lightweight `update_plan` TODO/progress checklist in normal chat

`update_plan` is a TODO/checklist tool, not Plan mode and not the repository's durable `TODOLIST.md`. Use it for complex, multi-step, or ambiguous tasks, and when the user asks for steps or progress tracking. Keep the checklist current as work advances: statuses are `pending`, `in_progress`, and `completed`; keep at most one item `in_progress`, and mark every item `completed` when the task is done. Do not use it for trivial one-step work or repeat the full checklist in chat after the tool call. Formal planning and approval use Plan mode; execution-session TODOs use their dedicated tools.

## Workspace
Your workspace is at: {workspace_path}
- Applied authority is consumed through the configured MemoryProvider boundary.
- Legacy authority files are compatibility/migration inputs only, not default authority."#
        ));

        PromptSection::new(ContextSection::MaskAndIdentity, identity)
    }

    fn build_frozen_core_section(&self, session_key: &str) -> PromptSection {
        let mut frozen = String::new();
        // Frozen Core projection: captured once per session (first assembly)
        // and frozen afterwards; sits under the mask overlay, above all other
        // context layers.
        let frozen_core = capture_frozen_core_for_session(&self.persona_root, session_key);
        let frozen_projection = frozen_core.render(DEFAULT_FROZEN_CORE_BUDGET);
        if !frozen_projection.is_empty() {
            frozen.push_str(&frozen_projection);
        }
        PromptSection::new(ContextSection::FrozenCore, frozen)
    }

    fn build_agent_rules_and_skills_section(&self) -> PromptSection {
        let mut rules_and_skills = String::new();
        self.append_agent_rules(&mut rules_and_skills);

        // Skills - progressive loading
        // 1) Always-loaded skills (full content)
        let always_skills = self.skills_loader.get_always_skills();
        if !always_skills.is_empty() {
            let always_content = self.skills_loader.load_skills_for_context(&always_skills);
            if !always_content.is_empty() {
                self.append_section(&mut rules_and_skills, "Active Skills", &always_content);
            }
        }

        // 2) Available skills summary
        let skills_summary = self.skills_loader.build_skills_summary();
        if !skills_summary.is_empty() {
            self.append_section(
                &mut rules_and_skills,
                "Skills",
                "The following skills extend your capabilities. To use a skill, read its SKILL.md file using the read_file tool.\n",
            );
            rules_and_skills
                .push_str("Skills with available=\"false\" need dependencies installed first.\n\n");
            rules_and_skills.push_str(&skills_summary);
        }

        PromptSection::new(
            ContextSection::AgentRulesAndSkills,
            rules_and_skills.trim_start().to_string(),
        )
    }

    fn build_memory_policy_and_index_section(&self) -> PromptSection {
        let mut memory = String::new();
        // Inject L0 memory management policy before the memory projection.
        self.append_section(
            &mut memory,
            "Memory Management Policy",
            agent_diva_core::memory::L0_MEMORY_POLICY,
        );

        // Inject long-term memory if available
        let memory_context = self
            .memory_provider
            .system_prompt_block(&SystemPromptRequest {
                workspace_root: self.workspace.clone(),
            })
            .map(render_startup_injection)
            .unwrap_or_else(|err| render_provider_error_startup_injection(&err.to_string()));
        if !memory_context.is_empty() {
            memory.push_str("\n\n");
            memory.push_str(&memory_context);
        }

        memory.push_str(
            r#"

IMPORTANT: When responding to direct questions or conversations, reply directly with your text response.
Only use the 'message' tool when you need to send a message to a specific chat channel.
For normal conversation, just respond with text - do not call the message tool.
When the task needs user input to continue — preference research, ambiguity resolution, or trade-off decisions — use the 'ask_user' tool to ask a structured question and wait for the answer, instead of listing questions as plain text.
When a user asks to create a reminder, timer, or recurring schedule, use the 'cron' tool instead of saying the feature is unavailable.

Always be helpful, accurate, and concise. When using tools, explain what you're doing."#,
        );

        memory.push_str(
            "\nWhen the user asks you to remember something, use the memory_add tool; to forget, use memory_remove; to recall, use memory_search or memory_list. Writes report one of: applied (durable), proposal_created (awaiting review, contains a proposal id), or failed. High-risk changes (updating or removing existing memory) create reviewable proposals and are not effective until approved. Never write arbitrary files as if they were memory authority; legacy authority files are compatibility inputs only, not default prompt authority.",
        );

        PromptSection::new(
            ContextSection::MemoryPolicyAndIndex,
            memory.trim_start().to_string(),
        )
    }

    /// Build the per-call time and channel metadata outside the stable prefix.
    pub fn build_volatile_meta_section(
        &self,
        channel: Option<&str>,
        chat_id: Option<&str>,
    ) -> PromptSection {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M (%A)");
        self.build_volatile_meta_section_at(channel, chat_id, &now.to_string())
    }

    /// Deterministic variant used by cache-stability tests.
    pub fn build_volatile_meta_section_at(
        &self,
        channel: Option<&str>,
        chat_id: Option<&str>,
        now: &str,
    ) -> PromptSection {
        let mut body = format!("## Current Time\n{now}");
        if let (Some(channel), Some(chat_id)) = (channel, chat_id) {
            body.push_str(&format!(
                "\n\n## Current Session\nChannel: {channel}\nChat ID: {chat_id}"
            ));
        }
        PromptSection::new(ContextSection::VolatileMeta, body)
    }

    fn append_agent_rules(&self, prompt: &mut String) {
        if let Some(content) = self.read_workspace_markdown("AGENTS.md") {
            self.append_section(prompt, "Agent Rules", &content);
        }
    }

    fn read_workspace_markdown(&self, rel: &str) -> Option<String> {
        let path = self.workspace.join(rel);
        read_trimmed_markdown(&path, WORKSPACE_MD_MAX_CHARS)
    }

    fn append_section(&self, prompt: &mut String, title: &str, content: &str) {
        prompt.push_str("\n\n## ");
        prompt.push_str(title);
        prompt.push('\n');
        prompt.push_str(content);
    }

    fn load_identity_header(&self) -> String {
        default_identity_header()
    }

    /// Build the complete message list for an LLM call.
    ///
    /// Inject at most one canonical checkpoint before the active history.
    pub fn build_messages(
        &self,
        history: Vec<agent_diva_core::session::ChatMessage>,
        current_message: String,
        channel: Option<&str>,
        chat_id: Option<&str>,
        canonical_checkpoint: Option<&agent_diva_core::session::CanonicalCheckpoint>,
    ) -> Vec<Message> {
        self.build_messages_for_session(
            history,
            current_message,
            channel,
            chat_id,
            canonical_checkpoint,
            &self.default_session_key,
        )
    }

    pub fn build_messages_for_session(
        &self,
        history: Vec<agent_diva_core::session::ChatMessage>,
        current_message: String,
        channel: Option<&str>,
        chat_id: Option<&str>,
        canonical_checkpoint: Option<&agent_diva_core::session::CanonicalCheckpoint>,
        session_key: &str,
    ) -> Vec<Message> {
        let mut messages = self.build_prefix_messages_for_session(
            history,
            canonical_checkpoint,
            session_key,
            None,
        );
        let volatile = self.build_volatile_meta_section(channel, chat_id);
        if let Ok(Some(message)) =
            serialize_dynamic_sections(&[volatile], DynamicContextTransport::UserContextEnvelope)
        {
            messages.push(message);
        }
        messages.push(Message::user(current_message));
        messages
    }

    /// Build stable system, one canonical checkpoint, and raw history without
    /// volatile context or the current user message.
    pub(crate) fn build_prefix_messages_for_session(
        &self,
        history: Vec<agent_diva_core::session::ChatMessage>,
        canonical_checkpoint: Option<&agent_diva_core::session::CanonicalCheckpoint>,
        session_key: &str,
        mask: Option<&MaskFile>,
    ) -> Vec<Message> {
        let snapshot = self.stable_prefix_snapshot_for_session(mask, session_key);
        self.build_prefix_messages_from_snapshot(history, canonical_checkpoint, &snapshot)
    }

    pub(crate) fn build_prefix_messages_from_snapshot(
        &self,
        history: Vec<agent_diva_core::session::ChatMessage>,
        canonical_checkpoint: Option<&agent_diva_core::session::CanonicalCheckpoint>,
        snapshot: &StablePrefixSnapshot,
    ) -> Vec<Message> {
        let mut messages = vec![Message::system(snapshot.rendered.clone())];

        if let Some(checkpoint) = canonical_checkpoint {
            messages.push(Message::system(format!(
                "{}\n{}",
                prompt::CANONICAL_CHECKPOINT_HINT,
                checkpoint.render_for_context()
            )));
        }

        // History - convert from ChatMessage to Message
        for msg in history {
            let message = match msg.role.as_str() {
                "user" => Message::user(&msg.content),
                "assistant" => {
                    let mut m = Message::assistant(&msg.content);
                    // Restore tool_calls from session history
                    if let Some(ref tc_values) = msg.tool_calls {
                        match serde_json::from_value::<Vec<agent_diva_providers::ToolCallRequest>>(
                            serde_json::Value::Array(tc_values.clone()),
                        ) {
                            Ok(calls) if !calls.is_empty() => m.tool_calls = Some(calls),
                            Ok(_) => {}
                            Err(error) => {
                                // Leaving tool results without tool_calls causes provider 400s.
                                warn!(%error, "failed to restore assistant tool_calls from history");
                            }
                        }
                    }
                    if let Some(reasoning) = msg.reasoning_content {
                        m.reasoning_content = Some(reasoning);
                    }
                    if let Some(thinking_blocks) = msg.thinking_blocks {
                        m.thinking_blocks = Some(thinking_blocks);
                    }
                    m
                }
                "tool" => {
                    let tool_call_id = msg.tool_call_id.unwrap_or_default();
                    let mut m = Message::tool(msg.content, tool_call_id);
                    m.name = msg.name;
                    m
                }
                _ => continue,
            };
            messages.push(message);
        }

        messages
    }

    /// Drop orphan tool messages / incomplete tool_call groups so providers
    /// like DeepSeek do not reject the request with HTTP 400.
    pub fn sanitize_messages_for_provider(messages: &mut Vec<Message>) {
        use std::collections::HashSet;

        let original = std::mem::take(messages);
        let mut out: Vec<Message> = Vec::with_capacity(original.len());
        let mut open_tool_ids: HashSet<String> = HashSet::new();

        for mut msg in original {
            match msg.role.as_str() {
                "tool" => {
                    let id = msg.tool_call_id.as_deref().unwrap_or("");
                    if !id.is_empty() && open_tool_ids.remove(id) {
                        out.push(msg);
                    } else {
                        warn!(
                            tool_call_id = id,
                            "dropping orphan tool message before provider call"
                        );
                    }
                }
                "assistant" => {
                    if !open_tool_ids.is_empty() {
                        if let Some(prev) = out.iter_mut().rev().find(|m| m.role == "assistant") {
                            prev.tool_calls = None;
                        }
                        open_tool_ids.clear();
                    }
                    if let Some(ref calls) = msg.tool_calls {
                        if calls.is_empty() {
                            msg.tool_calls = None;
                        } else {
                            for call in calls {
                                if !call.id.is_empty() {
                                    open_tool_ids.insert(call.id.clone());
                                }
                            }
                        }
                    }
                    out.push(msg);
                }
                _ => {
                    if !open_tool_ids.is_empty() {
                        if let Some(prev) = out.iter_mut().rev().find(|m| m.role == "assistant") {
                            prev.tool_calls = None;
                        }
                        open_tool_ids.clear();
                    }
                    out.push(msg);
                }
            }
        }

        if !open_tool_ids.is_empty() {
            if let Some(prev) = out.iter_mut().rev().find(|m| m.role == "assistant") {
                prev.tool_calls = None;
            }
        }
        *messages = out;
    }

    /// Add a tool result to the message list
    ///
    /// The caller has already converted large results to artifact references.
    pub fn add_tool_result(
        &self,
        messages: &mut Vec<Message>,
        tool_call_id: String,
        _tool_name: String,
        result: String,
    ) {
        messages.push(Message::tool(result, tool_call_id));
    }

    /// Add an assistant message with optional tool calls
    pub fn add_assistant_message(
        &self,
        messages: &mut Vec<Message>,
        content: Option<String>,
        tool_calls: Option<Vec<agent_diva_providers::ToolCallRequest>>,
        reasoning_content: Option<String>,
        thinking_blocks: Option<Vec<serde_json::Value>>,
    ) {
        let mut msg = Message::assistant(content.unwrap_or_default());
        if let Some(calls) = tool_calls {
            msg.tool_calls = Some(calls);
        }
        if let Some(reasoning) = reasoning_content {
            msg.reasoning_content = Some(reasoning);
        }
        if let Some(blocks) = thinking_blocks {
            msg.thinking_blocks = Some(blocks);
        }
        messages.push(msg);
    }
}

fn render_stable_sections(sections: &[PromptSection]) -> String {
    match render_stable_prefix(sections) {
        Ok(prompt) => prompt,
        Err(error) => {
            warn!(%error, "stable context sections violated the assembly contract");
            sections
                .iter()
                .filter_map(|section| {
                    (!section.body.trim().is_empty()).then_some(section.body.as_str())
                })
                .collect::<Vec<_>>()
                .join("\n\n")
        }
    }
}

fn next_builder_session_key() -> String {
    format!(
        "context-builder:{}",
        CONTEXT_BUILDER_ID.fetch_add(1, Ordering::Relaxed)
    )
}

fn render_startup_injection(response: SystemPromptResponse) -> String {
    let mut rendered = response
        .prompt_block
        .map(render_system_prompt_block)
        .unwrap_or_default();

    if let StartupStatus::Degraded { .. } = response.status {
        if rendered.is_empty() {
            rendered = "## Memory Startup Status\n- status: degraded\n- reason: provider returned no startup block\n- last_usable_wakeup: omitted (no cache reuse)\n".to_string();
        }
    }

    rendered
}

fn render_system_prompt_block(block: SystemPromptBlock) -> String {
    match block.shape {
        StartupInjectionShape::CompactRenderedMarkdown => block.markdown,
    }
}

fn render_provider_error_startup_injection(error: &str) -> String {
    format!(
        "## Memory Startup Status\n- status: degraded\n- reason: provider error: {}\n- last_usable_wakeup: omitted (no cache reuse)\n",
        error.trim()
    )
}

impl Default for ContextBuilder {
    fn default() -> Self {
        Self::new(PathBuf::from("."))
    }
}

fn read_trimmed_markdown(path: &Path, max_chars: usize) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.chars().count() <= max_chars {
        return Some(trimmed.to_string());
    }

    let mut out = String::new();
    for (idx, ch) in trimmed.chars().enumerate() {
        if idx >= max_chars.saturating_sub(3) {
            break;
        }
        out.push(ch);
    }
    out.push_str("...");
    Some(out)
}

#[cfg(test)]
fn parse_identity_field(content: &str, keys: &[&str]) -> Option<String> {
    for line in content.lines() {
        let line = line.trim().trim_start_matches(&['-', '*'][..]).trim();
        if line.is_empty() {
            continue;
        }

        // Split on ASCII or full-width colon to avoid manual byte indices.
        let (prefix, value_part) = match line.split_once(':').or_else(|| line.split_once('：')) {
            Some((p, v)) => (p.trim(), v.trim()),
            None => continue,
        };

        for key in keys {
            if prefix.eq_ignore_ascii_case(key) && !value_part.is_empty() {
                return Some(value_part.to_string());
            }
        }
    }
    None
}

fn default_identity_header() -> String {
    format!(
        "# {} {}\n\nYou are {}, a {}.",
        DEFAULT_AGENT_NAME, DEFAULT_AGENT_EMOJI, DEFAULT_AGENT_NAME, DEFAULT_AGENT_ROLE
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::memory::{
        PrefetchRequest, PrefetchResponse, PrefetchStatus, SessionEndRequest, SessionEndResponse,
        SessionEndStatus, SyncTurnRequest, SyncTurnResponse, SyncTurnStatus, SystemPromptBlock,
        SystemPromptRequest, SystemPromptResponse,
    };
    use agent_diva_core::session::{SessionManager, SessionSearchQuery};
    use agent_diva_laputa::{PersonaInitialization, PersonaKind, PersonaService};
    use std::fs;
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
    use std::sync::{Arc, RwLock};
    use tempfile::TempDir;

    struct TestMemoryProvider;

    struct RevisionMemoryProvider {
        markdown: RwLock<String>,
        revision: AtomicU64,
        prompt_calls: AtomicUsize,
    }

    impl RevisionMemoryProvider {
        fn new(markdown: &str) -> Self {
            Self {
                markdown: RwLock::new(markdown.to_string()),
                revision: AtomicU64::new(0),
                prompt_calls: AtomicUsize::new(0),
            }
        }

        fn update(&self, markdown: &str) {
            *self.markdown.write().unwrap() = markdown.to_string();
            self.revision.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[async_trait::async_trait]
    impl MemoryProvider for TestMemoryProvider {
        fn system_prompt_block(
            &self,
            request: &SystemPromptRequest,
        ) -> agent_diva_core::Result<SystemPromptResponse> {
            Ok(SystemPromptResponse::ready(SystemPromptBlock {
                shape: agent_diva_core::memory::StartupInjectionShape::CompactRenderedMarkdown,
                markdown: format!("## Provider Memory\n{}", request.workspace_root.display()),
            }))
        }

        async fn prefetch(
            &self,
            _request: PrefetchRequest,
        ) -> agent_diva_core::Result<PrefetchResponse> {
            Ok(PrefetchResponse {
                status: PrefetchStatus::SkippedNoIntent,
                prompt_block: None,
            })
        }

        async fn sync_turn(
            &self,
            _request: SyncTurnRequest,
        ) -> agent_diva_core::Result<SyncTurnResponse> {
            Ok(SyncTurnResponse {
                status: SyncTurnStatus::Noop,
            })
        }

        async fn on_session_end(
            &self,
            _request: SessionEndRequest,
        ) -> agent_diva_core::Result<SessionEndResponse> {
            Ok(SessionEndResponse {
                status: SessionEndStatus::Noop,
            })
        }
    }

    #[async_trait::async_trait]
    impl MemoryProvider for RevisionMemoryProvider {
        fn system_prompt_block(
            &self,
            _request: &SystemPromptRequest,
        ) -> agent_diva_core::Result<SystemPromptResponse> {
            self.prompt_calls.fetch_add(1, Ordering::SeqCst);
            Ok(SystemPromptResponse::ready(SystemPromptBlock {
                shape: agent_diva_core::memory::StartupInjectionShape::CompactRenderedMarkdown,
                markdown: self.markdown.read().unwrap().clone(),
            }))
        }

        fn system_prompt_revision(&self, _request: &SystemPromptRequest) -> u64 {
            self.revision.load(Ordering::SeqCst)
        }

        async fn prefetch(
            &self,
            _request: PrefetchRequest,
        ) -> agent_diva_core::Result<PrefetchResponse> {
            Ok(PrefetchResponse::default())
        }

        async fn sync_turn(
            &self,
            _request: SyncTurnRequest,
        ) -> agent_diva_core::Result<SyncTurnResponse> {
            Ok(SyncTurnResponse {
                status: SyncTurnStatus::Noop,
            })
        }

        async fn on_session_end(
            &self,
            _request: SessionEndRequest,
        ) -> agent_diva_core::Result<SessionEndResponse> {
            Ok(SessionEndResponse::default())
        }
    }

    fn snapshot_section(snapshot: &StablePrefixSnapshot, target: ContextSection) -> &PromptSection {
        snapshot
            .sections
            .iter()
            .find(|section| section.section == target)
            .unwrap()
    }

    fn seed_persona(workspace: &Path, identity: &str, redline: &str) -> PersonaService {
        let service = PersonaService::open(workspace).unwrap();
        service
            .initialize(PersonaInitialization {
                identity: identity.to_string(),
                relationship: "relationship".to_string(),
                redline: redline.to_string(),
                user: "preference".to_string(),
                world: "world".to_string(),
            })
            .unwrap();
        service
    }

    #[test]
    fn test_build_system_prompt() {
        let builder = ContextBuilder::new(PathBuf::from("/tmp/test"));
        let prompt = builder.build_system_prompt(None);
        assert!(prompt.contains("agent-diva"));
        assert!(prompt.contains("/tmp/test"));
    }

    #[test]
    fn frozen_core_is_captured_once_and_frozen_for_the_session() {
        let workspace = TempDir::new().unwrap();
        let service = seed_persona(workspace.path(), "vivy", "redline");

        let builder = ContextBuilder::new(workspace.path().to_path_buf());
        let prompt = builder.build_system_prompt(None);
        assert!(prompt.contains("Frozen Core — identity"));
        assert!(prompt.contains("vivy"));

        // A governance write lands mid-session; this session's assembly
        // must keep using the session-start snapshot.
        let identity = service.get_document(PersonaKind::Identity).unwrap();
        service
            .save_user_document(
                PersonaKind::Identity,
                "rewritten",
                identity.revision,
                "test rewrite",
            )
            .unwrap();
        let frozen_prompt = builder.build_system_prompt(None);
        assert!(frozen_prompt.contains("vivy"));
        assert!(!frozen_prompt.contains("rewritten"));

        // The next session (a fresh builder) sees the applied write.
        let next_builder = ContextBuilder::new(workspace.path().to_path_buf());
        let next_prompt = next_builder.build_system_prompt(None);
        assert!(next_prompt.contains("rewritten"));
    }

    #[test]
    fn frozen_core_precedes_later_context_layers() {
        let workspace = TempDir::new().unwrap();
        seed_persona(workspace.path(), "identity", "red_line: true");

        let builder = ContextBuilder::new(workspace.path().to_path_buf());
        let prompt = builder.build_system_prompt(None);
        let frozen_pos = prompt.find("Frozen Core — redline").unwrap();
        let policy_pos = prompt.find("## Memory Management Policy").unwrap();
        assert!(frozen_pos < policy_pos, "Frozen Core precedes later layers");
    }

    #[test]
    fn prompt_guides_memory_tool_usage() {
        let builder = ContextBuilder::new(PathBuf::from("/tmp/test"));
        let prompt = builder.build_system_prompt(None);
        assert!(
            prompt.contains("memory_add"),
            "prompt should name memory_add"
        );
        assert!(
            prompt.contains("memory_remove"),
            "prompt should name memory_remove"
        );
        assert!(
            prompt.contains("proposal_created"),
            "prompt should teach honest result states"
        );
        assert!(
            !prompt.contains("Memory management tools are not available"),
            "Wave 1 restored tool guidance; unavailable wording must be gone"
        );
    }

    #[test]
    fn prompt_injects_l0_memory_management_policy() {
        let builder = ContextBuilder::new(PathBuf::from("/tmp/test"));
        let prompt = builder.build_system_prompt(None);
        assert!(
            prompt.contains("## Memory Management Policy"),
            "prompt should carry the L0 policy section"
        );
        assert!(prompt.contains("Action-Verified"));
        assert!(prompt.contains("Minimal pointer principle"));
    }

    #[test]
    fn context_prompt_includes_update_plan() {
        let builder = ContextBuilder::new(PathBuf::from("/tmp/test"));
        let prompt = builder.build_system_prompt(None);
        assert!(prompt.contains("update_plan"));
        assert!(prompt.contains("pending"));
        assert!(prompt.contains("in_progress"));
        assert!(prompt.contains("completed"));
        assert!(prompt.contains("not Plan mode"));
        assert!(prompt.contains("TODOLIST.md"));
        assert!(prompt.contains("explanation") || prompt.contains("plan"));
    }

    #[test]
    fn test_build_messages() {
        let builder = ContextBuilder::new(PathBuf::from("/tmp/test"));
        let messages =
            builder.build_messages(vec![], "Hello".to_string(), Some("cli"), Some("test"), None);
        assert_eq!(messages.len(), 3); // stable system + volatile envelope + user
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
        assert!(messages[1]
            .content
            .as_text()
            .is_some_and(|text| text.contains("volatile_meta")));
        assert_eq!(messages[2].content, "Hello".into());
    }

    #[test]
    fn c1a_t1_clock_changes_only_the_volatile_envelope() {
        let builder = ContextBuilder::new(PathBuf::from("/tmp/test"));
        let stable_before = builder.build_system_prompt_for_session(None, "session-t1");
        let volatile_before =
            builder.build_volatile_meta_section_at(Some("cli"), Some("chat-1"), "2026-08-10 10:00");
        let stable_after = builder.build_system_prompt_for_session(None, "session-t1");
        let volatile_after =
            builder.build_volatile_meta_section_at(Some("cli"), Some("chat-1"), "2026-08-10 10:01");

        assert_eq!(stable_before, stable_after);
        assert!(!stable_before.contains("## Current Time"));
        assert_ne!(volatile_before.body, volatile_after.body);
    }

    #[test]
    fn c1a_t2_working_memory_change_does_not_change_stable_prefix() {
        let builder = ContextBuilder::new(PathBuf::from("/tmp/test"));
        let stable = builder.build_system_prompt_for_session(None, "session-t2");
        let first = PromptSection::new(ContextSection::WorkingMemory, "checkpoint one");
        let second = PromptSection::new(ContextSection::WorkingMemory, "checkpoint two");

        assert_eq!(
            stable,
            builder.build_system_prompt_for_session(None, "session-t2")
        );
        assert_ne!(first.body, second.body);
    }

    #[test]
    fn c1a_t3_recall_presence_does_not_change_stable_prefix() {
        let builder = ContextBuilder::new(PathBuf::from("/tmp/test"));
        let stable = builder.build_system_prompt_for_session(None, "session-t3");
        let recall = PromptSection::new(ContextSection::PrefetchRecall, "recalled fact");

        assert_eq!(
            stable,
            builder.build_system_prompt_for_session(None, "session-t3")
        );
        assert_eq!(
            recall.stability,
            crate::context_assembly::SectionStability::TurnVolatile
        );
    }

    #[test]
    fn c1c_same_session_reuses_cached_sections() {
        let workspace = TempDir::new().unwrap();
        let provider = Arc::new(RevisionMemoryProvider::new("## Memory\nfirst"));
        let builder = ContextBuilder::new(workspace.path().to_path_buf())
            .with_memory_provider(provider.clone());

        let first = builder.stable_prefix_snapshot_for_session(None, "cache-session");
        let second = builder.stable_prefix_snapshot_for_session(None, "cache-session");
        let other = builder.stable_prefix_snapshot_for_session(None, "other-session");

        assert_eq!(first, second);
        assert_eq!(first.prefix_version, 1);
        assert_eq!(other.prefix_version, 1);
        assert_eq!(provider.prompt_calls.load(Ordering::SeqCst), 2);

        let replacement = Arc::new(RevisionMemoryProvider::new("## Memory\nreplacement"));
        let builder = builder.with_memory_provider(replacement.clone());
        let replaced = builder.stable_prefix_snapshot_for_session(None, "cache-session");
        assert!(
            snapshot_section(&replaced, ContextSection::MemoryPolicyAndIndex)
                .body
                .contains("replacement")
        );
        assert_eq!(replaced.prefix_version, 1);
        assert_eq!(replacement.prompt_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn c1c_agent_rules_and_skills_require_explicit_reload() {
        let workspace = TempDir::new().unwrap();
        fs::write(workspace.path().join("AGENTS.md"), "# Rules v1").unwrap();
        let builder = ContextBuilder::new(workspace.path().to_path_buf());

        let first = builder.stable_prefix_snapshot_for_session(None, "rules-session");
        fs::write(workspace.path().join("AGENTS.md"), "# Rules v2").unwrap();
        let cached = builder.stable_prefix_snapshot_for_session(None, "rules-session");
        assert_eq!(first, cached);

        builder.invalidate_agent_rules("rules-session");
        let reloaded = builder.stable_prefix_snapshot_for_session(None, "rules-session");
        assert_eq!(reloaded.prefix_version, 2);
        assert_eq!(
            snapshot_section(&reloaded, ContextSection::AgentRulesAndSkills).cache_break_reason,
            Some(CacheBreakReason::AgentRulesReload)
        );
        assert!(
            snapshot_section(&reloaded, ContextSection::AgentRulesAndSkills)
                .body
                .contains("Rules v2")
        );
        for section in [
            ContextSection::MaskAndIdentity,
            ContextSection::FrozenCore,
            ContextSection::MemoryPolicyAndIndex,
        ] {
            assert_eq!(
                snapshot_section(&first, section).body,
                snapshot_section(&reloaded, section).body
            );
        }

        let skill_dir = workspace.path().join("skills").join("cached-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: cached-skill\ndescription: cache test\n---\n\n# Cached skill\n",
        )
        .unwrap();
        let before_skill_reload = builder.stable_prefix_snapshot_for_session(None, "rules-session");
        assert_eq!(reloaded, before_skill_reload);

        builder.invalidate_skills("rules-session");
        let skills_reloaded = builder.stable_prefix_snapshot_for_session(None, "rules-session");
        assert_eq!(skills_reloaded.prefix_version, 3);
        assert_eq!(
            skills_reloaded.cache_break_reasons(),
            vec![CacheBreakReason::SkillsReload]
        );
        assert!(
            snapshot_section(&skills_reloaded, ContextSection::AgentRulesAndSkills)
                .body
                .contains("cached-skill")
        );
    }

    #[test]
    fn c1c_workspace_skill_reload_invalidates_all_sessions_lazily() {
        let workspace = TempDir::new().unwrap();
        let builder = ContextBuilder::new(workspace.path().to_path_buf());
        let first_a = builder.stable_prefix_snapshot_for_session(None, "session-a");
        let first_b = builder.stable_prefix_snapshot_for_session(None, "session-b");

        let skill_dir = workspace.path().join("skills").join("shared-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: shared-skill\ndescription: shared cache test\n---\n\n# Shared skill\n",
        )
        .unwrap();

        assert_eq!(builder.invalidate_skills_for_all_sessions(), 2);
        let cached_a = builder.stable_prefix_snapshot_for_session(None, "session-a");
        let cached_b = builder.stable_prefix_snapshot_for_session(None, "session-b");
        for cached in [&cached_a, &cached_b] {
            assert_eq!(cached.prefix_version, 2);
            assert_eq!(
                cached.cache_break_reasons(),
                vec![CacheBreakReason::SkillsReload]
            );
            assert!(
                snapshot_section(cached, ContextSection::AgentRulesAndSkills)
                    .body
                    .contains("shared-skill")
            );
        }
        for (first, refreshed) in [(&first_a, &cached_a), (&first_b, &cached_b)] {
            for section in [
                ContextSection::MaskAndIdentity,
                ContextSection::FrozenCore,
                ContextSection::MemoryPolicyAndIndex,
            ] {
                assert_eq!(
                    snapshot_section(first, section).body,
                    snapshot_section(refreshed, section).body
                );
            }
        }
    }

    #[test]
    fn c1c_t5_mask_switch_rebuilds_only_mask_section() {
        let workspace = TempDir::new().unwrap();
        let builder = ContextBuilder::new(workspace.path().to_path_buf());
        let first = builder.stable_prefix_snapshot_for_session(None, "mask-session");
        let mask = MaskFile::parse("---\nname: researcher\n---\n\nAct as a researcher.").unwrap();

        let switched = builder.stable_prefix_snapshot_for_session(Some(&mask), "mask-session");

        assert_eq!(switched.prefix_version, 2);
        assert_ne!(first.rendered, switched.rendered);
        assert_eq!(
            switched.cache_break_reasons(),
            vec![CacheBreakReason::MaskChanged]
        );
        assert_eq!(
            snapshot_section(&switched, ContextSection::MaskAndIdentity).cache_break_reason,
            Some(CacheBreakReason::MaskChanged)
        );
        for section in [
            ContextSection::FrozenCore,
            ContextSection::AgentRulesAndSkills,
            ContextSection::MemoryPolicyAndIndex,
        ] {
            assert_eq!(
                snapshot_section(&first, section).body,
                snapshot_section(&switched, section).body
            );
        }
    }

    #[test]
    fn c1c_t6_l1_revision_rebuilds_only_memory_section() {
        let workspace = TempDir::new().unwrap();
        let provider = Arc::new(RevisionMemoryProvider::new("## Memory\nfirst"));
        let builder = ContextBuilder::new(workspace.path().to_path_buf())
            .with_memory_provider(provider.clone());
        let first = builder.stable_prefix_snapshot_for_session(None, "memory-session");

        provider.update("## Memory\nsecond");
        let refreshed = builder.stable_prefix_snapshot_for_session(None, "memory-session");
        let cached = builder.stable_prefix_snapshot_for_session(None, "memory-session");

        assert_eq!(refreshed.prefix_version, 2);
        assert_eq!(refreshed, cached);
        assert_eq!(provider.prompt_calls.load(Ordering::SeqCst), 2);
        assert_eq!(
            refreshed.cache_break_reasons(),
            vec![CacheBreakReason::L1HotRefresh]
        );
        assert_ne!(
            snapshot_section(&first, ContextSection::MemoryPolicyAndIndex).body,
            snapshot_section(&refreshed, ContextSection::MemoryPolicyAndIndex).body
        );
        for section in [
            ContextSection::MaskAndIdentity,
            ContextSection::FrozenCore,
            ContextSection::AgentRulesAndSkills,
        ] {
            assert_eq!(
                snapshot_section(&first, section).body,
                snapshot_section(&refreshed, section).body
            );
        }
    }

    #[test]
    fn c1c_session_reset_recaptures_every_stable_section() {
        let workspace = TempDir::new().unwrap();
        let service = seed_persona(workspace.path(), "first", "redline");
        let builder = ContextBuilder::new(workspace.path().to_path_buf());
        let first = builder.stable_prefix_snapshot_for_session(None, "reset-session");

        let identity = service.get_document(PersonaKind::Identity).unwrap();
        service
            .save_user_document(
                PersonaKind::Identity,
                "second",
                identity.revision,
                "test reset",
            )
            .unwrap();
        agent_diva_laputa::release_frozen_core_session(workspace.path(), "reset-session");
        builder.reset_session_cache("reset-session");
        let reset = builder.stable_prefix_snapshot_for_session(None, "reset-session");

        assert_eq!(reset.prefix_version, 2);
        assert!(snapshot_section(&first, ContextSection::FrozenCore)
            .body
            .contains("first"));
        assert!(snapshot_section(&reset, ContextSection::FrozenCore)
            .body
            .contains("second"));
        assert_eq!(
            reset.cache_break_reasons(),
            vec![CacheBreakReason::SessionReset]
        );
        assert!(reset
            .sections
            .iter()
            .all(|section| { section.cache_break_reason == Some(CacheBreakReason::SessionReset) }));
    }

    #[test]
    fn c1c_session_end_drops_the_snapshot() {
        let workspace = TempDir::new().unwrap();
        let provider = Arc::new(RevisionMemoryProvider::new("## Memory\nfirst"));
        let builder = ContextBuilder::new(workspace.path().to_path_buf())
            .with_memory_provider(provider.clone());
        builder.stable_prefix_snapshot_for_session(None, "ended-session");

        builder.end_session_cache("ended-session");
        let recaptured = builder.stable_prefix_snapshot_for_session(None, "ended-session");

        assert_eq!(recaptured.prefix_version, 1);
        assert!(recaptured.cache_break_reasons().is_empty());
        assert_eq!(provider.prompt_calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_build_system_prompt_includes_skills_sections() {
        let workspace = TempDir::new().unwrap();
        let skills_dir = workspace.path().join("skills");
        fs::create_dir_all(skills_dir.join("always-skill")).unwrap();
        fs::write(
            skills_dir.join("always-skill").join("SKILL.md"),
            "---\nname: always-skill\ndescription: Always loaded\nmetadata: '{\"nanobot\":{\"always\":true}}'\n---\n\n# Always skill body\n",
        )
        .unwrap();

        let builder = ContextBuilder::with_skills(workspace.path().to_path_buf(), None);
        let prompt = builder.build_system_prompt(None);

        assert!(prompt.contains("## Active Skills"));
        assert!(prompt.contains("## Skills"));
        assert!(prompt.contains("<skills>"));
    }

    #[test]
    fn test_build_system_prompt_uses_memory_provider_contract() {
        let workspace = TempDir::new().unwrap();
        let builder = ContextBuilder::new(workspace.path().to_path_buf())
            .with_memory_provider(Arc::new(TestMemoryProvider));

        let prompt = builder.build_system_prompt(None);

        assert!(prompt.contains("## Provider Memory"));
        assert!(prompt.contains(&workspace.path().display().to_string()));
    }

    #[test]
    fn session_search_hits_do_not_enter_default_prompt_authority() {
        let workspace = TempDir::new().unwrap();
        let mut manager = SessionManager::new(workspace.path());
        let session = manager.get_or_create("telegram:123");
        session.add_message("user", "Secret launch evidence from old session");
        let key = session.key.clone();
        manager.save(manager.get(&key).unwrap()).unwrap();

        let search = manager
            .search(SessionSearchQuery::new("launch evidence"))
            .unwrap();
        assert_eq!(search.hits.len(), 1);

        let builder = ContextBuilder::new(workspace.path().to_path_buf());
        let prompt = builder.build_system_prompt(None);
        assert!(!prompt.contains("Secret launch evidence from old session"));
        assert!(!prompt.contains("session://telegram%3A123"));
    }

    #[test]
    fn test_build_system_prompt_consumes_explicit_compact_rendered_shape() {
        let rendered = render_startup_injection(SystemPromptResponse::ready(SystemPromptBlock {
            shape: agent_diva_core::memory::StartupInjectionShape::CompactRenderedMarkdown,
            markdown: "## Wakeup Summary\nRendered startup block".to_string(),
        }));

        assert_eq!(rendered, "## Wakeup Summary\nRendered startup block");
    }

    #[test]
    fn test_build_system_prompt_renders_degraded_startup_state() {
        let rendered =
            render_startup_injection(SystemPromptResponse::degraded("wakeup generation failed"));

        assert!(rendered.contains("status: degraded"));
        assert!(rendered.contains("wakeup generation failed"));
        assert!(rendered.contains("last_usable_wakeup: omitted"));
    }

    #[test]
    fn test_build_system_prompt_renders_provider_error_as_degraded_startup() {
        let rendered = render_provider_error_startup_injection("disk full");

        assert!(rendered.contains("status: degraded"));
        assert!(rendered.contains("provider error: disk full"));
        assert!(rendered.contains("last_usable_wakeup: omitted"));
    }

    #[test]
    fn test_add_tool_result() {
        let builder = ContextBuilder::new(PathBuf::from("/tmp/test"));
        let mut messages = vec![Message::user("test")];
        builder.add_tool_result(
            &mut messages,
            "call_123".to_string(),
            "read_file".to_string(),
            "file content".to_string(),
        );
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[1].role, "tool");
    }

    #[test]
    fn sanitize_messages_for_provider_drops_orphan_tools() {
        use agent_diva_providers::ToolCallRequest;
        use std::collections::HashMap;

        let mut messages = vec![
            Message::system("sys"),
            Message::user("hi"),
            Message::tool("orphan", "call_missing"),
            {
                let mut assistant = Message::assistant("");
                assistant.tool_calls = Some(vec![ToolCallRequest {
                    id: "call_1".into(),
                    call_type: "function".into(),
                    name: "read_file".into(),
                    arguments: HashMap::new(),
                }]);
                assistant
            },
            Message::tool("ok", "call_1"),
            Message::user("next"),
        ];
        ContextBuilder::sanitize_messages_for_provider(&mut messages);
        assert_eq!(messages.len(), 5);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
        assert_eq!(messages[2].role, "assistant");
        assert_eq!(messages[3].role, "tool");
        assert_eq!(messages[3].tool_call_id.as_deref(), Some("call_1"));
        assert_eq!(messages[4].role, "user");
    }

    #[test]
    fn test_add_assistant_message() {
        let builder = ContextBuilder::new(PathBuf::from("/tmp/test"));
        let mut messages = vec![Message::user("test")];

        // Test with reasoning content
        builder.add_assistant_message(
            &mut messages,
            Some("response".to_string()),
            None,
            Some("reasoning".to_string()),
            None,
        );

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[1].role, "assistant");
        assert_eq!(messages[1].content, "response".into());
        assert_eq!(messages[1].reasoning_content, Some("reasoning".to_string()));
    }

    #[test]
    fn test_build_system_prompt_excludes_legacy_authority_sections_by_default() {
        let workspace = TempDir::new().unwrap();
        fs::write(workspace.path().join("AGENTS.md"), "# Repo Rules").unwrap();
        fs::write(workspace.path().join("SOUL.md"), "# Core Traits").unwrap();
        fs::write(workspace.path().join("IDENTITY.md"), "# Identity").unwrap();
        fs::write(workspace.path().join("USER.md"), "# Preferences").unwrap();
        fs::write(workspace.path().join("BOOTSTRAP.md"), "# Bootstrap Steps").unwrap();

        let builder = ContextBuilder::new(workspace.path().to_path_buf());
        let prompt = builder.build_system_prompt(None);

        assert!(prompt.contains("## Agent Rules"));
        assert!(prompt.contains("# Repo Rules"));
        assert!(!prompt.contains("## Bootstrap"));
        assert!(!prompt.contains("## Soul"));
        assert!(!prompt.contains("## Identity"));
        assert!(!prompt.contains("## User Profile"));
        assert!(!prompt.contains("# Core Traits"));
        assert!(!prompt.contains("# Preferences"));
    }

    #[test]
    fn test_read_trimmed_markdown_respects_char_limit() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("SOUL.md");
        fs::write(&path, "abcdefghij").unwrap();

        let got = read_trimmed_markdown(&path, 6).unwrap();
        assert_eq!(got, "abc...");
        assert!(got.chars().count() <= 6);
    }

    #[test]
    fn test_build_system_prompt_does_not_use_identity_file_for_header() {
        let workspace = TempDir::new().unwrap();
        fs::write(
            workspace.path().join("IDENTITY.md"),
            "# Identity\n- Name: Nova\n- Emoji: ✨\n- Role: strategic coding partner\n- Style: concise and direct\n",
        )
        .unwrap();
        let builder = ContextBuilder::new(workspace.path().to_path_buf());
        let prompt = builder.build_system_prompt(None);
        assert!(prompt.contains("# agent-diva 🐈"));
        assert!(prompt.contains("You are agent-diva, a helpful AI assistant."));
        assert!(!prompt.contains("# Nova ✨"));
        assert!(!prompt.contains("strategic coding partner"));
        assert!(!prompt.contains("Preferred communication style: concise and direct."));
    }

    #[test]
    fn test_build_system_prompt_identity_header_falls_back_to_default() {
        let workspace = TempDir::new().unwrap();
        let builder = ContextBuilder::new(workspace.path().to_path_buf());
        let prompt = builder.build_system_prompt(None);
        assert!(prompt.contains("# agent-diva 🐈"));
        assert!(prompt.contains("You are agent-diva, a helpful AI assistant."));
    }

    #[test]
    fn test_build_system_prompt_empty_identity_falls_back_to_default() {
        let workspace = TempDir::new().unwrap();
        fs::write(workspace.path().join("IDENTITY.md"), "   \n").unwrap();
        let builder = ContextBuilder::new(workspace.path().to_path_buf());
        let prompt = builder.build_system_prompt(None);
        assert!(prompt.contains("# agent-diva 🐈"));
    }

    #[test]
    fn test_build_system_prompt_long_identity_file_is_not_rendered() {
        let workspace = TempDir::new().unwrap();
        let long_name = "N".repeat(6000);
        fs::write(
            workspace.path().join("IDENTITY.md"),
            format!("- Name: {}\n- Role: helper", long_name),
        )
        .unwrap();
        let builder = ContextBuilder::new(workspace.path().to_path_buf());
        let prompt = builder.build_system_prompt(None);
        assert!(prompt.contains("You are agent-diva, a helpful AI assistant."));
        assert!(!prompt.contains(&"N".repeat(120)));
    }

    #[test]
    fn test_parse_identity_field_handles_markdown_list() {
        let raw = "- Name: Diva\n- Style: pragmatic";
        assert_eq!(
            parse_identity_field(raw, &["name"]).as_deref(),
            Some("Diva")
        );
        assert_eq!(
            parse_identity_field(raw, &["style"]).as_deref(),
            Some("pragmatic")
        );
    }

    #[test]
    fn test_parse_identity_field_supports_chinese_voice_line() {
        let raw = "- Voice: 简洁、实用、协作";
        assert_eq!(
            parse_identity_field(raw, &["voice"]).as_deref(),
            Some("简洁、实用、协作")
        );
    }

    // ── Mask integration tests ──────────────────────────────────────────

    #[test]
    fn test_build_system_prompt_no_mask_unchanged() {
        let workspace = TempDir::new().unwrap();
        let builder = ContextBuilder::new(workspace.path().to_path_buf());
        let prompt = builder.build_system_prompt(None);

        // Should start with identity header, not mask content.
        assert!(prompt.starts_with("# agent-diva"));
        assert!(!prompt.contains("## Mask"));
    }

    #[test]
    fn test_build_system_prompt_default_mask_no_injection() {
        let workspace = TempDir::new().unwrap();
        let builder = ContextBuilder::new(workspace.path().to_path_buf());
        let mask = MaskFile::default_mask();
        let prompt = builder.build_system_prompt(Some(&mask));

        // Default mask should produce the same prompt as None.
        let baseline =
            ContextBuilder::new(workspace.path().to_path_buf()).build_system_prompt(None);
        assert_eq!(prompt, baseline);
    }

    #[test]
    fn test_build_system_prompt_custom_mask_injected_at_top() {
        let workspace = TempDir::new().unwrap();
        let builder = ContextBuilder::new(workspace.path().to_path_buf());

        let content = r#"---
name: "研究员"
---

你是一个专注调研与分析的研究员。"#;
        let mask = MaskFile::parse(content).unwrap();
        let prompt = builder.build_system_prompt(Some(&mask));

        // Mask prompt should appear at the very top (方案 A).
        assert!(prompt.starts_with("你是一个专注调研与分析的研究员。"));
        // Identity header should follow.
        assert!(prompt.contains("# agent-diva"));
    }

    #[test]
    fn test_build_system_prompt_mask_does_not_replace_default_identity() {
        let workspace = TempDir::new().unwrap();
        fs::write(
            workspace.path().join("IDENTITY.md"),
            "- Name: Nova\n- Role: partner\n",
        )
        .unwrap();
        let builder = ContextBuilder::new(workspace.path().to_path_buf());

        let content = r#"---
name: "writer"
---

You are a technical writer."#;
        let mask = MaskFile::parse(content).unwrap();
        let prompt = builder.build_system_prompt(Some(&mask));

        // Mask and default identity should both be present; legacy IDENTITY.md is not authority.
        assert!(prompt.starts_with("You are a technical writer."));
        assert!(prompt.contains("You are agent-diva, a helpful AI assistant."));
        assert!(!prompt.contains("You are Nova, a partner."));
    }
}
