use crate::config::schema::{Config, ProviderConfig};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReloadPolicy {
    HotReload,
    RestartRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConfigDiff {
    pub hot_reload_changes: Vec<String>,
    pub restart_required_changes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReloadPlan {
    pub restart_required: bool,
    pub diff: ConfigDiff,
}

impl ReloadPlan {
    pub fn from_configs(old: &Config, new: &Config) -> crate::Result<Self> {
        let diff = compute_config_diff(old, new)?;
        Ok(Self {
            restart_required: !diff.restart_required_changes.is_empty(),
            diff,
        })
    }
}

pub fn compute_config_diff(old: &Config, new: &Config) -> crate::Result<ConfigDiff> {
    let old_value = serde_json::to_value(old)?;
    let new_value = serde_json::to_value(new)?;
    let mut changed_paths = Vec::new();
    collect_changed_paths("", &old_value, &new_value, &mut changed_paths);
    changed_paths.sort();
    changed_paths.dedup();

    let mut hot_reload_changes = Vec::new();
    let mut restart_required_changes = Vec::new();
    for path in changed_paths {
        match classify_reload_policy(&path) {
            ReloadPolicy::HotReload => hot_reload_changes.push(path),
            ReloadPolicy::RestartRequired => restart_required_changes.push(path),
        }
    }

    Ok(ConfigDiff {
        hot_reload_changes,
        restart_required_changes,
    })
}

fn collect_changed_paths(path: &str, old: &Value, new: &Value, changed_paths: &mut Vec<String>) {
    match (old, new) {
        (Value::Object(old_map), Value::Object(new_map)) => {
            let mut keys: Vec<&str> = old_map
                .keys()
                .chain(new_map.keys())
                .map(String::as_str)
                .collect();
            keys.sort_unstable();
            keys.dedup();

            for key in keys {
                let next_path = join_path(path, key);
                match (old_map.get(key), new_map.get(key)) {
                    (Some(old_value), Some(new_value)) => {
                        collect_changed_paths(&next_path, old_value, new_value, changed_paths);
                    }
                    _ => changed_paths.push(next_path),
                }
            }
        }
        _ if old != new => changed_paths.push(path.to_string()),
        _ => {}
    }
}

fn join_path(prefix: &str, segment: &str) -> String {
    if prefix.is_empty() {
        segment.to_string()
    } else {
        format!("{prefix}.{segment}")
    }
}

fn classify_reload_policy(path: &str) -> ReloadPolicy {
    if is_hot_reload_path(path) {
        ReloadPolicy::HotReload
    } else {
        ReloadPolicy::RestartRequired
    }
}

fn is_hot_reload_path(path: &str) -> bool {
    matches!(
        path,
        "logging.level"
            | "tools.exec.timeout"
            | "security.level"
            | "security.workspace_only"
            | "security.max_actions_per_hour"
            | "heartbeat.enabled"
            | "heartbeat.interval_s"
    ) || path.starts_with("presence.")
        || path.starts_with("audit.")
        || path.starts_with("pii.")
        || path.starts_with("injection.")
        || path.starts_with("tools.mcp_servers.")
        || path.starts_with("tools.mcp_manager.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_config_diff_classifies_hot_and_restart_changes() {
        let old = Config::default();
        let mut new = old.clone();
        new.logging.level = "debug".to_string();
        new.tools.exec.timeout = 120;
        new.presence.active_timeout_s += 1;
        new.heartbeat.interval_s += 1;
        new.audit.emit_presence_changed = !new.audit.emit_presence_changed;
        new.pii.redact_email = !new.pii.redact_email;
        new.injection.detect_tool_abuse = !new.injection.detect_tool_abuse;
        new.gateway.port += 1;

        let diff = compute_config_diff(&old, &new).unwrap();

        assert!(diff
            .hot_reload_changes
            .contains(&"logging.level".to_string()));
        assert!(diff
            .hot_reload_changes
            .contains(&"tools.exec.timeout".to_string()));
        assert!(diff
            .hot_reload_changes
            .contains(&"presence.active_timeout_s".to_string()));
        assert!(diff
            .hot_reload_changes
            .contains(&"heartbeat.interval_s".to_string()));
        assert!(diff
            .hot_reload_changes
            .contains(&"audit.emit_presence_changed".to_string()));
        assert!(diff
            .hot_reload_changes
            .contains(&"pii.redact_email".to_string()));
        assert!(diff
            .hot_reload_changes
            .contains(&"injection.detect_tool_abuse".to_string()));
        assert!(diff
            .restart_required_changes
            .contains(&"gateway.port".to_string()));
    }

    #[test]
    fn reload_plan_marks_restart_required_when_restart_paths_change() {
        let old = Config::default();
        let mut new = old.clone();
        new.providers.openai_compatible = Some(ProviderConfig {
            api_key: "sk-updated".to_string(),
            ..Default::default()
        });

        let plan = ReloadPlan::from_configs(&old, &new).unwrap();

        assert!(plan.restart_required);
        assert_eq!(plan.diff.hot_reload_changes, Vec::<String>::new());
        // Since openai_compatible is Option<ProviderConfig>, the change from
        // None to Some is reported at the field level (not sub-field).
        assert_eq!(
            plan.diff.restart_required_changes,
            vec!["providers.openai_compatible".to_string()]
        );
    }

    #[test]
    fn compute_config_diff_returns_empty_for_identical_configs() {
        let config = Config::default();

        let diff = compute_config_diff(&config, &config).unwrap();

        assert!(diff.hot_reload_changes.is_empty());
        assert!(diff.restart_required_changes.is_empty());
    }
}
