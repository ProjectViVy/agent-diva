//! Heartbeat configuration types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Default heartbeat interval: 30 minutes (in seconds)
pub const DEFAULT_HEARTBEAT_INTERVAL_S: i64 = 30 * 60;

/// System prompt for the heartbeat decision LLM call
pub const HEARTBEAT_SYSTEM_PROMPT: &str =
    "You are a heartbeat agent. Call the heartbeat tool to report your decision.";

/// Structured decision returned by the heartbeat tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatDecision {
    /// "skip" or "run"
    pub action: String,
    /// Summary of tasks to execute (only when action == "run")
    #[serde(default)]
    pub tasks: Option<String>,
}

impl HeartbeatDecision {
    /// Parse a HeartbeatDecision from tool-call arguments.
    /// Returns a "skip" decision on any parse failure.
    pub fn from_tool_args(args: &HashMap<String, serde_json::Value>) -> Self {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("skip")
            .to_string();
        let tasks = args
            .get("tasks")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        Self { action, tasks }
    }

    pub fn is_run(&self) -> bool {
        self.action == "run"
    }
}

/// Return the tool definition for the heartbeat decision tool.
pub fn heartbeat_tool_definition() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": "heartbeat",
            "description": "Report your heartbeat decision. Use action='skip' if there are no tasks, or action='run' with a tasks summary if work is needed.",
            "parameters": {
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["skip", "run"],
                        "description": "Whether to skip (nothing to do) or run (tasks found)"
                    },
                    "tasks": {
                        "type": "string",
                        "description": "Summary of tasks to execute (required when action is 'run')"
                    }
                },
                "required": ["action"]
            }
        }
    })
}

/// Heartbeat configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatConfig {
    /// Whether heartbeat is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Interval in seconds between heartbeats
    #[serde(default = "default_interval")]
    pub interval_s: i64,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_s: DEFAULT_HEARTBEAT_INTERVAL_S,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_interval() -> i64 {
    DEFAULT_HEARTBEAT_INTERVAL_S
}

/// Check if HEARTBEAT.md has no actionable content
pub fn is_heartbeat_empty(content: Option<&str>) -> bool {
    let content = match content {
        Some(c) => c,
        None => return true,
    };

    // Lines to skip: empty, headers, HTML comments, empty checkboxes
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with("<!--")
            || line == "- [ ]"
            || line == "* [ ]"
            || line == "- [x]"
            || line == "* [x]"
        {
            continue;
        }
        return false; // Found actionable content
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heartbeat_config_default() {
        let config = HeartbeatConfig::default();
        assert!(config.enabled);
        assert_eq!(config.interval_s, DEFAULT_HEARTBEAT_INTERVAL_S);
    }

    #[test]
    fn test_is_heartbeat_empty_none() {
        assert!(is_heartbeat_empty(None));
    }

    #[test]
    fn test_is_heartbeat_empty_empty_string() {
        assert!(is_heartbeat_empty(Some("")));
    }

    #[test]
    fn test_is_heartbeat_empty_whitespace_only() {
        assert!(is_heartbeat_empty(Some("   \n\t  ")));
    }

    #[test]
    fn test_is_heartbeat_empty_headers_only() {
        let content = "# Header\n## Subheader\n### Another";
        assert!(is_heartbeat_empty(Some(content)));
    }

    #[test]
    fn test_is_heartbeat_empty_comments_only() {
        let content = "<!-- comment -->\n<!-- another -->";
        assert!(is_heartbeat_empty(Some(content)));
    }

    #[test]
    fn test_is_heartbeat_empty_empty_checkboxes() {
        let content = "- [ ]\n* [ ]\n- [x]\n* [x]";
        assert!(is_heartbeat_empty(Some(content)));
    }

    #[test]
    fn test_is_heartbeat_empty_mixed_skippable() {
        let content = "# Title\n\n<!-- comment -->\n- [ ]\n\n* [x]\n";
        assert!(is_heartbeat_empty(Some(content)));
    }

    #[test]
    fn test_is_heartbeat_empty_has_content() {
        let content = "# Title\n\nSome actionable content here";
        assert!(!is_heartbeat_empty(Some(content)));
    }

    #[test]
    fn test_is_heartbeat_empty_task_item() {
        let content = "- [ ] Task with description";
        assert!(!is_heartbeat_empty(Some(content)));
    }

    #[test]
    fn test_is_heartbeat_empty_normal_text() {
        let content = "Just some normal text";
        assert!(!is_heartbeat_empty(Some(content)));
    }

    #[test]
    fn test_heartbeat_tool_definition_structure() {
        let tool = heartbeat_tool_definition();
        assert_eq!(tool["type"], "function");
        assert_eq!(tool["function"]["name"], "heartbeat");
        assert!(tool["function"]["parameters"]["properties"]["action"].is_object());
    }

    #[test]
    fn test_heartbeat_decision_from_tool_args_skip() {
        let mut args = HashMap::new();
        args.insert("action".to_string(), serde_json::json!("skip"));
        let decision = HeartbeatDecision::from_tool_args(&args);
        assert_eq!(decision.action, "skip");
        assert!(decision.tasks.is_none());
        assert!(!decision.is_run());
    }

    #[test]
    fn test_heartbeat_decision_from_tool_args_run() {
        let mut args = HashMap::new();
        args.insert("action".to_string(), serde_json::json!("run"));
        args.insert("tasks".to_string(), serde_json::json!("Check the logs"));
        let decision = HeartbeatDecision::from_tool_args(&args);
        assert_eq!(decision.action, "run");
        assert_eq!(decision.tasks.as_deref(), Some("Check the logs"));
        assert!(decision.is_run());
    }

    #[test]
    fn test_heartbeat_decision_from_tool_args_malformed() {
        let args = HashMap::new();
        let decision = HeartbeatDecision::from_tool_args(&args);
        assert_eq!(decision.action, "skip");
        assert!(!decision.is_run());
    }

    #[test]
    fn test_heartbeat_system_prompt() {
        assert!(HEARTBEAT_SYSTEM_PROMPT.contains("heartbeat"));
    }
}
