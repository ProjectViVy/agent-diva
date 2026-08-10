use std::collections::HashMap;

use agent_diva_providers::{PromptCachePolicy, PromptCacheProfile};
use agent_diva_tooling::ToolDefinitionSet;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};

use super::CacheBreakReason;
use crate::token_estimate::estimate_tokens;

const MIN_CACHEABLE_PREFIX_TOKENS: usize = 2_000;
const MIN_SIGNIFICANT_MISS_TOKENS: i64 = 2_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreCallClassification {
    Baseline,
    Stable,
    ExpectedBreak,
    PolicyBreak,
    UndeclaredStructuralBreak,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PostCallClassification {
    SampleOnly,
    Warmup,
    Hit,
    MissSample,
    SuspectedCacheMiss,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CacheObservationTicket(u64);

pub struct CacheObserveInput<'a> {
    pub session_id: &'a str,
    pub model: &'a str,
    pub profile: &'a PromptCacheProfile,
    pub stable_prefix: &'a str,
    pub prefix_version: u64,
    pub break_reasons: &'a [CacheBreakReason],
    pub tools: &'a ToolDefinitionSet,
    pub expected_deletion: bool,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct BucketKey {
    provider: String,
    model: String,
    policy: PromptCachePolicy,
    ttl: String,
}

#[derive(Default)]
struct BucketState {
    last_system_hash: Option<String>,
    last_tools_hash: Option<String>,
    last_prefix_version: Option<u64>,
    last_tool_names: Vec<String>,
    warmup_pending: bool,
    consecutive_misses: u8,
}

#[derive(Default)]
struct SessionState {
    last_bucket: Option<BucketKey>,
    buckets: HashMap<BucketKey, BucketState>,
}

struct PendingObservation {
    session_id: String,
    bucket: BucketKey,
    stable_prefix_tokens: usize,
    structurally_stable: bool,
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
        let bucket_key = BucketKey {
            provider: input.profile.provider.clone(),
            model: input.model.to_string(),
            policy: input.profile.policy,
            ttl: input.profile.ttl.clone(),
        };
        let system_hash = hash_bytes(input.stable_prefix.as_bytes());
        let tools_hash = hash_json(&input.tools.definitions);
        let tool_names = input
            .tools
            .definitions
            .iter()
            .filter_map(tool_name)
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let per_tool_hashes = input
            .tools
            .definitions
            .iter()
            .map(|tool| (tool_name(tool).unwrap_or("unknown"), hash_json(tool)))
            .collect::<Vec<_>>();

        let session = self
            .sessions
            .entry(input.session_id.to_string())
            .or_default();
        let policy_changed = session
            .last_bucket
            .as_ref()
            .is_some_and(|last| last != &bucket_key);
        let bucket = session.buckets.entry(bucket_key.clone()).or_default();
        let baseline = bucket.last_system_hash.is_none();
        let system_changed = bucket
            .last_system_hash
            .as_ref()
            .is_some_and(|last| last != &system_hash);
        let tools_changed = bucket
            .last_tools_hash
            .as_ref()
            .is_some_and(|last| last != &tools_hash);
        let version_advanced = bucket
            .last_prefix_version
            .is_some_and(|last| last != input.prefix_version);
        let active_break_reasons = if version_advanced {
            input.break_reasons
        } else {
            &[]
        };
        let declared_prefix_break = !active_break_reasons.is_empty();
        let declared_tool_set_break = tools_changed && bucket.last_tool_names != tool_names;
        let expected_break = input.expected_deletion
            || (system_changed && declared_prefix_break)
            || declared_tool_set_break;
        let undeclared_break = (system_changed || tools_changed) && !expected_break;

        let classification = if policy_changed {
            bucket.warmup_pending = true;
            info!(
                session_id = input.session_id,
                provider = %input.profile.provider,
                model = input.model,
                policy = input.profile.policy.as_str(),
                ttl = %input.profile.ttl,
                classification = "policy_break",
                "prompt cache policy bucket changed"
            );
            PreCallClassification::PolicyBreak
        } else if baseline {
            bucket.warmup_pending = true;
            PreCallClassification::Baseline
        } else if expected_break {
            bucket.warmup_pending = true;
            info!(
                session_id = input.session_id,
                reasons = ?active_break_reasons,
                expected_deletion = input.expected_deletion,
                classification = if input.expected_deletion { "expected_deletion" } else { "expected_break" },
                "prompt cache structure changed as declared"
            );
            PreCallClassification::ExpectedBreak
        } else if undeclared_break {
            bucket.warmup_pending = true;
            warn!(
                session_id = input.session_id,
                system_changed,
                tools_changed,
                classification = "undeclared_structural_break",
                "prompt cache structure changed without a declared reason"
            );
            PreCallClassification::UndeclaredStructuralBreak
        } else {
            PreCallClassification::Stable
        };

        debug!(
            session_id = input.session_id,
            provider = %input.profile.provider,
            model = input.model,
            policy = input.profile.policy.as_str(),
            ttl = %input.profile.ttl,
            system_hash = %system_hash,
            tools_hash = %tools_hash,
            per_tool_hashes = ?per_tool_hashes,
            prefix_version = input.prefix_version,
            stable_prefix_tokens = estimate_tokens(input.stable_prefix),
            break_reasons = ?active_break_reasons,
            cache_read = tracing::field::Empty,
            cache_creation = tracing::field::Empty,
            "prompt_cache_sample"
        );

        bucket.last_system_hash = Some(system_hash);
        bucket.last_tools_hash = Some(tools_hash);
        bucket.last_prefix_version = Some(input.prefix_version);
        bucket.last_tool_names = tool_names;
        session.last_bucket = Some(bucket_key.clone());

        self.next_ticket = self.next_ticket.saturating_add(1);
        let ticket = CacheObservationTicket(self.next_ticket);
        self.pending.insert(
            ticket.0,
            PendingObservation {
                session_id: input.session_id.to_string(),
                bucket: bucket_key,
                stable_prefix_tokens: estimate_tokens(input.stable_prefix),
                structurally_stable: classification == PreCallClassification::Stable,
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
            return PostCallClassification::SampleOnly;
        };
        let Some(session) = self.sessions.get_mut(&pending.session_id) else {
            return PostCallClassification::SampleOnly;
        };
        let Some(bucket) = session.buckets.get_mut(&pending.bucket) else {
            return PostCallClassification::SampleOnly;
        };
        let cache_read = usage.get("cache_read_input_tokens").copied();
        let cache_creation = usage.get("cache_creation_input_tokens").copied();
        debug!(
            session_id = %pending.session_id,
            provider = %pending.bucket.provider,
            model = %pending.bucket.model,
            cache_read = ?cache_read,
            cache_creation = ?cache_creation,
            "prompt_cache_usage"
        );

        if pending.bucket.policy == PromptCachePolicy::Disabled
            || !pending.structurally_stable
            || cache_read.is_none()
            || pending.stable_prefix_tokens < MIN_CACHEABLE_PREFIX_TOKENS
        {
            bucket.consecutive_misses = 0;
            return PostCallClassification::SampleOnly;
        }
        if std::mem::take(&mut bucket.warmup_pending) {
            bucket.consecutive_misses = 0;
            return PostCallClassification::Warmup;
        }

        let cache_read = cache_read.unwrap_or_default().max(0);
        let expected = pending.stable_prefix_tokens as i64;
        let significant_miss = expected.saturating_sub(cache_read) >= MIN_SIGNIFICANT_MISS_TOKENS
            && cache_read.saturating_mul(2) < expected;
        if !significant_miss {
            bucket.consecutive_misses = 0;
            return PostCallClassification::Hit;
        }

        bucket.consecutive_misses = bucket.consecutive_misses.saturating_add(1);
        if bucket.consecutive_misses >= 2 {
            warn!(
                session_id = %pending.session_id,
                provider = %pending.bucket.provider,
                model = %pending.bucket.model,
                cache_read,
                expected_prefix_tokens = expected,
                classification = "suspected_cache_miss",
                "prompt cache read missed a stable eligible prefix twice"
            );
            PostCallClassification::SuspectedCacheMiss
        } else {
            PostCallClassification::MissSample
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

fn tool_name(tool: &Value) -> Option<&str> {
    tool.get("function")
        .and_then(|function| function.get("name"))
        .and_then(Value::as_str)
        .or_else(|| tool.get("name").and_then(Value::as_str))
}

fn hash_json(value: &impl serde::Serialize) -> String {
    hash_bytes(&serde_json::to_vec(value).unwrap_or_default())
}

fn hash_bytes(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_tooling::ToolDefinitionSet;

    fn profile() -> PromptCacheProfile {
        PromptCacheProfile::ephemeral("anthropic")
    }

    fn tools() -> ToolDefinitionSet {
        ToolDefinitionSet {
            definitions: vec![
                serde_json::json!({"function":{"name":"core_a"}}),
                serde_json::json!({"function":{"name":"core_b"}}),
                serde_json::json!({"function":{"name":"mcp_a"}}),
            ],
            core_count: 2,
        }
    }

    fn observe(
        state: &mut CacheObserveState,
        prefix: &str,
        version: u64,
        reasons: &[CacheBreakReason],
        tools: &ToolDefinitionSet,
    ) -> (CacheObservationTicket, PreCallClassification) {
        let profile = profile();
        state.note_pre_call(CacheObserveInput {
            session_id: "session",
            model: "claude-sonnet-4-5",
            profile: &profile,
            stable_prefix: prefix,
            prefix_version: version,
            break_reasons: reasons,
            tools,
            expected_deletion: false,
        })
    }

    #[test]
    fn core_anchor_marks_only_the_core_endpoint() {
        let mut definitions = tools();
        apply_core_tool_cache_anchor(&mut definitions, &profile());
        assert!(definitions.definitions[0].get("cache_control").is_none());
        assert_eq!(
            definitions.definitions[1]["cache_control"]["type"],
            "ephemeral"
        );
        assert!(definitions.definitions[2].get("cache_control").is_none());
    }

    #[test]
    fn disabled_policy_and_empty_core_do_not_mark_tools() {
        let mut disabled = tools();
        apply_core_tool_cache_anchor(&mut disabled, &PromptCacheProfile::disabled("deepseek"));
        assert!(disabled
            .definitions
            .iter()
            .all(|tool| tool.get("cache_control").is_none()));

        let mut deferred_only = ToolDefinitionSet {
            definitions: vec![serde_json::json!({"function":{"name":"mcp"}})],
            core_count: 0,
        };
        apply_core_tool_cache_anchor(&mut deferred_only, &profile());
        assert!(deferred_only.definitions[0].get("cache_control").is_none());
    }

    #[test]
    fn classifies_declared_and_undeclared_structure_changes() {
        let mut state = CacheObserveState::default();
        let definitions = tools();
        assert_eq!(
            observe(&mut state, "prefix one", 1, &[], &definitions).1,
            PreCallClassification::Baseline
        );
        assert_eq!(
            observe(&mut state, "prefix one", 1, &[], &definitions).1,
            PreCallClassification::Stable
        );
        assert_eq!(
            observe(&mut state, "prefix two", 1, &[], &definitions).1,
            PreCallClassification::UndeclaredStructuralBreak
        );
        assert_eq!(
            observe(
                &mut state,
                "prefix three",
                2,
                &[CacheBreakReason::L1HotRefresh],
                &definitions,
            )
            .1,
            PreCallClassification::ExpectedBreak
        );
    }

    #[test]
    fn warns_only_after_two_stable_significant_misses() {
        let mut state = CacheObserveState::default();
        let prefix = "x".repeat(7_500);
        let definitions = tools();
        let (baseline, _) = observe(&mut state, &prefix, 1, &[], &definitions);
        assert_eq!(
            state.note_post_call(
                baseline,
                &HashMap::from([("cache_read_input_tokens".to_string(), 0)])
            ),
            PostCallClassification::SampleOnly
        );
        let (warmup, _) = observe(&mut state, &prefix, 1, &[], &definitions);
        assert_eq!(
            state.note_post_call(
                warmup,
                &HashMap::from([("cache_read_input_tokens".to_string(), 0)])
            ),
            PostCallClassification::Warmup
        );
        let (first, _) = observe(&mut state, &prefix, 1, &[], &definitions);
        assert_eq!(
            state.note_post_call(
                first,
                &HashMap::from([("cache_read_input_tokens".to_string(), 0)])
            ),
            PostCallClassification::MissSample
        );
        let (second, _) = observe(&mut state, &prefix, 1, &[], &definitions);
        assert_eq!(
            state.note_post_call(
                second,
                &HashMap::from([("cache_read_input_tokens".to_string(), 0)])
            ),
            PostCallClassification::SuspectedCacheMiss
        );
    }

    #[test]
    fn policy_changes_and_expected_deletions_are_classified_without_warning() {
        let mut state = CacheObserveState::default();
        let definitions = tools();
        observe(&mut state, "prefix", 1, &[], &definitions);

        let disabled = PromptCacheProfile::disabled("anthropic");
        let (_, policy_change) = state.note_pre_call(CacheObserveInput {
            session_id: "session",
            model: "claude-sonnet-4-5",
            profile: &disabled,
            stable_prefix: "prefix",
            prefix_version: 1,
            break_reasons: &[],
            tools: &definitions,
            expected_deletion: false,
        });
        assert_eq!(policy_change, PreCallClassification::PolicyBreak);

        let (_, deletion) = state.note_pre_call(CacheObserveInput {
            session_id: "session",
            model: "claude-sonnet-4-5",
            profile: &disabled,
            stable_prefix: "changed",
            prefix_version: 1,
            break_reasons: &[],
            tools: &definitions,
            expected_deletion: true,
        });
        assert_eq!(deletion, PreCallClassification::ExpectedBreak);
    }
}
