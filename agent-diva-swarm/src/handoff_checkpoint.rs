//! 序曲链 Handoff 检查点 v0（Story 5.3）：turn 内最后 **成功** 一步的可序列化快照。
//!
//! **存放位置：** 与 Epic 1 皮层/过程事件一致 — **进程内**，不写入会话 transcript；
//! 失败时经 **结构化日志**（`tracing`）输出 JSON，供运维与开发者查询。未进入 `ProcessEventV0` 白名单 v0。
//!
//! 字段与 MVP 语义见仓库内 `agent-diva-swarm/docs/handoff-checkpoint-v0.md`。

use crate::sanitize_tool_summary_for_process_event;
use serde::{Deserialize, Serialize};

/// 与过程事件 v0 对齐的 schema 版本字段（独立 DTO，当前为 0）。
pub const HANDOFF_CHECKPOINT_SCHEMA_VERSION_V0: u32 = 0;

/// `summary_preview` 最大 Unicode 标量数（NFR-S2 限长）。
pub const HANDOFF_CHECKPOINT_PREVIEW_MAX_CHARS: usize = 256;

/// 参与指纹计算的消毒正文上限（防极端长输出占用 CPU）。
const HANDOFF_CHECKPOINT_DIGEST_SOURCE_MAX_CHARS: usize = 4096;

/// FNV-1a 64-bit，输入为 **已消毒** UTF-8 字节；输出十六进制小写字符串（16 hex chars）。
fn fnv1a64_hex(sanitized_utf8: &[u8]) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in sanitized_utf8 {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

/// 序曲 handoff 检查点（可 `serde_json` 序列化；**不**默认进入用户 transcript）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmHandoffCheckpointV0 {
    pub schema_version: u32,
    /// 最后一个成功角色的 `phase_id`（与 `swarm-prelude` 配置一致）。
    pub role_id: String,
    /// 序曲链内 0-based 步号：第 1 个成功角色为 `0`。
    pub prelude_round_index: u32,
    /// 消毒、截断后的输出预览（非完整模型原文）。
    pub summary_preview: String,
    /// 基于消毒全文（上限见常量）的确定性指纹，供对账或「仅哈希」扩展。
    pub content_fingerprint_hex: String,
}

impl SwarmHandoffCheckpointV0 {
    /// 由某一成功 `chat` 的模型输出构造检查点（限长 + 消毒 + 指纹）。
    #[must_use]
    pub fn from_successful_role_output(
        prelude_round_index: u32,
        role_id: impl Into<String>,
        raw_llm_output: &str,
    ) -> Self {
        let summary_preview = sanitize_tool_summary_for_process_event(
            raw_llm_output,
            HANDOFF_CHECKPOINT_PREVIEW_MAX_CHARS,
        );
        let digest_src = sanitize_tool_summary_for_process_event(
            raw_llm_output,
            HANDOFF_CHECKPOINT_DIGEST_SOURCE_MAX_CHARS,
        );
        let content_fingerprint_hex = fnv1a64_hex(digest_src.as_bytes());
        Self {
            schema_version: HANDOFF_CHECKPOINT_SCHEMA_VERSION_V0,
            role_id: role_id.into(),
            prelude_round_index,
            summary_preview,
            content_fingerprint_hex,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_preview_truncates_and_digest_stable() {
        let long = "a".repeat(500);
        let c = SwarmHandoffCheckpointV0::from_successful_role_output(1, "role_b", &long);
        assert_eq!(c.schema_version, HANDOFF_CHECKPOINT_SCHEMA_VERSION_V0);
        assert_eq!(c.role_id, "role_b");
        assert_eq!(c.prelude_round_index, 1);
        assert!(c.summary_preview.len() < long.len());
        assert!(c.summary_preview.ends_with("..."));
        assert_eq!(c.content_fingerprint_hex.len(), 16);

        let c2 = SwarmHandoffCheckpointV0::from_successful_role_output(1, "role_b", &long);
        assert_eq!(c.content_fingerprint_hex, c2.content_fingerprint_hex);
    }

    #[test]
    fn checkpoint_flattens_control_chars_in_preview() {
        let c = SwarmHandoffCheckpointV0::from_successful_role_output(0, "p", "x\ny\tz");
        assert!(!c.summary_preview.contains('\n'));
        assert!(c.summary_preview.contains("x"));
        assert!(c.summary_preview.contains("z"));
    }
}
