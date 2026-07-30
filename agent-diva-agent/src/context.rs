//! Context builder for assembling prompts

use crate::mask::MaskFile;
use crate::mask::MaskPromptComposer;
use crate::memory_boundary::default_memory_provider;
use crate::skills::SkillsLoader;
use agent_diva_core::memory::{
    MemoryProvider, StartupInjectionShape, StartupStatus, SystemPromptBlock, SystemPromptRequest,
    SystemPromptResponse,
};
use agent_diva_core::soul::SoulStateStore;
use agent_diva_providers::Message;
use agent_diva_tools::sanitize::truncate_tool_result;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::warn;

mod prompt;

const DEFAULT_AGENT_NAME: &str = "agent-diva";
const DEFAULT_AGENT_EMOJI: &str = "🐈";
const DEFAULT_AGENT_ROLE: &str = "helpful AI assistant";

/// Runtime controls for soul prompt injection.
#[derive(Debug, Clone)]
pub struct SoulContextSettings {
    pub enabled: bool,
    pub max_chars: usize,
    pub bootstrap_once: bool,
}

impl Default for SoulContextSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            max_chars: 4000,
            bootstrap_once: true,
        }
    }
}

/// Builds the context for LLM requests
pub struct ContextBuilder {
    workspace: PathBuf,
    skills_loader: SkillsLoader,
    memory_provider: Arc<dyn MemoryProvider>,
    soul_settings: SoulContextSettings,
}

impl ContextBuilder {
    /// Create a new context builder
    pub fn new(workspace: PathBuf) -> Self {
        let skills_loader = SkillsLoader::new(&workspace, None);
        let memory_provider = default_memory_provider(&workspace);
        Self {
            workspace,
            skills_loader,
            memory_provider,
            soul_settings: SoulContextSettings::default(),
        }
    }

    /// Create a new context builder with skills
    pub fn with_skills(workspace: PathBuf, builtin_skills_dir: Option<PathBuf>) -> Self {
        let skills_loader = SkillsLoader::new(&workspace, builtin_skills_dir);
        let memory_provider = default_memory_provider(&workspace);
        Self {
            workspace,
            skills_loader,
            memory_provider,
            soul_settings: SoulContextSettings::default(),
        }
    }

    /// Override the memory provider boundary used for prompt assembly.
    pub fn with_memory_provider(mut self, memory_provider: Arc<dyn MemoryProvider>) -> Self {
        self.memory_provider = memory_provider;
        self
    }

    /// Override soul context settings.
    pub fn set_soul_settings(&mut self, settings: SoulContextSettings) {
        self.soul_settings = settings;
    }

