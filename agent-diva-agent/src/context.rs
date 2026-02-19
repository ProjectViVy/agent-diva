//! Context builder for assembling prompts

use crate::skills::SkillsLoader;
use agent_diva_providers::Message;
use std::path::PathBuf;

/// Builds the context for LLM requests
pub struct ContextBuilder {
    workspace: PathBuf,
    skills_loader: Option<SkillsLoader>,
}

impl ContextBuilder {
    /// Create a new context builder
    pub fn new(workspace: PathBuf) -> Self {
        Self {
            workspace,
            skills_loader: None,
        }
    }

    /// Create a new context builder with skills
    pub fn with_skills(workspace: PathBuf, builtin_skills_dir: Option<PathBuf>) -> Self {
        let skills_loader = Some(SkillsLoader::new(&workspace, builtin_skills_dir));
        Self {
            workspace,
            skills_loader,
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

## Current Time
{now}

## Workspace
Your workspace is at: {workspace_path}
- Memory files: {workspace_path}/memory/MEMORY.md
- Memory history log: {workspace_path}/memory/HISTORY.md"#
        );

        // Add skills summary if available
        if let Some(ref loader) = self.skills_loader {
            let skills_summary = loader.build_skills_summary();
            if !skills_summary.is_empty() {
                prompt.push_str("\n\n## Available Skills\n");
                prompt.push_str(&skills_summary);
                prompt.push_str(
                    "\n\nYou can read the full skill content using the read_file tool with the location path.",
                );
            }
        }

        prompt.push_str(
            r#"

IMPORTANT: When responding to direct questions or conversations, reply directly with your text response.
Only use the 'message' tool when you need to send a message to a specific chat channel.
For normal conversation, just respond with text - do not call the message tool.

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
                "assistant" => Message::assistant(&msg.content),
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
    ) {
        let mut msg = Message::assistant(content.unwrap_or_default());
        if let Some(calls) = tool_calls {
            msg.tool_calls = Some(calls);
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
}
