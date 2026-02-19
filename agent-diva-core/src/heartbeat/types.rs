//! Heartbeat configuration types

use serde::{Deserialize, Serialize};

/// Default heartbeat interval: 30 minutes (in seconds)
pub const DEFAULT_HEARTBEAT_INTERVAL_S: i64 = 30 * 60;

/// The prompt sent to agent during heartbeat
pub const HEARTBEAT_PROMPT: &str = r#"Read HEARTBEAT.md in your workspace (if it exists).
Follow any instructions or tasks listed there.
If nothing needs attention, reply with just: HEARTBEAT_OK"#;

/// Token that indicates "nothing to do"
pub const HEARTBEAT_OK_TOKEN: &str = "HEARTBEAT_OK";

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
    fn test_heartbeat_ok_token() {
        assert_eq!(HEARTBEAT_OK_TOKEN, "HEARTBEAT_OK");
    }

    #[test]
    fn test_heartbeat_prompt_contains_ok_token() {
        assert!(HEARTBEAT_PROMPT.contains(HEARTBEAT_OK_TOKEN));
    }
}
