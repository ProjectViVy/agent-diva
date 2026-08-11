use std::sync::Arc;

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::base::PromptCacheProfile;

/// Cache-relevant structure captured after provider request shaping.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FinalWireCacheSnapshot {
    pub provider_namespace: String,
    pub model: String,
    pub profile: PromptCacheProfile,
    pub stable_system_hash: String,
    pub core_tools_hash: String,
    pub active_tool_names: Vec<String>,
    pub stable_prefix_tokens: usize,
}

/// Listener invoked once for a fully shaped outbound provider request.
pub type FinalWireCacheListener = Arc<dyn Fn(FinalWireCacheSnapshot) + Send + Sync>;

pub(crate) fn snapshot_from_wire(
    provider_namespace: impl Into<String>,
    model: impl Into<String>,
    profile: PromptCacheProfile,
    stable_system: &Value,
    tools: &[Value],
) -> FinalWireCacheSnapshot {
    let core_end = tools
        .iter()
        .position(|tool| tool.get("cache_control").is_some())
        .map_or(0, |index| index + 1);
    let active_tool_names = tools
        .iter()
        .filter_map(tool_name)
        .map(str::to_owned)
        .collect();
    let stable_bytes = serde_json::to_vec(stable_system).unwrap_or_default();
    FinalWireCacheSnapshot {
        provider_namespace: provider_namespace.into(),
        model: model.into(),
        profile,
        stable_system_hash: hash_bytes(&stable_bytes),
        core_tools_hash: hash_json(&tools[..core_end]),
        active_tool_names,
        stable_prefix_tokens: stable_bytes.len().div_ceil(4),
    }
}

fn tool_name(tool: &Value) -> Option<&str> {
    tool.get("function")
        .and_then(|function| function.get("name"))
        .and_then(Value::as_str)
        .or_else(|| tool.get("name").and_then(Value::as_str))
}

fn hash_json(value: &(impl serde::Serialize + ?Sized)) -> String {
    hash_bytes(&serde_json::to_vec(value).unwrap_or_default())
}

fn hash_bytes(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tools(deferred_description: &str) -> Vec<Value> {
        vec![
            json!({"type":"function","function":{"name":"core_a"}}),
            json!({"type":"function","function":{"name":"core_b"},"cache_control":{"type":"ephemeral"}}),
            json!({"type":"function","function":{"name":"mcp_a","description":deferred_description}}),
        ]
    }

    #[test]
    fn deferred_suffix_does_not_change_core_prefix_hash() {
        let first = snapshot_from_wire(
            "anthropic",
            "claude-sonnet-4-5",
            PromptCacheProfile::ephemeral("anthropic"),
            &json!({"role":"system","content":"stable"}),
            &tools("one"),
        );
        let second = snapshot_from_wire(
            "anthropic",
            "claude-sonnet-4-5",
            PromptCacheProfile::ephemeral("anthropic"),
            &json!({"role":"system","content":"stable"}),
            &tools("two"),
        );
        assert_eq!(first.stable_system_hash, second.stable_system_hash);
        assert_eq!(first.core_tools_hash, second.core_tools_hash);
        assert_eq!(first.active_tool_names, second.active_tool_names);
    }

    #[test]
    fn final_wire_system_and_core_anchor_changes_are_visible() {
        let baseline = snapshot_from_wire(
            "anthropic",
            "claude-sonnet-4-5",
            PromptCacheProfile::ephemeral("anthropic"),
            &json!({"role":"system","content":"stable"}),
            &tools("one"),
        );
        let changed_system = snapshot_from_wire(
            "anthropic",
            "claude-sonnet-4-5",
            PromptCacheProfile::ephemeral("anthropic"),
            &json!({"role":"system","content":"changed"}),
            &tools("one"),
        );
        let changed_core = snapshot_from_wire(
            "anthropic",
            "claude-sonnet-4-5",
            PromptCacheProfile::ephemeral("anthropic"),
            &json!({"role":"system","content":"stable"}),
            &tools("one")[..1],
        );
        assert_ne!(
            baseline.stable_system_hash,
            changed_system.stable_system_hash
        );
        assert_ne!(baseline.core_tools_hash, changed_core.core_tools_hash);
    }

    #[test]
    fn raw_model_and_provider_namespace_are_preserved() {
        let snapshot = snapshot_from_wire(
            "deepseek",
            "deepseek-chat",
            PromptCacheProfile::disabled("deepseek"),
            &Value::Null,
            &[],
        );
        assert_eq!(snapshot.provider_namespace, "deepseek");
        assert_eq!(snapshot.model, "deepseek-chat");
    }
}
