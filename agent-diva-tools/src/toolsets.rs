//! Toolset definitions for grouping related tools.
//!
//! A [`Toolset`] is a named category of tools (e.g., "filesystem", "shell", "web")
//! that can be used for toolset-based tool filtering and discovery.

use serde::{Deserialize, Serialize};

/// A named group of related tools.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Toolset {
    /// Human-readable name for this toolset.
    pub name: String,
    /// List of tool names (as registered in [`ToolRegistry`]) that belong to this toolset.
    pub tool_names: Vec<String>,
    /// Brief description of what this toolset provides.
    pub description: String,
}

/// Returns the default built-in toolsets shipped with agent-diva.
///
/// These toolsets correspond to the tool categories already implemented
/// in the crate: filesystem, shell, web. Additional toolsets (cron, spawn,
/// attachment, message, mcp) are NOT included in defaults — they can be
/// added by callers or loaded from config.
pub fn default_toolsets() -> Vec<Toolset> {
    vec![
        Toolset {
            name: "filesystem".to_string(),
            tool_names: vec![
                "read_file".to_string(),
                "write_file".to_string(),
                "edit_file".to_string(),
                "list_dir".to_string(),
            ],
            description: "File system tools: read, write, edit files and list directories."
                .to_string(),
        },
        Toolset {
            name: "shell".to_string(),
            tool_names: vec!["exec".to_string()],
            description: "Shell command execution tool.".to_string(),
        },
        Toolset {
            name: "web".to_string(),
            tool_names: vec!["web_search".to_string(), "web_fetch".to_string()],
            description: "Web search and fetch tools for internet access.".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_toolsets_returns_non_empty() {
        let toolsets = default_toolsets();
        assert!(!toolsets.is_empty(), "default_toolsets should return at least one toolset");
    }

    #[test]
    fn each_toolset_has_required_fields() {
        let toolsets = default_toolsets();
        for ts in &toolsets {
            assert!(!ts.name.is_empty(), "Toolset name should not be empty");
            assert!(
                !ts.tool_names.is_empty(),
                "Toolset '{}' should have at least one tool name",
                ts.name
            );
            assert!(
                !ts.description.is_empty(),
                "Toolset '{}' should have a non-empty description",
                ts.name
            );
        }
    }

    #[test]
    fn filesystem_toolset_has_correct_tools() {
        let toolsets = default_toolsets();
        let fs = toolsets.iter().find(|t| t.name == "filesystem").expect("filesystem toolset should exist");
        assert_eq!(fs.tool_names, vec!["read_file", "write_file", "edit_file", "list_dir"]);
    }

    #[test]
    fn shell_toolset_has_exec() {
        let toolsets = default_toolsets();
        let sh = toolsets.iter().find(|t| t.name == "shell").expect("shell toolset should exist");
        assert_eq!(sh.tool_names, vec!["exec"]);
    }

    #[test]
    fn web_toolset_has_search_and_fetch() {
        let toolsets = default_toolsets();
        let web = toolsets.iter().find(|t| t.name == "web").expect("web toolset should exist");
        assert_eq!(web.tool_names, vec!["web_search", "web_fetch"]);
    }

    #[test]
    fn toolsets_are_clone_and_eq() {
        let a = default_toolsets();
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn no_duplicate_toolset_names() {
        let toolsets = default_toolsets();
        let names: Vec<&str> = toolsets.iter().map(|t| t.name.as_str()).collect();
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(names.len(), unique.len(), "default_toolsets should have unique names");
    }

    #[test]
    fn default_toolset_count_is_at_least_three() {
        let toolsets = default_toolsets();
        assert!(toolsets.len() >= 3, "default_toolsets should have at least 3 entries (filesystem, shell, web)");
    }
}