    /// Build system prompt from workspace files and memory.
    ///
    /// When a non-default `mask` is provided, its body is injected at the top
    /// of the prompt (before the identity header) — 方案 A placement.
    pub fn build_system_prompt(&self, mask: Option<&MaskFile>) -> String {
        let workspace_path = self.workspace.display();
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M (%A)");
        let identity_header = self.load_identity_header();

        let mut prompt = String::new();

        // 方案 A: mask prompt at the very top when active.
        if let Some(mask_prompt) = MaskPromptComposer::compose(mask) {
            prompt.push_str(&mask_prompt);
            prompt.push_str("\n\n");
        }

        prompt.push_str(&format!(
            r#"{identity_header}

You have access to tools that allow you to:
- Read, write, and edit files
- Execute shell commands
- Search the web and fetch web pages
- Send messages to users on chat channels
- Schedule reminders and recurring jobs (cron)
- Track the current task with the lightweight `update_plan` TODO/progress checklist in normal chat

`update_plan` is a TODO/checklist tool, not Plan mode and not the repository's durable `TODOLIST.md`. Use it for complex, multi-step, or ambiguous tasks, and when the user asks for steps or progress tracking. Keep the checklist current as work advances: statuses are `pending`, `in_progress`, and `completed`; keep at most one item `in_progress`, and mark every item `completed` when the task is done. Do not use it for trivial one-step work or repeat the full checklist in chat after the tool call. Formal planning and approval use Plan mode; execution-session TODOs use their dedicated tools.

## Current Time
{now}

## Workspace
Your workspace is at: {workspace_path}
- Applied authority is consumed through the configured MemoryProvider boundary.
- Legacy authority files are compatibility/migration inputs only, not default authority."#
        ));

        self.append_agent_rules_and_bootstrap(&mut prompt);

        // Skills - progressive loading
        // 1) Always-loaded skills (full content)
        let always_skills = self.skills_loader.get_always_skills();
        if !always_skills.is_empty() {
            let always_content = self.skills_loader.load_skills_for_context(&always_skills);
            if !always_content.is_empty() {
                prompt.push_str("\n\n## Active Skills\n");
                prompt.push_str(&always_content);
            }
        }

        // 2) Available skills summary
        let skills_summary = self.skills_loader.build_skills_summary();
        if !skills_summary.is_empty() {
            prompt.push_str("\n\n## Skills\n");
            prompt.push_str(
                "The following skills extend your capabilities. To use a skill, read its SKILL.md file using the read_file tool.\n",
            );
            prompt
                .push_str("Skills with available=\"false\" need dependencies installed first.\n\n");
            prompt.push_str(&skills_summary);
        }

        // Inject long-term memory if available
        let memory_context = self
            .memory_provider
            .system_prompt_block(&SystemPromptRequest {
                workspace_root: self.workspace.clone(),
            })
            .map(render_startup_injection)
            .unwrap_or_else(|err| render_provider_error_startup_injection(&err.to_string()));
        if !memory_context.is_empty() {
            prompt.push_str("\n\n");
            prompt.push_str(&memory_context);
        }

        prompt.push_str(
            r#"

IMPORTANT: When responding to direct questions or conversations, reply directly with your text response.
Only use the 'message' tool when you need to send a message to a specific chat channel.
For normal conversation, just respond with text - do not call the message tool.
When a user asks to create a reminder, timer, or recurring schedule, use the 'cron' tool instead of saying the feature is unavailable.

Always be helpful, accurate, and concise. When using tools, explain what you're doing."#,
        );

        prompt.push_str(
            "\nWhen remembering something, create or update governed memory through the available memory tools or compatibility path; do not treat legacy authority files as default prompt authority.",
        );
        prompt.push_str(
            "\nBOOTSTRAP.md is a one-time onboarding input, not runtime authority. Do not read or replay it unless the user explicitly asks to start onboarding again.",
        );

        prompt
    }

    fn append_agent_rules_and_bootstrap(&self, prompt: &mut String) {
        if let Some(content) = self.read_soul_file("AGENTS.md") {
            self.append_section(prompt, "Agent Rules", &content);
        }

        if self.soul_settings.enabled && self.should_include_bootstrap() {
            let _ = SoulStateStore::new(&self.workspace).mark_bootstrap_seeded();
        }
    }

    fn should_include_bootstrap(&self) -> bool {
        if !self.soul_settings.bootstrap_once {
            return false;
        }

        let store = SoulStateStore::new(&self.workspace);
        match store.load() {
            Ok(state) => {
                state.bootstrap_seeded_at.is_none() && state.bootstrap_completed_at.is_none()
            }
            Err(error) => {
                warn!(
                    workspace = %self.workspace.display(),
                    error = %error,
                    "Skipping automatic Bootstrap because soul state could not be read"
                );
                false
            }
        }
    }

