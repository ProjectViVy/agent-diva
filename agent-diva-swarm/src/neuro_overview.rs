//! 神经系统总览快照（Story 3.2）：与 [`CortexState`]、[`ProcessEventV0`] 对齐的 **单一 DTO**，
//! 供 Tauri / gateway 同源下发；**禁止** GUI 另建第二套皮层状态机。
//!
//! # `data_phase`（可测、须对用户诚实）
//!
//! - **`degraded`** — `cortex.enabled == false`（简化路径；过程事件不发射，见 `PROCESS_EVENTS_CORTEX_OFF.md`）。
//! - **`stub`** — 皮层开，但 **尚无** 可展示的过程事件缓冲（或缓冲为空）；**不得** 用假列表冒充实时连接。
//! - **`live`** — 皮层开且存在至少一条过程事件映射行（与 Story 1.5 白名单 DTO 同源）。

use crate::{
    CortexState, ProcessEventNameV0, ProcessEventV0, SwarmRunStopReason,
};

use serde::{Deserialize, Serialize};

/// 线协议 schema 版本（v0 = 0）；演进时 bump 并与 GUI 对齐。
pub const NEURO_OVERVIEW_SCHEMA_VERSION_V0: u32 = 0;

/// 数据阶段：驱动 `DataPhaseBadge` / 等价诚实文案（FR6、UX-IMPL-4）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NeuroDataPhase {
    Live,
    Stub,
    Degraded,
}

/// 单侧活动行（列表渲染；无则占位 + 角标，禁止骨架冒充真数据）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeuroActivityRowV0 {
    pub id: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// 供 UI 状态点：`idle` | `active` | `done` | `error`
    pub status: String,
}

/// 神经总览快照 v0：`cortex` 与 `get_cortex_state` 同源；活动行由 `ProcessEventV0` 推导。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeuroOverviewSnapshotV0 {
    pub schema_version: u32,
    pub data_phase: NeuroDataPhase,
    pub cortex: CortexState,
    pub left_rows: Vec<NeuroActivityRowV0>,
    pub right_rows: Vec<NeuroActivityRowV0>,
}

/// 由权威 [`CortexState`] 与 **同源** 过程事件切片构造快照（Tauri / 测试共用）。
#[must_use]
pub fn build_neuro_overview_snapshot_v0(cortex: CortexState, events: &[ProcessEventV0]) -> NeuroOverviewSnapshotV0 {
    if !cortex.enabled {
        return NeuroOverviewSnapshotV0 {
            schema_version: NEURO_OVERVIEW_SCHEMA_VERSION_V0,
            data_phase: NeuroDataPhase::Degraded,
            cortex,
            left_rows: Vec::new(),
            right_rows: Vec::new(),
        };
    }

    if events.is_empty() {
        return NeuroOverviewSnapshotV0 {
            schema_version: NEURO_OVERVIEW_SCHEMA_VERSION_V0,
            data_phase: NeuroDataPhase::Stub,
            cortex,
            left_rows: Vec::new(),
            right_rows: Vec::new(),
        };
    }

    let mut left_rows = Vec::new();
    let mut right_rows = Vec::new();

    for (i, ev) in events.iter().enumerate() {
        let id = row_id(ev, i);
        match ev.name {
            ProcessEventNameV0::SwarmPhaseChanged => {
                left_rows.push(NeuroActivityRowV0 {
                    id,
                    label: ev.message.clone(),
                    detail: ev.phase_id.clone(),
                    status: "active".to_string(),
                });
            }
            ProcessEventNameV0::ToolCallStarted => {
                right_rows.push(NeuroActivityRowV0 {
                    id,
                    label: ev.message.clone(),
                    detail: ev.tool_name.clone(),
                    status: "active".to_string(),
                });
            }
            // v0 `ProcessEventV0` has no structured ok/err for tool finish; UI maps to a dot via `status`.
            // Convention: emitters SHOULD include a substring "error" (ASCII, case-insensitive) in `message`
            // on failure. Prefer a dedicated wire field in a future schema bump if false positives matter.
            ProcessEventNameV0::ToolCallFinished => {
                let status = if ev.message.to_lowercase().contains("error") {
                    "error"
                } else {
                    "done"
                };
                right_rows.push(NeuroActivityRowV0 {
                    id,
                    label: ev.message.clone(),
                    detail: ev.tool_name.clone(),
                    status: status.to_string(),
                });
            }
            ProcessEventNameV0::SwarmRunFinished | ProcessEventNameV0::SwarmRunCapped => {
                let status = if matches!(ev.stop_reason, Some(SwarmRunStopReason::Error)) {
                    "error"
                } else {
                    "done"
                };
                left_rows.push(NeuroActivityRowV0 {
                    id,
                    label: ev.message.clone(),
                    detail: ev
                        .stop_reason
                        .map(|r| r.as_stop_reason_wire_str().to_string()),
                    status: status.to_string(),
                });
            }
        }
    }

    NeuroOverviewSnapshotV0 {
        schema_version: NEURO_OVERVIEW_SCHEMA_VERSION_V0,
        data_phase: NeuroDataPhase::Live,
        cortex,
        left_rows,
        right_rows,
    }
}

