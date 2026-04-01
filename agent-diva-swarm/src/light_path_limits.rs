//! Light 路径 **墙钟超时** 与 **内部步数上限**（FR19 / ADR-E）。
//!
//! 默认值在此 **单点** 定义；[`ExecutionTier::Light`] 下由 `agent-diva-agent` 的 **AgentLoop** 主循环在每步与流式
//! 轮询处对照本模块常量 enforcement，触顶时通过 [`format_light_path_stop_for_user`] 返回可对用户说明的文案（含稳定
//! [`LightPathStopReason::machine_code`]）。

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

/// 轻量路径 **触顶/超时** 时对用户可见的英文说明（含稳定 `code:` 行，供 UI/契约解析）。
#[must_use]
pub fn format_light_path_stop_for_user(reason: LightPathStopReason) -> String {
    let code = reason.machine_code();
    match reason {
        LightPathStopReason::Completed => {
            debug_assert!(false, "format_light_path_stop_for_user(Completed) is unexpected");
            "Light path completed.".to_string()
        }
        LightPathStopReason::WallClockTimeout { elapsed_ms } => format!(
            "This light-path turn stopped: wall-clock budget exceeded after {} ms (code: {}).",
            elapsed_ms, code
        ),
        LightPathStopReason::InternalStepLimit { steps_used } => format!(
            "This light-path turn stopped: internal step limit reached ({} steps, code: {}).",
            steps_used, code
        ),
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

    #[test]
    fn user_message_includes_machine_codes() {
        let wall = format_light_path_stop_for_user(LightPathStopReason::WallClockTimeout {
            elapsed_ms: 120_000,
        });
        assert!(wall.contains("light_path.wall_clock_timeout"));

        let steps = format_light_path_stop_for_user(LightPathStopReason::InternalStepLimit {
            steps_used: LIGHT_PATH_MAX_INTERNAL_STEPS,
        });
        assert!(steps.contains("light_path.internal_step_limit"));
        assert!(steps.contains(&LIGHT_PATH_MAX_INTERNAL_STEPS.to_string()));
    }
}
