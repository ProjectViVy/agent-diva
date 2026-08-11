//! Typed, layer-aware context budgeting and assembly reporting.

use std::collections::BTreeMap;

use agent_diva_providers::Message;
use agent_diva_tooling::ToolDefinitionSet;

use crate::context_budget::BudgetConfig;
use crate::token_estimate::estimate_tokens;

use super::{ContextSection, PromptSection, SectionStability};

/// Independently measured context budget buckets.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum BudgetLayer {
    FrozenCoreAndRules,
    ToolSchemasCore,
    ToolSchemasDeferred,
    L1Index,
    WorkingMemory,
    PrefetchRecall,
    CompactionSummaries,
    History,
    ToolResultInline,
    CurrentTurn,
}

impl BudgetLayer {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FrozenCoreAndRules => "frozen_core_and_rules",
            Self::ToolSchemasCore => "tool_schemas_core",
            Self::ToolSchemasDeferred => "tool_schemas_deferred",
            Self::L1Index => "l1_index",
            Self::WorkingMemory => "working_memory",
            Self::PrefetchRecall => "prefetch_recall",
            Self::CompactionSummaries => "compaction_summaries",
            Self::History => "history",
            Self::ToolResultInline => "tool_result_inline",
            Self::CurrentTurn => "current_turn",
        }
    }
}

/// Action permitted when a fragment exceeds its layer budget.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvictionPolicy {
    Never,
    Compact,
    Drop,
    LegacyCountCap,
}

/// Why a fragment was omitted or reduced.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssemblyDecisionReason {
    LayerSoftLimit,
    TotalHardLimit,
    LegacyCountCap,
    MacroCompaction,
    Microcompact,
}

impl AssemblyDecisionReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LayerSoftLimit => "layer_soft_limit",
            Self::TotalHardLimit => "total_hard_limit",
            Self::LegacyCountCap => "legacy_count_cap",
            Self::MacroCompaction => "macro_compaction",
            Self::Microcompact => "microcompact",
        }
    }
}

/// One typed input to context assembly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextFragment {
    pub id: String,
    pub section: ContextSection,
    pub layer: BudgetLayer,
    pub stability: SectionStability,
    pub priority: u8,
    pub token_estimate: usize,
    pub eviction: EvictionPolicy,
}

/// Soft and hard constraints for one budget layer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LayerBudget {
    pub soft_limit: usize,
    pub hard_limit: usize,
}

/// Layer-aware plan derived from the backward-compatible budget config.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextBudgetPlan {
    pub total_max: usize,
    pub layers: BTreeMap<BudgetLayer, LayerBudget>,
}

impl ContextBudgetPlan {
    pub fn from_config(config: &BudgetConfig) -> Self {
        let total = config.max_tokens;
        let layer = |soft_ratio: f64, hard_ratio: f64| LayerBudget {
            soft_limit: ratio(total, soft_ratio),
            hard_limit: ratio(total, hard_ratio),
        };
        let mut layers = BTreeMap::new();
        layers.insert(
            BudgetLayer::FrozenCoreAndRules,
            LayerBudget {
                soft_limit: ratio(total, config.system_budget_ratio),
                hard_limit: ratio(total, config.system_budget_ratio),
            },
        );
        layers.insert(BudgetLayer::ToolSchemasCore, layer(0.12, 0.15));
        layers.insert(BudgetLayer::ToolSchemasDeferred, layer(0.03, 0.05));
        layers.insert(BudgetLayer::L1Index, layer(0.04, 0.06));
        layers.insert(BudgetLayer::WorkingMemory, layer(0.03, 0.04));
        layers.insert(BudgetLayer::PrefetchRecall, layer(0.03, 0.04));
        layers.insert(BudgetLayer::CompactionSummaries, layer(0.08, 0.10));
        layers.insert(BudgetLayer::History, layer(0.55, 0.80));
        layers.insert(BudgetLayer::ToolResultInline, layer(0.08, 0.12));
        layers.insert(
            BudgetLayer::CurrentTurn,
            LayerBudget {
                soft_limit: total,
                hard_limit: total,
            },
        );
        Self {
            total_max: total,
            layers,
        }
    }

    pub fn layer(&self, layer: BudgetLayer) -> LayerBudget {
        self.layers.get(&layer).copied().unwrap_or(LayerBudget {
            soft_limit: self.total_max,
            hard_limit: self.total_max,
        })
    }
}

/// One report entry for a dropped or compacted fragment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssemblyDecision {
    pub id: String,
    pub layer: BudgetLayer,
    pub reason: AssemblyDecisionReason,
}

/// Mandatory context assembly measurement emitted without prompt content.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ContextAssemblyReport {
    pub fragments_in: usize,
    pub selected: usize,
    pub dropped: Vec<AssemblyDecision>,
    pub compacted: Vec<AssemblyDecision>,
    pub totals_by_layer: BTreeMap<BudgetLayer, usize>,
    pub total_estimated: usize,
    pub total_max: usize,
}