    fn read_soul_file(&self, rel: &str) -> Option<String> {
        let path = self.workspace.join(rel);
        read_trimmed_markdown(&path, self.soul_settings.max_chars)
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
    /// If `session_compaction_history` is non-empty, each compacted summary is
    /// injected as a boundary marker + system message before the raw history,
    /// so the LLM sees all summaries and the recent messages.
    pub fn build_messages(
        &self,
        history: Vec<agent_diva_core::session::ChatMessage>,
        current_message: String,
        channel: Option<&str>,
        chat_id: Option<&str>,
        session_compaction_history: &[agent_diva_core::session::CompactSummary],
    ) -> Vec<Message> {
        let mut messages = Vec::new();

        // System prompt
        let mut system_prompt = self.build_system_prompt(None);
        if let (Some(ch), Some(id)) = (channel, chat_id) {
            system_prompt.push_str(&format!(
                "\n\n## Current Session\nChannel: {}\nChat ID: {}",
                ch, id
            ));
        }
        messages.push(Message::system(system_prompt));

        // Inject compaction boundaries for each summary
        for (i, compaction) in session_compaction_history.iter().enumerate() {
            if compaction.summary.is_empty() {
                continue;
            }
            if i == 0 {
                // First compaction: full boundary markers
                messages.push(Message::system(prompt::COMPACTION_BOUNDARY));
            } else {
                // Subsequent compactions: shorter markers
                messages.push(Message::system(prompt::subsequent_compaction(i + 1)));
            }
            messages.push(Message::system(&compaction.summary));
            messages.push(Message::system("[compacted context end]"));
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

        // Current message
        messages.push(Message::user(current_message));

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
    /// Large tool results are truncated to prevent oversized API requests
    /// that could cause 400 errors from LLM providers.
    pub fn add_tool_result(
        &self,
        messages: &mut Vec<Message>,
        tool_call_id: String,
        _tool_name: String,
        result: String,
    ) {
        // Truncate large tool results to prevent API errors
        let truncated_result = truncate_tool_result(&result);
        messages.push(Message::tool(truncated_result, tool_call_id));
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
    use std::fs;
    use std::sync::Arc;
    use tempfile::TempDir;

    struct TestMemoryProvider;

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

    #[test]
    fn test_build_system_prompt() {
        let builder = ContextBuilder::new(PathBuf::from("/tmp/test"));
        let prompt = builder.build_system_prompt(None);
        assert!(prompt.contains("agent-diva"));
        assert!(prompt.contains("/tmp/test"));
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
            builder.build_messages(vec![], "Hello".to_string(), Some("cli"), Some("test"), &[]);
        assert_eq!(messages.len(), 2); // system + user
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
        assert_eq!(messages[1].content, "Hello".into());
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
    fn test_build_system_prompt_skips_bootstrap_when_completed() {
        let workspace = TempDir::new().unwrap();
        fs::write(workspace.path().join("BOOTSTRAP.md"), "# Bootstrap Steps").unwrap();
        let store = SoulStateStore::new(workspace.path());
        let state = agent_diva_core::soul::SoulState {
            bootstrap_completed_at: Some(chrono::Utc::now()),
            ..Default::default()
        };
        store.save(&state).unwrap();

        let builder = ContextBuilder::new(workspace.path().to_path_buf());
        let prompt = builder.build_system_prompt(None);
        assert!(!prompt.contains("## Bootstrap"));
    }

    #[test]
    fn test_bootstrap_is_seeded_once_and_not_reentered() {
        let workspace = TempDir::new().unwrap();
        fs::write(workspace.path().join("BOOTSTRAP.md"), "# Bootstrap Steps").unwrap();

        let builder = ContextBuilder::new(workspace.path().to_path_buf());
        builder.build_system_prompt(None);
        let store = SoulStateStore::new(workspace.path());
        let first_state = store.load().unwrap();
        assert!(first_state.bootstrap_seeded_at.is_some());
        assert!(first_state.bootstrap_completed_at.is_none());

        builder.build_system_prompt(None);
        let second_state = store.load().unwrap();
        assert_eq!(
            first_state.bootstrap_seeded_at,
            second_state.bootstrap_seeded_at
        );
        assert!(!builder.should_include_bootstrap());
    }

    #[test]
    fn test_bootstrap_does_not_start_when_state_file_is_corrupt() {
        let workspace = TempDir::new().unwrap();
        fs::write(workspace.path().join("BOOTSTRAP.md"), "# Bootstrap Steps").unwrap();
        let state_path = workspace.path().join(".agent-diva").join("soul-state.json");
        fs::create_dir_all(state_path.parent().unwrap()).unwrap();
        fs::write(&state_path, "not-json").unwrap();

        let builder = ContextBuilder::new(workspace.path().to_path_buf());
        builder.build_system_prompt(None);

        assert_eq!(fs::read_to_string(state_path).unwrap(), "not-json");
        assert!(!builder.should_include_bootstrap());
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
        let mut builder = ContextBuilder::new(workspace.path().to_path_buf());
        builder.set_soul_settings(SoulContextSettings {
            enabled: true,
            max_chars: 120,
            bootstrap_once: true,
        });
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
