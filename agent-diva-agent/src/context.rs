//! Context builder for assembling prompts

use crate::skills::SkillsLoader;
use agent_diva_core::memory::MemoryManager;
use agent_diva_core::soul::SoulStateStore;
use agent_diva_providers::Message;
use std::path::Path;
use std::path::PathBuf;

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
    memory_manager: MemoryManager,
    soul_settings: SoulContextSettings,
}

impl ContextBuilder {
    /// Create a new context builder
    pub fn new(workspace: PathBuf) -> Self {
        let skills_loader = SkillsLoader::new(&workspace, None);
        let memory_manager = MemoryManager::new(&workspace);
        Self {
            workspace,
            skills_loader,
            memory_manager,
            soul_settings: SoulContextSettings::default(),
        }
    }

    /// Create a new context builder with skills
    pub fn with_skills(workspace: PathBuf, builtin_skills_dir: Option<PathBuf>) -> Self {
        let skills_loader = SkillsLoader::new(&workspace, builtin_skills_dir);
        let memory_manager = MemoryManager::new(&workspace);
        Self {
            workspace,
            skills_loader,
            memory_manager,
            soul_settings: SoulContextSettings::default(),
        }
    }

    /// Override soul context settings.
    pub fn set_soul_settings(&mut self, settings: SoulContextSettings) {
        self.soul_settings = settings;
    }

    /// Build system prompt from workspace files and memory
    pub fn build_system_prompt(&self) -> String {
        let workspace_path = self.workspace.display();
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M (%A)");
        let identity_header = self.load_identity_header();

        let mut prompt = format!(
            r#"{identity_header}

You have access to tools that allow you to:
- Read, write, and edit files
- Execute shell commands
- Search the web and fetch web pages
- Send messages to users on chat channels
- Schedule reminders and recurring jobs (cron)

## Current Time
{now}

## Workspace
Your workspace is at: {workspace_path}
- Memory files: {workspace_path}/memory/MEMORY.md
- Memory history log: {workspace_path}/memory/HISTORY.md"#
        );

        if self.soul_settings.enabled {
            self.append_soul_sections(&mut prompt);
        }

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
        let memory_context = self.memory_manager.get_memory_context();
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

        prompt.push_str(&format!(
            "\nWhen remembering something, write to {}/memory/MEMORY.md",
            workspace_path
        ));

        prompt
    }

    fn append_soul_sections(&self, prompt: &mut String) {
        let sections = [
            ("AGENTS.md", "Agent Rules"),
            ("SOUL.md", "Soul"),
            ("IDENTITY.md", "Identity"),
            ("USER.md", "User Profile"),
        ];

        for (rel, title) in sections {
            if let Some(content) = self.read_soul_file(rel) {
                self.append_section(prompt, title, &content);
            }
        }

        if self.should_include_bootstrap() {
            if let Some(content) = self.read_soul_file("BOOTSTRAP.md") {
                let _ = SoulStateStore::new(&self.workspace).mark_bootstrap_seeded();
                self.append_section(prompt, "Bootstrap", &content);
            }
        }
    }

    fn should_include_bootstrap(&self) -> bool {
        if !self.soul_settings.bootstrap_once {
            return true;
        }
        let store = SoulStateStore::new(&self.workspace);
        !store.is_bootstrap_completed()
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
        let Some(content) = self.read_soul_file("IDENTITY.md") else {
            return default_identity_header();
        };

        let name = parse_identity_field(&content, &["name", "agent", "assistant"])
            .unwrap_or_else(|| DEFAULT_AGENT_NAME.to_string());
        let emoji = parse_identity_field(&content, &["emoji", "icon", "signature"])
            .unwrap_or_else(|| DEFAULT_AGENT_EMOJI.to_string());
        let role = parse_identity_field(&content, &["role", "nature", "type"])
            .unwrap_or_else(|| DEFAULT_AGENT_ROLE.to_string());
        let voice = parse_identity_field(&content, &["voice", "style", "vibe"]);

        let mut header = format!("# {} {}\n\nYou are {}, a {}.", name, emoji, name, role);
        if let Some(voice) = voice {
            header.push_str(" Preferred communication style: ");
            header.push_str(&voice);
            header.push('.');
        }
        header
    }

