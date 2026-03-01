//! Context builder for assembling prompts

use crate::skills::SkillsLoader;
use agent_diva_core::memory::MemoryManager;
use agent_diva_providers::Message;
use std::path::PathBuf;

/// Builds the context for LLM requests
pub struct ContextBuilder {
    workspace: PathBuf,
    skills_loader: SkillsLoader,
    memory_manager: MemoryManager,
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
        }
    }

    /// Build system prompt from workspace files and memory
    pub fn build_system_prompt(&self) -> String {
        let workspace_path = self.workspace.display();
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M (%A)");

        let mut prompt = format!(
            r#"# agent-diva 🐈

You are agent-diva, a helpful AI assistant. You have access to tools that allow you to:
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
        let builder = ContextBuilder::new(PathBuf::from("/tmp/test".into()));
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
}
