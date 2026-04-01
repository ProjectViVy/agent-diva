//! 大脑皮层（Cortex）运行时状态：进程内单一真相源（Epic 1 Story 1.2）。
//!
//! # 持久化边界
//!
//! 当前实现仅在 **进程内内存**（[`CortexRuntime`] 内 `std::sync::RwLock`）。**不**引入本特性专用数据库。
//! 若后续需要跨重启恢复，应沿用 `architecture.md` 数据架构（既有配置/存储实践），并在该处单点说明。
//!
//! # FR14（单一真相源）
//!
//! [`CortexState`] / [`CortexRuntime`] 为 Rust 侧权威副本落点；**Vue 前端不得另立长期持久化的第二真相源**。
//! GUI 仅通过后续 gateway / Tauri 契约（Story 1.3+）读取与触发。
//!
//! # 默认值
//!
//! [`CORTEX_DEFAULT_ENABLED`] 冻结为 **`true`**：与 PRD 主路径叙事一致（蜂群层默认可用，用户可显式关闭进入简化路径，FR3）。
//! 若产品变更默认，须同步修改该常量、本段说明与单元测试。
//!
//! # 锁中毒
//!
//! `RwLock` 在持有者 panic 后可能 **中毒**；当前实现通过 `into_inner()` 恢复读写并先打 **`tracing::error!`**（target：`agent_diva_swarm::cortex`），便于运维过滤。状态在极端情况下可能与预期不一致，应以日志为信号排查根因 panic。

use serde::{Deserialize, Serialize};
use std::sync::RwLock;

/// Serde / 线协议模式版本（`architecture.md` Pattern Examples；与 NFR-I1 演进路径一致，当前固定 v0）。
pub const CORTEX_STATE_SCHEMA_VERSION_V0: u32 = 0;

/// [`CortexState::enabled`] 的默认值（单点冻结，供实现与测试引用）。
pub const CORTEX_DEFAULT_ENABLED: bool = true;

/// 与架构示例对齐的可序列化快照；IPC / JSON 与 GUI 共用。**线格式** 字段名为 **camelCase**（`enabled`, `schemaVersion`），全链路统一。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CortexState {
    pub enabled: bool,
    pub schema_version: u32,
}

/// 对外稳定契约的别名（Story 1.3）：与 [`CortexState`] 同一类型。
pub type CortexSyncDto = CortexState;

impl Default for CortexState {
    fn default() -> Self {
        Self {
            enabled: CORTEX_DEFAULT_ENABLED,
            schema_version: CORTEX_STATE_SCHEMA_VERSION_V0,
        }
    }
}

impl CortexState {
    /// 构造指定开关状态，模式版本固定为 [`CORTEX_STATE_SCHEMA_VERSION_V0`]。
    #[must_use]
    pub fn with_enabled(enabled: bool) -> Self {
        Self {
            enabled,
            schema_version: CORTEX_STATE_SCHEMA_VERSION_V0,
        }
    }
}

/// 会话 / 进程边界内的权威持有者：同实例上查询与写入一致。
#[derive(Debug)]
pub struct CortexRuntime {
    inner: RwLock<CortexState>,
}

impl Default for CortexRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl CortexRuntime {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(CortexState::default()),
        }
    }

    #[must_use]
    pub fn from_state(initial: CortexState) -> Self {
        Self {
            inner: RwLock::new(initial),
        }
    }

    /// 查询当前权威状态（与同实例内最近一次写入一致）。
    #[must_use]
    pub fn snapshot(&self) -> CortexState {
        self.read_state()
    }

    /// 设置开/关（内部与测试可见；对外 Tauri 在 Story 1.3 接线）。
    pub fn set_enabled(&self, enabled: bool) {
        let mut guard = self.inner.write().unwrap_or_else(|e| {
            tracing::error!(
                target: "agent_diva_swarm::cortex",
                "CortexRuntime write lock poisoned; recovering with into_inner — state may be inconsistent after a panicking holder"
            );
            e.into_inner()
        });
        guard.enabled = enabled;
    }

    /// 切换开/关并返回切换后的快照（**单元测试 / 内部**；桌面壳须先经 gateway 同步钩再调用 [`set_enabled`](Self::set_enabled)，见 `agent-diva-gui` `cortex_sync` / Story 2.2）。
    pub fn toggle(&self) -> CortexState {
        let mut guard = self.inner.write().unwrap_or_else(|e| {
            tracing::error!(
                target: "agent_diva_swarm::cortex",
                "CortexRuntime write lock poisoned; recovering with into_inner — state may be inconsistent after a panicking holder"
            );
            e.into_inner()
        });
        guard.enabled = !guard.enabled;
        guard.clone()
    }

    fn read_state(&self) -> CortexState {
        self.inner
            .read()
            .unwrap_or_else(|e| {
                tracing::error!(
                    target: "agent_diva_swarm::cortex",
                    "CortexRuntime read lock poisoned; recovering with into_inner — snapshot may be inconsistent after a panicking holder"
                );
                e.into_inner()
            })
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_toggle_query_matches_session() {
        let rt = CortexRuntime::new();
        assert_eq!(rt.snapshot(), CortexState::default());
        assert!(rt.snapshot().enabled);
        assert_eq!(rt.snapshot().schema_version, CORTEX_STATE_SCHEMA_VERSION_V0);

        rt.set_enabled(false);
        assert!(!rt.snapshot().enabled);

        let after_toggle = rt.toggle();
        assert!(after_toggle.enabled);
        assert_eq!(rt.snapshot(), after_toggle);
    }

    #[test]
    fn serde_json_round_trip() {
        let s = CortexState::default();
        let json = serde_json::to_string(&s).expect("serialize");
        let back: CortexState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(s, back);
    }
}