fn row_id(ev: &ProcessEventV0, idx: usize) -> String {
    if let Some(c) = &ev.correlation_id {
        return format!("{}-{}", idx, c);
    }
    if let Some(p) = &ev.phase_id {
        return format!("{}-{}", idx, p);
    }
    format!("evt-{idx}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CortexState, ProcessEventV0};

    #[test]
    fn cortex_off_yields_degraded_empty_rows() {
        let cortex = CortexState::with_enabled(false);
        let snap = build_neuro_overview_snapshot_v0(
            cortex.clone(),
            &[ProcessEventV0::tool_call_started("t", "running", "c1")],
        );
        assert_eq!(snap.data_phase, NeuroDataPhase::Degraded);
        assert!(snap.left_rows.is_empty());
        assert!(snap.right_rows.is_empty());
        assert_eq!(snap.cortex, cortex);
    }

    #[test]
    fn cortex_on_no_events_yields_stub() {
        let cortex = CortexState::with_enabled(true);
        let snap = build_neuro_overview_snapshot_v0(cortex.clone(), &[]);
        assert_eq!(snap.data_phase, NeuroDataPhase::Stub);
        assert!(snap.left_rows.is_empty());
        assert_eq!(snap.schema_version, NEURO_OVERVIEW_SCHEMA_VERSION_V0);
    }

    #[test]
    fn cortex_on_with_events_yields_live_partitioned_rows() {
        let cortex = CortexState::with_enabled(true);
        let events = vec![
            ProcessEventV0::swarm_phase_changed("p1", "phase a"),
            ProcessEventV0::tool_call_started("web_search", "searching", "t1"),
        ];
        let snap = build_neuro_overview_snapshot_v0(cortex.clone(), &events);
        assert_eq!(snap.data_phase, NeuroDataPhase::Live);
        assert_eq!(snap.left_rows.len(), 1);
        assert_eq!(snap.right_rows.len(), 1);
        assert_eq!(snap.left_rows[0].label, "phase a");
        assert_eq!(snap.right_rows[0].detail.as_deref(), Some("web_search"));
    }

    #[test]
    fn tool_finished_error_hint_sets_status_error() {
        let cortex = CortexState::with_enabled(true);
        let events = vec![ProcessEventV0::tool_call_finished(
            "x",
            "Tool error: boom",
            "c2",
        )];
        let snap = build_neuro_overview_snapshot_v0(cortex, &events);
        assert_eq!(snap.right_rows[0].status, "error");
    }

    #[test]
    fn swarm_terminal_row_detail_uses_stop_reason_camel_case() {
        let cortex = CortexState::with_enabled(true);
        let events = vec![ProcessEventV0::swarm_run_capped("capped msg")];
        let snap = build_neuro_overview_snapshot_v0(cortex, &events);
        assert_eq!(snap.left_rows.len(), 1);
        assert_eq!(
            snap.left_rows[0].detail.as_deref(),
            Some("budgetExceeded"),
            "detail must match process-event JSON stopReason, not Debug"
        );
    }
}
