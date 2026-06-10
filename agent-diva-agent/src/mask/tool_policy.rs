//! Tool policy resolution for the mask system.
//!
//! Computes the effective set of tools available to an agent by applying
//! mask-level allow/deny lists against the global built-in tool set.

use std::collections::BTreeSet;

use agent_diva_core::config::schema::{AgentMode, ToolLimits};

use super::mask_file::MaskFile;

/// Stateless helper that resolves effective tool capabilities.
///
/// Formula: `effective = global_builtin ∩ allow − deny`
///
/// - When `allow` is empty, every global tool is permitted (except those
///   explicitly denied).
/// - When `deny` is empty, no tools are removed.
/// - Unknown tool names that appear in `allow` or `deny` but are absent
///   from the global set are silently ignored.
/// - The result is always sorted alphabetically for deterministic output.
pub struct ToolPolicy;

impl ToolPolicy {
    /// Compute the effective tool list given global tools and mask limits.
    pub fn resolve(global_tools: &[String], tool_limits: &ToolLimits) -> Vec<String> {
        let global_set: BTreeSet<&str> = global_tools.iter().map(|s| s.as_str()).collect();

        // Step 1: determine the allowed superset.
        let allowed: BTreeSet<&str> = if tool_limits.allow.is_empty() {
            // No allow list → all global tools are candidates.
            global_set.clone()
        } else {
            // Allow list → intersection with global (unknown entries ignored).
            let allow_set: BTreeSet<&str> =
                tool_limits.allow.iter().map(|s| s.as_str()).collect();
            global_set.intersection(&allow_set).copied().collect()
        };

        // Step 2: subtract denied tools (unknown entries ignored).
        let deny_set: BTreeSet<&str> = tool_limits.deny.iter().map(|s| s.as_str()).collect();
        let effective: BTreeSet<&str> = allowed.difference(&deny_set).copied().collect();

        // Step 3: convert to sorted Vec<String>.
        effective.into_iter().map(String::from).collect()
    }

    /// Compute the effective tool list for a child agent.
    ///
    /// Formula: `child = parent_effective ∩ child_allow − child_deny`
    ///
    /// - When `child_allow` is empty, the child inherits all parent tools
    ///   (except those explicitly denied).
    /// - The child can never access tools outside the parent set —
    ///   `child ⊆ parent` is an invariant.
    /// - Unknown tool names in `child_allow` or `child_deny` are silently
    ///   ignored (they cannot appear in the parent set anyway).
    /// - The result is always sorted alphabetically for deterministic output.
    pub fn resolve_child(parent_tools: &[String], child_limits: &ToolLimits) -> Vec<String> {
        let parent_set: BTreeSet<&str> = parent_tools.iter().map(|s| s.as_str()).collect();

        // Step 1: determine the allowed superset from the child's allow list.
        let allowed: BTreeSet<&str> = if child_limits.allow.is_empty() {
            // No allow list → child inherits all parent tools.
            parent_set.clone()
        } else {
            // Allow list → intersection with parent (unknown entries ignored).
            let allow_set: BTreeSet<&str> =
                child_limits.allow.iter().map(|s| s.as_str()).collect();
            parent_set.intersection(&allow_set).copied().collect()
        };

        // Step 2: subtract denied tools (unknown entries ignored).
        let deny_set: BTreeSet<&str> = child_limits.deny.iter().map(|s| s.as_str()).collect();
        let effective: BTreeSet<&str> = allowed.difference(&deny_set).copied().collect();

        // Step 3: convert to sorted Vec<String>.
        effective.into_iter().map(String::from).collect()
    }

    /// Check whether a single tool is allowed under the given limits.
    pub fn is_tool_allowed(
        tool_name: &str,
        global_tools: &[String],
        tool_limits: &ToolLimits,
    ) -> bool {
        // The tool must exist in the global set.
        if !global_tools.iter().any(|t| t == tool_name) {
            return false;
        }

        // Check allow list.
        if !tool_limits.allow.is_empty()
            && !tool_limits.allow.iter().any(|t| t == tool_name)
        {
            return false;
        }

        // Check deny list.
        if tool_limits.deny.iter().any(|t| t == tool_name) {
            return false;
        }

        true
    }

