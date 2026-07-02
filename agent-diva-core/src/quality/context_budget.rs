//! Context budget policy for quality management.
//!
//! Defines per-section token budgets and overflow handling strategies
//! for context assembly in agent sessions.

use serde::{Deserialize, Serialize};

/// Action to take when a context section exceeds its budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OverflowAction {
    /// Log a warning but allow the overflow.
    #[default]
    Warn,
    /// Truncate the section to fit within budget.
    Truncate,
    /// Block the operation that would cause overflow.
    Block,
    /// Consolidate or compress the section to reduce token count.
    Consolidate,
}

/// Per-section context budget policy.
///
/// Defines token budgets for each context section and the action
/// to take when a section exceeds its allocated budget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextBudgetPolicy {
    /// Token budget for the system prompt section.
    pub system_section_budget: u64,
    /// Token budget for the memory/context section.
    pub memory_section_budget: u64,
    /// Token budget for the skills section.
    pub skills_section_budget: u64,
    /// Token budget for the tool definitions section.
    pub tool_section_budget: u64,
    /// Total token budget for subagent context.
    pub subagent_total_budget: u64,
    /// Token budget for the output/response section.
    pub output_budget: u64,
    /// Action to take when any section exceeds its budget.
    pub overflow_action: OverflowAction,
}

impl Default for ContextBudgetPolicy {
    fn default() -> Self {
        ContextBudgetPolicy {
            system_section_budget: 2048,
            memory_section_budget: 4096,
            skills_section_budget: 1024,
            tool_section_budget: 2048,
            subagent_total_budget: 8192,
            output_budget: 2048,
            overflow_action: OverflowAction::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overflow_action_default() {
        assert_eq!(OverflowAction::default(), OverflowAction::Warn);
    }

    #[test]
    fn test_context_budget_policy_default_values() {
        let policy = ContextBudgetPolicy::default();
        assert_eq!(policy.system_section_budget, 2048);
        assert_eq!(policy.memory_section_budget, 4096);
        assert_eq!(policy.skills_section_budget, 1024);
        assert_eq!(policy.tool_section_budget, 2048);
        assert_eq!(policy.subagent_total_budget, 8192);
        assert_eq!(policy.output_budget, 2048);
        assert_eq!(policy.overflow_action, OverflowAction::Warn);
    }

    #[test]
    fn test_overflow_action_serialization() {
        let actions = vec![
            OverflowAction::Warn,
            OverflowAction::Truncate,
            OverflowAction::Block,
            OverflowAction::Consolidate,
        ];

        for action in &actions {
            let json = serde_json::to_string(action).expect("serialize");
            let deserialized: OverflowAction = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*action, deserialized);
        }
    }

    #[test]
    fn test_context_budget_policy_serialization() {
        let policy = ContextBudgetPolicy::default();
        let json = serde_json::to_string(&policy).expect("serialize");
        let deserialized: ContextBudgetPolicy = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(policy, deserialized);
    }

    #[test]
    fn test_context_budget_policy_yaml_serialization() {
        let policy = ContextBudgetPolicy::default();
        let yaml = serde_yaml::to_string(&policy).expect("serialize to yaml");
        let deserialized: ContextBudgetPolicy =
            serde_yaml::from_str(&yaml).expect("deserialize from yaml");
        assert_eq!(policy, deserialized);
    }

    #[test]
    fn test_custom_context_budget_policy() {
        let policy = ContextBudgetPolicy {
            system_section_budget: 1024,
            memory_section_budget: 2048,
            skills_section_budget: 512,
            tool_section_budget: 1024,
            subagent_total_budget: 4096,
            output_budget: 1024,
            overflow_action: OverflowAction::Block,
        };

        assert_eq!(policy.system_section_budget, 1024);
        assert_eq!(policy.memory_section_budget, 2048);
        assert_eq!(policy.skills_section_budget, 512);
        assert_eq!(policy.tool_section_budget, 1024);
        assert_eq!(policy.subagent_total_budget, 4096);
        assert_eq!(policy.output_budget, 1024);
        assert_eq!(policy.overflow_action, OverflowAction::Block);
    }
}