impl ContextAssemblyReport {
    pub fn record_selected(&mut self, fragment: &ContextFragment) {
        self.selected += 1;
        self.total_estimated = self.total_estimated.saturating_add(fragment.token_estimate);
        *self.totals_by_layer.entry(fragment.layer).or_default() += fragment.token_estimate;
    }

    pub fn has_pressure_action(&self) -> bool {
        !self.dropped.is_empty() || !self.compacted.is_empty()
    }
}

/// Select dynamic sections under their layer and total limits.
///
/// Required working-memory and plan constraints are retained. Recall is the
/// first optional turn fragment removed under pressure.
pub fn select_dynamic_sections(
    sections: &[PromptSection],
    base_tokens: usize,
    plan: &ContextBudgetPlan,
) -> (Vec<PromptSection>, ContextAssemblyReport) {
    let mut report = ContextAssemblyReport {
        fragments_in: sections.len().saturating_add(1),
        total_estimated: base_tokens,
        total_max: plan.total_max,
        ..ContextAssemblyReport::default()
    };
    if base_tokens > 0 {
        report.selected = 1;
        report
            .totals_by_layer
            .insert(BudgetLayer::History, base_tokens);
    }
    let mut selected = Vec::with_capacity(sections.len());
    for (index, section) in sections.iter().enumerate() {
        let fragment = fragment_for_section(index, section);
        let used = report
            .totals_by_layer
            .get(&fragment.layer)
            .copied()
            .unwrap_or_default();
        let layer_limit = plan.layer(fragment.layer).soft_limit;
        let exceeds_layer = used.saturating_add(fragment.token_estimate) > layer_limit;
        let exceeds_total = report
            .total_estimated
            .saturating_add(fragment.token_estimate)
            > plan.total_max;
        if fragment.eviction == EvictionPolicy::Drop && (exceeds_layer || exceeds_total) {
            report.dropped.push(AssemblyDecision {
                id: fragment.id,
                layer: fragment.layer,
                reason: if exceeds_total {
                    AssemblyDecisionReason::TotalHardLimit
                } else {
                    AssemblyDecisionReason::LayerSoftLimit
                },
            });
            continue;
        }
        report.record_selected(&fragment);
        selected.push(section.clone());
    }
    (selected, report)
}

/// Add provider-ready messages and tool schemas to a content-free report.
pub fn measure_provider_context(
    messages: &[Message],
    tools: &ToolDefinitionSet,
    plan: &ContextBudgetPlan,
) -> ContextAssemblyReport {
    let mut report = ContextAssemblyReport {
        fragments_in: messages.len().saturating_add(tools.definitions.len()),
        total_max: plan.total_max,
        ..ContextAssemblyReport::default()
    };
    for (index, message) in messages.iter().enumerate() {
        let layer = if index == 0 && message.role == "system" {
            BudgetLayer::FrozenCoreAndRules
        } else if index + 1 == messages.len() {
            BudgetLayer::CurrentTurn
        } else if message.role == "tool" {
            BudgetLayer::ToolResultInline
        } else {
            BudgetLayer::History
        };
        let fragment = ContextFragment {
            id: format!("message:{index}"),
            section: if index + 1 == messages.len() {
                ContextSection::CurrentUser
            } else {
                ContextSection::History
            },
            layer,
            stability: if index == 0 {
                SectionStability::SessionStable
            } else {
                SectionStability::TurnVolatile
            },
            priority: if index == 0 || index + 1 == messages.len() {
                u8::MAX
            } else {
                100
            },
            token_estimate: estimate_message(message),
            eviction: if index == 0 || index + 1 == messages.len() {
                EvictionPolicy::Never
            } else {
                EvictionPolicy::Compact
            },
        };
        report.record_selected(&fragment);
    }
    for (index, definition) in tools.definitions.iter().enumerate() {
        let layer = if index < tools.core_count {
            BudgetLayer::ToolSchemasCore
        } else {
            BudgetLayer::ToolSchemasDeferred
        };
        let fragment = ContextFragment {
            id: format!("tool_schema:{index}"),
            section: ContextSection::History,
            layer,
            stability: SectionStability::SessionStable,
            priority: if index < tools.core_count { 220 } else { 80 },
            token_estimate: estimate_tokens(&definition.to_string()),
            eviction: if index < tools.core_count {
                EvictionPolicy::Never
            } else {
                EvictionPolicy::Drop
            },
        };
        report.record_selected(&fragment);
    }
    report
}

pub fn estimate_messages(messages: &[Message]) -> usize {
    messages.iter().map(estimate_message).sum()
}

