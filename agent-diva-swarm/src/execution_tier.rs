//! ADR-E：`ExecutionTier`（Light vs FullSwarm）路由入口（Story 1.7 / FR19）。
//!
//! **判定轻量类** 仅允许调用 [`crate::light_intent_rules`]，不得重复实现阈值。
//!
//! # FR19 核心语义
//!
//! - 无 **显式**「深度 / 全蜂群」选择时，**不得** 仅因 **大脑皮层 ON** 将 **轻量类** 输入升级为 [`ExecutionTier::FullSwarm`]。

use crate::light_intent_rules::is_light_intent;

/// 执行分层：轻量短路径 vs 完整多参与者蜂群编排。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionTier {
    /// 可完成短路径（显式 skill、短问答等）。
    Light,
    /// 多参与者 handoff / 对弈图级编排。
    FullSwarm,
}

/// 每次用户 turn 在编排入口调用：根据意图分类、皮层开关与显式深度选择决定分层。
///
/// `explicit_full_swarm` 来自 **用户或设置显式选择**（非本函数从正文猜测），与 epics 表述一致。
#[must_use]
pub fn resolve_execution_tier(
    user_text: &str,
    cortex_enabled: bool,
    explicit_full_swarm: bool,
) -> ExecutionTier {
    if explicit_full_swarm {
        return ExecutionTier::FullSwarm;
    }
    if is_light_intent(user_text) {
        return ExecutionTier::Light;
    }
    if cortex_enabled {
        ExecutionTier::FullSwarm
    } else {
        ExecutionTier::Light
    }
}

/// 是否将构造 / 进入 FullSwarm 拓扑（无 GUI 测试与 gateway 观测点可用此布尔）。
#[must_use]
pub fn would_enter_full_swarm_topology(
    user_text: &str,
    cortex_enabled: bool,
    explicit_full_swarm: bool,
) -> bool {
    matches!(
        resolve_execution_tier(user_text, cortex_enabled, explicit_full_swarm),
        ExecutionTier::FullSwarm
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PRD **旅程五** / AC#3：皮层 ON + 轻量输入 → 仍 **不** 启全图。
    #[test]
    fn journey_five_cortex_on_light_qa_no_full_swarm() {
        let text = "什么是 session？";
        assert!(
            !would_enter_full_swarm_topology(text, true, false),
            "轻量短问答 + 皮层 ON 不得进入 FullSwarm"
        );
        assert_eq!(
            resolve_execution_tier(text, true, false),
            ExecutionTier::Light
        );
    }

    #[test]
    fn journey_five_cortex_on_slash_skill_no_full_swarm() {
        assert!(!would_enter_full_swarm_topology("/help", true, false));
    }

    #[test]
    fn explicit_full_swarm_forces_topology_even_if_short() {
        assert!(would_enter_full_swarm_topology("hi", true, true));
    }

    #[test]
    fn long_non_light_cortex_on_enters_full_swarm() {
        let long = "x".repeat(crate::light_intent_rules::SHORT_QA_MAX_SCALARS + 1);
        assert!(would_enter_full_swarm_topology(&long, true, false));
    }

    #[test]
    fn long_text_cortex_off_stays_light() {
        let long = "x".repeat(400);
        assert!(!would_enter_full_swarm_topology(&long, false, false));
    }
}