    /// Build the complete message list for an LLM call
    pub fn build_messages(
        &self,
        history: Vec<agent_diva_core::session::ChatMessage>,
        current_message: String,
        channel: Option<&str>,
        chat_id: Option<&str>,
    ) -> Vec<Message> {
        let mut messages = Vec::new();

        // System prompt
        let mut system_prompt = self.build_system_prompt();
        if let (Some(ch), Some(id)) = (channel, chat_id) {
            system_prompt.push_str(&format!(
                "\n\n## Current Session\nChannel: {}\nChat ID: {}",
                ch, id
            ));
        }
        messages.push(Message::system(system_prompt));

        // History - convert from ChatMessage to Message
        for msg in history {
            let message = match msg.role.as_str() {
                "user" => Message::user(&msg.content),
                "assistant" => {
                    let mut m = Message::assistant(&msg.content);
                    // Restore tool_calls from session history
                    if let Some(ref tc_values) = msg.tool_calls {
                        if let Ok(calls) =
                            serde_json::from_value::<Vec<agent_diva_providers::ToolCallRequest>>(
                                serde_json::Value::Array(tc_values.clone()),
                            )
                        {
                            m.tool_calls = Some(calls);
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

    /// Add a tool result to the message list
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

fn parse_identity_field(content: &str, keys: &[&str]) -> Option<String> {
    for line in content.lines() {
        let line = line.trim().trim_start_matches(&['-', '*'][..]).trim();
        for key in keys {
            if line.len() <= key.len() + 1 {
                continue;
            }
            let (prefix, rest) = line.split_at(key.len());
            if !prefix.eq_ignore_ascii_case(key) {
                continue;
            }
            let value = rest
                .strip_prefix(':')
                .or_else(|| rest.strip_prefix('：'))
                .map(str::trim);
            if let Some(value) = value {
                if !value.is_empty() {
                    return Some(value.to_string());
                }
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
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_build_system_prompt() {
        let builder = ContextBuilder::new(PathBuf::from("/tmp/test"));
        let prompt = builder.build_system_prompt();
        assert!(prompt.contains("agent-diva"));
        assert!(prompt.contains("/tmp/test"));
    }

    #[test]
    fn test_build_messages() {
        let builder = ContextBuilder::new(PathBuf::from("/tmp/test"));
        let messages =
            builder.build_messages(vec![], "Hello".to_string(), Some("cli"), Some("test"));
        assert_eq!(messages.len(), 2); // system + user
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
        assert_eq!(messages[1].content, "Hello");
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
        let prompt = builder.build_system_prompt();

        assert!(prompt.contains("## Active Skills"));
        assert!(prompt.contains("## Skills"));
        assert!(prompt.contains("<skills>"));
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
        assert_eq!(messages[1].content, "response");
        assert_eq!(messages[1].reasoning_content, Some("reasoning".to_string()));
    }

    #[test]
    fn test_build_system_prompt_includes_soul_sections_in_order() {
        let workspace = TempDir::new().unwrap();
        fs::write(workspace.path().join("AGENTS.md"), "# Repo Rules").unwrap();
        fs::write(workspace.path().join("SOUL.md"), "# Core Traits").unwrap();
        fs::write(workspace.path().join("IDENTITY.md"), "# Identity").unwrap();
        fs::write(workspace.path().join("USER.md"), "# Preferences").unwrap();
        fs::write(workspace.path().join("BOOTSTRAP.md"), "# Bootstrap Steps").unwrap();

        let builder = ContextBuilder::new(workspace.path().to_path_buf());
        let prompt = builder.build_system_prompt();

        let idx_agents = prompt.find("## Agent Rules").unwrap();
        let idx_soul = prompt.find("## Soul").unwrap();
        let idx_identity = prompt.find("## Identity").unwrap();
        let idx_user = prompt.find("## User Profile").unwrap();
        let idx_bootstrap = prompt.find("## Bootstrap").unwrap();

        assert!(idx_agents < idx_soul);
        assert!(idx_soul < idx_identity);
        assert!(idx_identity < idx_user);
        assert!(idx_user < idx_bootstrap);
    }

    #[test]
    fn test_build_system_prompt_skips_bootstrap_when_completed() {
        let workspace = TempDir::new().unwrap();
        fs::write(workspace.path().join("BOOTSTRAP.md"), "# Bootstrap Steps").unwrap();
        let store = SoulStateStore::new(workspace.path());
        let mut state = agent_diva_core::soul::SoulState::default();
        state.bootstrap_completed_at = Some(chrono::Utc::now());
        store.save(&state).unwrap();

        let builder = ContextBuilder::new(workspace.path().to_path_buf());
        let prompt = builder.build_system_prompt();
        assert!(!prompt.contains("## Bootstrap"));
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
    fn test_build_system_prompt_uses_identity_file_for_header() {
        let workspace = TempDir::new().unwrap();
        fs::write(
            workspace.path().join("IDENTITY.md"),
            "# Identity\n- Name: Nova\n- Emoji: ✨\n- Role: strategic coding partner\n- Style: concise and direct\n",
        )
        .unwrap();
        let builder = ContextBuilder::new(workspace.path().to_path_buf());
        let prompt = builder.build_system_prompt();
        assert!(prompt.contains("# Nova ✨"));
        assert!(prompt.contains("You are Nova, a strategic coding partner."));
        assert!(prompt.contains("Preferred communication style: concise and direct."));
    }

    #[test]
    fn test_build_system_prompt_identity_header_falls_back_to_default() {
        let workspace = TempDir::new().unwrap();
        let builder = ContextBuilder::new(workspace.path().to_path_buf());
        let prompt = builder.build_system_prompt();
        assert!(prompt.contains("# agent-diva 🐈"));
        assert!(prompt.contains("You are agent-diva, a helpful AI assistant."));
    }

    #[test]
    fn test_build_system_prompt_empty_identity_falls_back_to_default() {
        let workspace = TempDir::new().unwrap();
        fs::write(workspace.path().join("IDENTITY.md"), "   \n").unwrap();
        let builder = ContextBuilder::new(workspace.path().to_path_buf());
        let prompt = builder.build_system_prompt();
        assert!(prompt.contains("# agent-diva 🐈"));
    }

    #[test]
    fn test_build_system_prompt_long_identity_is_trimmed_by_max_chars() {
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
        let prompt = builder.build_system_prompt();
        assert!(prompt.contains("You are "));
        assert!(prompt.chars().count() > 120);
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
}
