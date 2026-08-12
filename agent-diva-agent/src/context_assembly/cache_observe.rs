use std::collections::HashMap;

use agent_diva_providers::{FinalWireCacheSnapshot, PromptCachePolicy, PromptCacheProfile};
use agent_diva_tooling::ToolDefinitionSet;
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};

use super::CacheBreakReason;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreCallClassification {
    Baseline,
    Stable,
    ExpectedBreak,
    PolicyBreak,
    UndeclaredStructuralBreak,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PostCallClassification {
    UsageAvailable,
    UsageUnavailable,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CacheObservationTicket(u64);

pub struct CacheObserveInput<'a> {
    pub session_id: &'a str,
    pub snapshot: Option<FinalWireCacheSnapshot>,
    pub break_reasons: &'a [CacheBreakReason],
    pub expected_deletion: bool,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct BucketKey {
    provider_namespace: String,
    model: String,
    policy: PromptCachePolicy,
}

#[derive(Default)]
struct BucketState {
    last_stable_system_hash: Option<String>,
    last_core_tools_hash: Option<String>,
    last_active_tool_names: Vec<String>,
}

#[derive(Default)]
struct SessionState {
    last_bucket: Option<BucketKey>,
    buckets: HashMap<BucketKey, BucketState>,
}

struct PendingObservation {
    session_id: String,
    bucket: Option<BucketKey>,
}

#[derive(Default)]
pub struct CacheObserveState {
    sessions: HashMap<String, SessionState>,
    pending: HashMap<u64, PendingObservation>,
    next_ticket: u64,
}

impl CacheObserveState {
    pub fn note_pre_call(
        &mut self,
        input: CacheObserveInput<'_>,
    ) -> (CacheObservationTicket, PreCallClassification) {
        let (bucket_key, classification) = if let Some(snapshot) = input.snapshot {
            let bucket_key = BucketKey {
                provider_namespace: snapshot.provider_namespace.clone(),
                model: snapshot.model.clone(),
                policy: snapshot.profile.policy,
            };
            let session = self
                .sessions
                .entry(input.session_id.to_owned())
                .or_default();
            let policy_changed = session
                .last_bucket
                .as_ref()
                .is_some_and(|last| last != &bucket_key);
            let bucket = session.buckets.entry(bucket_key.clone()).or_default();
            let baseline = bucket.last_stable_system_hash.is_none();
            let system_changed = bucket
                .last_stable_system_hash
                .as_ref()
                .is_some_and(|last| last != &snapshot.stable_system_hash);
            let core_tools_changed = bucket
                .last_core_tools_hash
                .as_ref()
                .is_some_and(|last| last != &snapshot.core_tools_hash);
            let active_tools_changed = bucket.last_active_tool_names != snapshot.active_tool_names;
            let declared_break = !input.break_reasons.is_empty();
            let expected_break = input.expected_deletion
                || active_tools_changed
                || (system_changed && declared_break);
            let undeclared_break = (system_changed || core_tools_changed) && !expected_break;
            let classification = if policy_changed {
                PreCallClassification::PolicyBreak
            } else if baseline {
                PreCallClassification::Baseline
            } else if expected_break {
                PreCallClassification::ExpectedBreak
            } else if undeclared_break {
                PreCallClassification::UndeclaredStructuralBreak
            } else {
                PreCallClassification::Stable
            };

            match classification {
                PreCallClassification::PolicyBreak | PreCallClassification::ExpectedBreak => info!(
                    session_id = input.session_id,
                    provider = %snapshot.provider_namespace,
                    model = %snapshot.model,
                    reasons = ?input.break_reasons,
                    expected_deletion = input.expected_deletion,
                    "prompt cache final-wire structure changed as declared"
                ),
                PreCallClassification::UndeclaredStructuralBreak => warn!(
                    session_id = input.session_id,
                    system_changed,
                    core_tools_changed,
                    "prompt cache final-wire structure changed without a declared reason"
                ),
                _ => {}
            }
            debug!(
                session_id = input.session_id,
                provider = %snapshot.provider_namespace,
                model = %snapshot.model,
                policy = snapshot.profile.policy.as_str(),
                stable_system_hash = %snapshot.stable_system_hash,
                core_tools_hash = %snapshot.core_tools_hash,
                stable_prefix_tokens = snapshot.stable_prefix_tokens,
                "prompt_cache_final_wire_sample"
            );
            bucket.last_stable_system_hash = Some(snapshot.stable_system_hash);
            bucket.last_core_tools_hash = Some(snapshot.core_tools_hash);
            bucket.last_active_tool_names = snapshot.active_tool_names;
            session.last_bucket = Some(bucket_key.clone());
            (Some(bucket_key), classification)
        } else {
            debug!(
                session_id = input.session_id,
                "provider final-wire cache snapshot unavailable"
            );
            (None, PreCallClassification::Unavailable)
        };

        self.next_ticket = self.next_ticket.saturating_add(1);
        let ticket = CacheObservationTicket(self.next_ticket);
        self.pending.insert(
            ticket.0,
            PendingObservation {
                session_id: input.session_id.to_owned(),
                bucket: bucket_key,
            },
        );
        (ticket, classification)
    }

    pub fn note_post_call(
        &mut self,
        ticket: CacheObservationTicket,
        usage: &HashMap<String, i64>,
    ) -> PostCallClassification {
        let Some(pending) = self.pending.remove(&ticket.0) else {
            return PostCallClassification::UsageUnavailable;
        };
        let cache_read = usage.get("cache_read_input_tokens").copied();
        let cache_creation = usage.get("cache_creation_input_tokens").copied();
        debug!(
            session_id = %pending.session_id,
            bucket = ?pending.bucket,
            cache_read = ?cache_read,
            cache_creation = ?cache_creation,
            "prompt_cache_usage"
        );
        if cache_read.is_some() || cache_creation.is_some() {
            PostCallClassification::UsageAvailable
        } else {
            PostCallClassification::UsageUnavailable
        }
    }

    pub fn abandon_call(&mut self, ticket: CacheObservationTicket) {
        self.pending.remove(&ticket.0);
    }

    pub fn clear_session(&mut self, session_id: &str) {
        self.sessions.remove(session_id);
        self.pending
            .retain(|_, pending| pending.session_id != session_id);
    }

    pub fn clear(&mut self) {
        self.sessions.clear();
        self.pending.clear();
    }
}

pub fn apply_core_tool_cache_anchor(tools: &mut ToolDefinitionSet, profile: &PromptCacheProfile) {
    if !profile.is_enabled() || tools.core_count == 0 {
        return;
    }
    if let Some(tool) = tools.definitions.get_mut(tools.core_count - 1) {
        tool["cache_control"] = serde_json::json!({"type": "ephemeral"});
    }
}

/// Hash the canonical CORE prefix captured at the agent boundary.
///
/// Providers that do not expose a cache anchor still need a structural key so
/// deferred activation cannot perturb the CORE cache identity.  The caller
/// passes the pre-provider `ToolDefinitionSet`, whose `core_count` is the
/// authoritative partition boundary.
pub fn core_tool_hash(tools: &ToolDefinitionSet) -> String {
    let end = tools.core_count.min(tools.definitions.len());
    let bytes = serde_json::to_vec(&tools.definitions[..end]).unwrap_or_default();
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(system: &str, core: &str, active: &[&str]) -> FinalWireCacheSnapshot {
        FinalWireCacheSnapshot {
            provider_namespace: "anthropic".into(),
            model: "claude-sonnet-4-5".into(),
            profile: PromptCacheProfile::ephemeral("anthropic"),
            stable_system_hash: system.into(),
            core_tools_hash: core.into(),
            active_tool_names: active.iter().map(|name| (*name).into()).collect(),
            stable_prefix_tokens: 2_500,
        }
    }

    fn observe(
        state: &mut CacheObserveState,
        snapshot: FinalWireCacheSnapshot,
        reasons: &[CacheBreakReason],
    ) -> (CacheObservationTicket, PreCallClassification) {
        state.note_pre_call(CacheObserveInput {
            session_id: "session",
            snapshot: Some(snapshot),
            break_reasons: reasons,
            expected_deletion: false,
        })
    }

    #[test]
    fn deferred_tool_changes_are_declared_without_changing_core_prefix() {
        let mut state = CacheObserveState::default();
        assert_eq!(
            observe(&mut state, snapshot("s", "c", &["core"]), &[]).1,
            PreCallClassification::Baseline
        );
        assert_eq!(
            observe(&mut state, snapshot("s", "c", &["core", "mcp"]), &[]).1,
            PreCallClassification::ExpectedBreak
        );
        assert_eq!(
            observe(&mut state, snapshot("s", "c", &["core", "mcp"]), &[]).1,
            PreCallClassification::Stable
        );
    }

    #[test]
    fn undeclared_final_wire_change_is_reported() {
        let mut state = CacheObserveState::default();
        observe(&mut state, snapshot("s1", "c", &["core"]), &[]);
        assert_eq!(
            observe(&mut state, snapshot("s2", "c", &["core"]), &[]).1,
            PreCallClassification::UndeclaredStructuralBreak
        );
    }

    #[test]
    fn usage_is_never_inferred_when_provider_omits_cache_counters() {
        let mut state = CacheObserveState::default();
        let (ticket, _) = observe(&mut state, snapshot("s", "c", &["core"]), &[]);
        assert_eq!(
            state.note_post_call(ticket, &HashMap::new()),
            PostCallClassification::UsageUnavailable
        );
    }

    #[test]
    fn core_hash_uses_only_the_explicit_core_prefix() {
        let first = ToolDefinitionSet {
            definitions: vec![
                serde_json::json!({"name":"core"}),
                serde_json::json!({"name":"deferred-a"}),
            ],
            core_count: 1,
        };
        let second = ToolDefinitionSet {
            definitions: vec![
                serde_json::json!({"name":"core"}),
                serde_json::json!({"name":"deferred-b"}),
            ],
            core_count: 1,
        };
        assert_eq!(core_tool_hash(&first), core_tool_hash(&second));
        let mut changed = first.clone();
        changed.definitions[0]["description"] = serde_json::json!("changed");
        assert_ne!(core_tool_hash(&first), core_tool_hash(&changed));
    }

    #[test]
    fn retired_guessing_state_cannot_reenter_the_observer() {
        let source = include_str!("cache_observe.rs");
        let retired = [
            ["warmup", "_pending"].concat(),
            ["consecutive", "_misses"].concat(),
            ["Suspected", "CacheMiss"].concat(),
            ["last", "_tools_hash"].concat(),
            ["per", "_tool_hashes"].concat(),
        ];
        for symbol in retired {
            assert!(
                !source.contains(&symbol),
                "retired symbol returned: {symbol}"
            );
        }
    }
}
