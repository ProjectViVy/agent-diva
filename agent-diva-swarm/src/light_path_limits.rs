//! Light 路径 **墙钟超时** 与 **内部步数上限**（FR19 / ADR-E）。
//!
//! 默认值在此 **单点** 定义；gateway 或 agent 循环在 [`crate::ExecutionTier::Light`] 下应遵守并在触顶时返回
//! [`LightPathStopReason`]（可机读、可对用户说明）。具体 enforcement 可在后续 story 接入运行时。

use serde::{Deserialize, Serialize};

/// 轻量路径默认墙钟上限（毫秒）。维护者调参仅改此处并更新
/// [`docs/adr-e-fr19-execution-tier.md`](../../../docs/adr-e-fr19-execution-tier.md)。
pub const LIGHT_PATH_MAX_WALL_MS: u64 = 120_000;

/// 轻量路径默认内部步数上限（例如 LLM+工具轮次计数，语义由接入方定义）。
pub const LIGHT_PATH_MAX_INTERNAL_STEPS: u32 = 64;

/// 轻量路径结束原因（成功或失败类触顶）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LightPathStopReason {
    /// 正常完成（无触顶）。
    Completed,
    /// 超过墙钟预算。
    WallClockTimeout { elapsed_ms: u64 },
    /// 超过内部步数上限。
    InternalStepLimit { steps_used: u32 },
}

impl LightPathStopReason {
    /// 供日志或对用户简述的固定英文码（稳定契约）；UI 可再映射文案。
    #[must_use]
    pub fn machine_code(&self) -> &'static str {
        match self {
            Self::Completed => "light_path.completed",
            Self::WallClockTimeout { .. } => "light_path.wall_clock_timeout",
            Self::InternalStepLimit { .. } => "light_path.internal_step_limit",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_round_trip_timeout() {
        let r = LightPathStopReason::WallClockTimeout { elapsed_ms: 99 };
        let j = serde_json::to_string(&r).expect("ser");
        let back: LightPathStopReason = serde_json::from_str(&j).expect("de");
        assert_eq!(r, back);
        assert_eq!(back.machine_code(), "light_path.wall_clock_timeout");
    }
}
