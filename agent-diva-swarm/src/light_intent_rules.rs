//! FR19 轻量意图判定 — **唯一真相源**（Story 1.7 / ADR-E）。
//!
//! # 维护约定
//!
//! - **所有**「轻量类」启发式须在本文件迭代；**禁止**在其它模块复制阈值或模式，以免与测试、文档漂移。
//! - 变更时同步 [`crate::light_path_limits`] 文档注释、[`docs/adr-e-fr19-execution-tier.md`](../../../docs/adr-e-fr19-execution-tier.md) 与 `execution_tier` 无 GUI 测试。
//!
//! # 规则清单（v0，可迭代）
//!
//! 1. **显式 skill / 命令式调用**：trim 后以 **`/`** 开头（slash command），或 trim 后以 **`bmad-` / `BMAD-`** 开头（BMAD 等工作流前缀）。
//! 2. **短问答**：Unicode 标量个数（[`str::chars`]，非字节长度）≤ [`SHORT_QA_MAX_SCALARS`]。
//!
//! 不满足以上任一者 **不** 自动视为轻量；是否进入 [`crate::ExecutionTier::FullSwarm`] 由 [`crate::resolve_execution_tier`] 结合皮层开关与显式深度选择决定。

/// 短问答阈值：标量个数上限（与 PRD「短问答」一致，可在本文件单点调参）。
pub const SHORT_QA_MAX_SCALARS: usize = 256;

#[must_use]
pub fn is_explicit_skill_style_input(text: &str) -> bool {
    let t = text.trim_start();
    let lower = t.to_ascii_lowercase();
    lower.starts_with('/') || lower.starts_with("bmad-")
}

#[must_use]
pub fn is_short_qa(text: &str) -> bool {
    text.trim().chars().count() <= SHORT_QA_MAX_SCALARS
}

/// 用户输入是否落在 **轻量类**（FR19）：显式 skill 风格 **或** 短问答。
#[must_use]
pub fn is_light_intent(user_text: &str) -> bool {
    is_explicit_skill_style_input(user_text) || is_short_qa(user_text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slash_prefix_is_light() {
        assert!(is_light_intent("  /skill run"));
    }

    #[test]
    fn bmad_prefix_is_light() {
        assert!(is_light_intent("bmad-dev-story x"));
    }

    #[test]
    fn short_chinese_question_is_light() {
        assert!(is_light_intent("什么是 session？"));
    }

    #[test]
    fn over_limit_is_not_short_qa_alone() {
        let s = "x".repeat(SHORT_QA_MAX_SCALARS + 1);
        assert!(!is_short_qa(&s));
        assert!(!is_light_intent(&s));
    }
}