    /// Check whether a mask enforces read-only (reviewer) mode.
    ///
    /// Returns `true` only when the mask's `mode` is explicitly set to
    /// [`AgentMode::Assist`]. Returns `false` for [`AgentMode::Normal`]
    /// or when no mode is configured (`None`).
    pub fn is_read_only_mode(mask: &MaskFile) -> bool {
        matches!(mask.frontmatter.mode, Some(AgentMode::Assist))
    }

    /// Filter a tool list to only include read-safe tools.
    ///
    /// When a mask operates in read-only (Assist) mode, write tools are
    /// excluded from the available set. This function removes any tool
    /// that is not in the read-safe allowlist.
    ///
    /// **Read-safe tools**: `read_file`, `search_files`, `web_search`, `web_extract`
    /// **Excluded tools**: everything else (notably `terminal`, `write_file`, `patch`)
    pub fn filter_read_only_tools(tools: &[String]) -> Vec<String> {
        const READ_ONLY_TOOLS: &[&str] = &[
            "read_file",
            "search_files",
            "web_search",
            "web_extract",
        ];

        let read_set: BTreeSet<&str> = READ_ONLY_TOOLS.iter().copied().collect();

        tools
            .iter()
            .filter(|t| read_set.contains(t.as_str()))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mask::mask_file::MaskFile;

    /// Helper to build a `Vec<String>` from a slice of &str.
    fn sv(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn no_limits_returns_all_global_tools() {
        let global = sv(&["filesystem", "shell", "web_search"]);
        let limits = ToolLimits::default();

        let result = ToolPolicy::resolve(&global, &limits);
        assert_eq!(result, sv(&["filesystem", "shell", "web_search"]));
    }

    #[test]
    fn allow_list_only_returns_intersection() {
        let global = sv(&["filesystem", "shell", "web_search", "spawn"]);
        let limits = ToolLimits {
            allow: sv(&["shell", "spawn"]),
            deny: vec![],
        };

        let result = ToolPolicy::resolve(&global, &limits);
        assert_eq!(result, sv(&["shell", "spawn"]));
    }

    #[test]
    fn deny_list_only_removes_denied() {
        let global = sv(&["filesystem", "shell", "web_search"]);
        let limits = ToolLimits {
            allow: vec![],
            deny: sv(&["shell"]),
        };

        let result = ToolPolicy::resolve(&global, &limits);
        assert_eq!(result, sv(&["filesystem", "web_search"]));
    }

    #[test]
    fn allow_and_deny_applied_together() {
        let global = sv(&["filesystem", "shell", "web_search", "spawn"]);
        let limits = ToolLimits {
            allow: sv(&["shell", "web_search", "spawn"]),
            deny: sv(&["spawn"]),
        };

        let result = ToolPolicy::resolve(&global, &limits);
        assert_eq!(result, sv(&["shell", "web_search"]));
    }

    #[test]
    fn unknown_tools_in_allow_are_ignored() {
        let global = sv(&["filesystem", "shell"]);
        let limits = ToolLimits {
            allow: sv(&["shell", "nonexistent_tool", "another_fake"]),
            deny: vec![],
        };

        let result = ToolPolicy::resolve(&global, &limits);
        assert_eq!(result, sv(&["shell"]));
    }

    #[test]
    fn unknown_tools_in_deny_are_ignored() {
        let global = sv(&["filesystem", "shell"]);
        let limits = ToolLimits {
            allow: vec![],
            deny: sv(&["nonexistent_tool"]),
        };

        let result = ToolPolicy::resolve(&global, &limits);
        assert_eq!(result, sv(&["filesystem", "shell"]));
    }

    #[test]
    fn empty_global_tools_returns_empty() {
        let global: Vec<String> = vec![];
        let limits = ToolLimits {
            allow: sv(&["shell"]),
            deny: sv(&["filesystem"]),
        };

        let result = ToolPolicy::resolve(&global, &limits);
        assert!(result.is_empty());
    }

    #[test]
    fn deterministic_ordering() {
        let global = sv(&["zebra", "alpha", "mike", "bravo"]);
        let limits = ToolLimits::default();

        let first = ToolPolicy::resolve(&global, &limits);
        let second = ToolPolicy::resolve(&global, &limits);
        assert_eq!(first, second);
        // Verify it is alphabetically sorted.
        assert_eq!(first, sv(&["alpha", "bravo", "mike", "zebra"]));
    }

    // ── resolve_child tests ───────────────────────────────────────────

    #[test]
    fn child_no_limits_gets_all_parent_tools() {
        let parent = sv(&["filesystem", "shell", "web_search", "spawn"]);
        let child_limits = ToolLimits::default();

        let result = ToolPolicy::resolve_child(&parent, &child_limits);
        assert_eq!(result, sv(&["filesystem", "shell", "spawn", "web_search"]));
    }

    #[test]
    fn child_with_allow_list_gets_intersection() {
        let parent = sv(&["filesystem", "shell", "web_search", "spawn"]);
        let child_limits = ToolLimits {
            allow: sv(&["shell", "spawn"]),
            deny: vec![],
        };

        let result = ToolPolicy::resolve_child(&parent, &child_limits);
        assert_eq!(result, sv(&["shell", "spawn"]));
    }

    #[test]
    fn child_cannot_access_tools_not_in_parent() {
        let parent = sv(&["filesystem", "shell"]);
        let child_limits = ToolLimits {
            allow: sv(&["shell", "web_search", "spawn"]),
            deny: vec![],
        };

        let result = ToolPolicy::resolve_child(&parent, &child_limits);
        // web_search and spawn are not in parent, so they are excluded.
        assert_eq!(result, sv(&["shell"]));
    }

    #[test]
    fn child_subset_of_parent_invariant_with_deny() {
        let parent = sv(&["filesystem", "shell", "web_search", "spawn"]);
        let child_limits = ToolLimits {
            allow: sv(&["shell", "web_search", "spawn"]),
            deny: sv(&["spawn"]),
        };

        let result = ToolPolicy::resolve_child(&parent, &child_limits);
        // spawn is denied; shell and web_search remain.
        assert_eq!(result, sv(&["shell", "web_search"]));
        // Verify every child tool is in the parent set.
        for tool in &result {
            assert!(parent.contains(tool), "child tool '{}' not in parent", tool);
        }
    }

    #[test]
    fn child_subset_invariant_for_all_combinations() {
        // Exhaustive check: child ⊆ parent for various allow/deny combos.
        let parent = sv(&["filesystem", "shell", "web_search", "spawn", "mcp"]);

        let combos = vec![
            ToolLimits::default(),
            ToolLimits {
                allow: sv(&["shell"]),
                deny: vec![],
            },
            ToolLimits {
                allow: vec![],
                deny: sv(&["filesystem", "mcp"]),
            },
            ToolLimits {
                allow: sv(&["shell", "web_search", "nonexistent"]),
                deny: sv(&["web_search"]),
            },
            ToolLimits {
                allow: sv(&["shell", "spawn"]),
                deny: sv(&["spawn", "filesystem"]),
            },
        ];

        for limits in combos {
            let child = ToolPolicy::resolve_child(&parent, &limits);
            for tool in &child {
                assert!(
                    parent.contains(tool),
                    "violation: child tool '{}' not in parent (limits: {:?})",
                    tool,
                    limits
                );
            }
        }
    }

    #[test]
    fn child_empty_parent_returns_empty() {
        let parent: Vec<String> = vec![];
        let child_limits = ToolLimits {
            allow: sv(&["shell"]),
            deny: vec![],
        };

        let result = ToolPolicy::resolve_child(&parent, &child_limits);
        assert!(result.is_empty());
    }

    #[test]
    fn child_deny_only_removes_from_parent() {
        let parent = sv(&["filesystem", "shell", "web_search"]);
        let child_limits = ToolLimits {
            allow: vec![],
            deny: sv(&["shell"]),
        };

        let result = ToolPolicy::resolve_child(&parent, &child_limits);
        assert_eq!(result, sv(&["filesystem", "web_search"]));
    }

    #[test]
    fn child_deny_tool_not_in_parent_is_harmless() {
        let parent = sv(&["filesystem", "shell"]);
        let child_limits = ToolLimits {
            allow: vec![],
            deny: sv(&["spawn", "web_search"]),
        };

        let result = ToolPolicy::resolve_child(&parent, &child_limits);
        assert_eq!(result, sv(&["filesystem", "shell"]));
    }

    #[test]
    fn child_deterministic_ordering() {
        let parent = sv(&["zebra", "alpha", "mike", "bravo"]);
        let child_limits = ToolLimits::default();

        let first = ToolPolicy::resolve_child(&parent, &child_limits);
        let second = ToolPolicy::resolve_child(&parent, &child_limits);
        assert_eq!(first, second);
        assert_eq!(first, sv(&["alpha", "bravo", "mike", "zebra"]));
    }

    // ── is_tool_allowed tests ──────────────────────────────────────────

    #[test]
    fn is_allowed_no_limits() {
        let global = sv(&["filesystem", "shell"]);
        let limits = ToolLimits::default();

        assert!(ToolPolicy::is_tool_allowed("shell", &global, &limits));
    }

    #[test]
    fn is_allowed_tool_not_in_global() {
        let global = sv(&["filesystem"]);
        let limits = ToolLimits::default();

        assert!(!ToolPolicy::is_tool_allowed("shell", &global, &limits));
    }

    #[test]
    fn is_allowed_tool_denied() {
        let global = sv(&["filesystem", "shell"]);
        let limits = ToolLimits {
            allow: vec![],
            deny: sv(&["shell"]),
        };

        assert!(!ToolPolicy::is_tool_allowed("shell", &global, &limits));
    }

    #[test]
    fn is_allowed_tool_not_in_allow_list() {
        let global = sv(&["filesystem", "shell"]);
        let limits = ToolLimits {
            allow: sv(&["filesystem"]),
            deny: vec![],
        };

        assert!(!ToolPolicy::is_tool_allowed("shell", &global, &limits));
    }

    // ── is_read_only_mode tests ─────────────────────────────────────────

    #[test]
    fn reviewer_mask_with_assist_mode_is_read_only() {
        use agent_diva_core::config::schema::{AgentMode, MaskConfig};

        let mask = MaskFile {
            frontmatter: MaskConfig {
                name: "Reviewer".to_string(),
                mode: Some(AgentMode::Assist),
                ..Default::default()
            },
            body: String::new(),
        };

        assert!(ToolPolicy::is_read_only_mode(&mask));
    }

    #[test]
    fn normal_mask_is_not_read_only() {
        use agent_diva_core::config::schema::{AgentMode, MaskConfig};

        let mask = MaskFile {
            frontmatter: MaskConfig {
                name: "Default".to_string(),
                mode: Some(AgentMode::Normal),
                ..Default::default()
            },
            body: String::new(),
        };

        assert!(!ToolPolicy::is_read_only_mode(&mask));
    }

    #[test]
    fn is_read_only_mode_with_none_mode_returns_false() {
        use agent_diva_core::config::schema::MaskConfig;

        let mask = MaskFile {
            frontmatter: MaskConfig {
                name: "NoMode".to_string(),
                mode: None,
                ..Default::default()
            },
            body: String::new(),
        };

        assert!(!ToolPolicy::is_read_only_mode(&mask));
    }

    #[test]
    fn default_mask_is_not_read_only() {
        let mask = MaskFile::default_mask();
        assert!(!ToolPolicy::is_read_only_mode(&mask));
    }

    // ── filter_read_only_tools tests ────────────────────────────────────

    #[test]
    fn filter_read_only_tools_removes_write_tools() {
        let tools = sv(&[
            "read_file",
            "search_files",
            "web_search",
            "web_extract",
            "terminal",
            "write_file",
            "patch",
        ]);

        let filtered = ToolPolicy::filter_read_only_tools(&tools);

        assert_eq!(filtered, sv(&["read_file", "search_files", "web_search", "web_extract"]));
    }

    #[test]
    fn filter_read_only_tools_keeps_only_read_tools() {
        let tools = sv(&[
            "terminal",
            "write_file",
            "patch",
            "spawn",
            "mcp",
        ]);

        let filtered = ToolPolicy::filter_read_only_tools(&tools);
        assert!(filtered.is_empty());
    }

    #[test]
    fn filter_read_only_tools_preserves_order() {
        let tools = sv(&["web_search", "terminal", "read_file", "write_file", "search_files"]);

        let filtered = ToolPolicy::filter_read_only_tools(&tools);

        // Input order is preserved since we iterate linearly.
        assert_eq!(filtered, sv(&["web_search", "read_file", "search_files"]));
    }

    #[test]
    fn filter_read_only_tools_empty_input() {
        let tools: Vec<String> = vec![];
        let filtered = ToolPolicy::filter_read_only_tools(&tools);
        assert!(filtered.is_empty());
    }
}