fn fragment_for_section(index: usize, section: &PromptSection) -> ContextFragment {
    let (layer, priority, eviction) = match section.section {
        ContextSection::WorkingMemory => (BudgetLayer::WorkingMemory, 240, EvictionPolicy::Never),
        ContextSection::PrefetchRecall => (BudgetLayer::PrefetchRecall, 40, EvictionPolicy::Drop),
        ContextSection::Compaction => (
            BudgetLayer::CompactionSummaries,
            180,
            EvictionPolicy::Compact,
        ),
        ContextSection::MemoryPolicyAndIndex => {
            (BudgetLayer::L1Index, 170, EvictionPolicy::Compact)
        }
        ContextSection::PlanGuard | ContextSection::CurrentUser => {
            (BudgetLayer::CurrentTurn, u8::MAX, EvictionPolicy::Never)
        }
        ContextSection::MaskAndIdentity
        | ContextSection::FrozenCore
        | ContextSection::AgentRulesAndSkills => (
            BudgetLayer::FrozenCoreAndRules,
            u8::MAX,
            EvictionPolicy::Never,
        ),
        ContextSection::History | ContextSection::VolatileMeta => {
            (BudgetLayer::History, 100, EvictionPolicy::Compact)
        }
    };
    ContextFragment {
        id: format!("{}:{index}", section.section.wire_name()),
        section: section.section,
        layer,
        stability: section.stability,
        priority,
        token_estimate: estimate_tokens(&section.body),
        eviction,
    }
}

fn estimate_message(message: &Message) -> usize {
    let mut total = estimate_tokens(&message.content.to_text_lossy());
    if let Some(reasoning) = &message.reasoning_content {
        total = total.saturating_add(estimate_tokens(reasoning));
    }
    if let Some(tool_calls) = &message.tool_calls {
        total = total.saturating_add(estimate_tokens(
            &serde_json::to_string(tool_calls).unwrap_or_default(),
        ));
    }
    total
}

fn ratio(total: usize, value: f64) -> usize {
    (total as f64 * value) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_ratio_is_a_limit_not_consumed_tokens() {
        let plan = ContextBudgetPlan::from_config(&BudgetConfig {
            max_tokens: 1_000,
            system_budget_ratio: 0.15,
            ..BudgetConfig::default()
        });
        assert_eq!(plan.layer(BudgetLayer::FrozenCoreAndRules).hard_limit, 150);
        let report = measure_provider_context(&[], &empty_tools(), &plan);
        assert_eq!(report.total_estimated, 0);
    }

    #[test]
    fn recall_is_dropped_before_required_working_memory() {
        let plan = ContextBudgetPlan::from_config(&BudgetConfig {
            max_tokens: 100,
            ..BudgetConfig::default()
        });
        let sections = vec![
            PromptSection::new(ContextSection::WorkingMemory, "constraint"),
            PromptSection::new(ContextSection::PrefetchRecall, "x".repeat(30)),
        ];
        let (selected, report) = select_dynamic_sections(&sections, 0, &plan);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].section, ContextSection::WorkingMemory);
        assert_eq!(report.dropped[0].layer, BudgetLayer::PrefetchRecall);
    }

    #[test]
    fn current_user_and_stable_prefix_are_measured_not_dropped() {
        let plan = ContextBudgetPlan::from_config(&BudgetConfig {
            max_tokens: 1,
            ..BudgetConfig::default()
        });
        let messages = vec![Message::system("stable"), Message::user("current")];
        let report = measure_provider_context(&messages, &empty_tools(), &plan);
        assert_eq!(report.selected, 2);
        assert!(report.dropped.is_empty());
        assert!(report.total_estimated > plan.total_max);
    }

    #[test]
    fn core_and_deferred_tool_schemas_use_distinct_layers() {
        let tools = ToolDefinitionSet {
            definitions: vec![
                serde_json::json!({"name":"core"}),
                serde_json::json!({"name":"mcp"}),
            ],
            core_count: 1,
        };
        let report = measure_provider_context(
            &[],
            &tools,
            &ContextBudgetPlan::from_config(&BudgetConfig::default()),
        );
        assert!(report
            .totals_by_layer
            .contains_key(&BudgetLayer::ToolSchemasCore));
        assert!(report
            .totals_by_layer
            .contains_key(&BudgetLayer::ToolSchemasDeferred));
    }

    #[test]
    fn report_layer_totals_equal_the_total_estimate() {
        let plan = ContextBudgetPlan::from_config(&BudgetConfig::default());
        let sections = vec![PromptSection::new(
            ContextSection::WorkingMemory,
            "active constraint",
        )];
        let (_, report) = select_dynamic_sections(&sections, 17, &plan);
        assert_eq!(
            report.totals_by_layer.values().sum::<usize>(),
            report.total_estimated
        );
    }

    fn empty_tools() -> ToolDefinitionSet {
        ToolDefinitionSet {
            definitions: Vec::new(),
            core_count: 0,
        }
    }
}
