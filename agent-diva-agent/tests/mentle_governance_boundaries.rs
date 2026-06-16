use agent_diva_agent::tool_config::mentle::{MentleToolMode, MentleToolRuntimeConfig};

#[test]
fn mentle_v1_runtime_config_does_not_authorize_governance_writes_in_read_only_mode() {
    let config = MentleToolRuntimeConfig {
        enabled: true,
        mode: MentleToolMode::ReadOnly,
        allowed_tools: Vec::new(),
    };

    assert!(config.allows_tool("memtle_status"));
    assert!(config.allows_tool("memtle_search"));
    assert!(!config.allows_tool("memtle_diary_write"));
    assert!(!config.allows_tool("memtle_add_drawer"));
}

#[test]
fn mentle_v1_selected_tool_names_exclude_governance_ownership_paths() {
    let config = MentleToolRuntimeConfig {
        enabled: true,
        mode: MentleToolMode::ReadOnly,
        allowed_tools: Vec::new(),
    };

    let selected = config.selected_tool_names([
        "memtle_status".to_string(),
        "memtle_search".to_string(),
        "memtle_diary_write".to_string(),
        "memtle_add_drawer".to_string(),
    ]);

    assert_eq!(
        selected,
        vec!["memtle_status".to_string(), "memtle_search".to_string()]
    );
}
