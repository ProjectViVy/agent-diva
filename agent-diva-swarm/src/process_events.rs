//! 最小过程事件（FR2 发射侧，Story 1.5）：白名单 DTO、皮层门控与节流/批处理出口。
//!
//! # 默认节流策略（NFR-P2）
//!
//! - **`swarm_phase_changed`**：进入 **批处理缓冲**；在 **100ms** 合并窗口内或达到 **`max_batch`（默认 32）** 条时 **整批** 下发到下游。
//! - **`tool_call_started` / `tool_call_finished`**：**立即随当前缓冲一并 flush**，避免工具里程碑被长时间延迟；仍不阻塞调用方（仅持锁推送向量）。
//!
//! 热路径只做 `Mutex` 保护与 `Vec` 推送；下游 `deliver_batch` 须保持轻量（例如 channel `try_send` 或追加队列）。Tauri `emit` 应在异步线程调用，避免长时间占 UI 线程（NFR-P1/P2）。

use crate::CortexRuntime;
use serde::de::Error as SerdeDeError;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 与 `CortexState` / 1.3 契约一致的 schema 版本字段语义（v0 = 0）。
pub const PROCESS_EVENT_SCHEMA_VERSION_V0: u32 = 0;

/// 蜂群编排终局原因（ADR-E **`StopReason`** 子集；与 `swarm_run_*` 事件配对，Story 1.8 / FR20）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SwarmRunStopReason {
    /// 满足完成定义（`ConvergencePolicy` / done 谓词）。
    Done,
    /// 达到 `max_internal_rounds`（或等价预算）触顶。
    BudgetExceeded,
    /// 墙钟或外部超时。
    Timeout,
    /// 不可恢复错误；详情见 `message`。
    Error,
}

impl SwarmRunStopReason {
    /// JSON `stopReason` 字面量（camelCase），与 `#[serde(rename_all = "camelCase")]` 一致。
    #[must_use]
    pub fn as_stop_reason_wire_str(self) -> &'static str {
        match self {
            Self::Done => "done",
            Self::BudgetExceeded => "budgetExceeded",
            Self::Timeout => "timeout",
            Self::Error => "error",
        }
    }
}

/// v0 白名单事件名（wire：`snake_case`，NFR-I2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessEventNameV0 {
    /// 执行阶段推进（如 agent 迭代索引）。
    SwarmPhaseChanged,
    /// 工具开始执行（载荷仅含摘要与 id，不含完整参数/结果）。
    ToolCallStarted,
    /// 工具结束（成功或错误摘要由 `message` 承载，大块结果留在会话/总线）。
    ToolCallFinished,
    /// 蜂群 run 正常结束或超时/错误终局（载荷须带 `stop_reason`：`Done` | `Timeout` | `Error`）。
    ///
    /// **与 [`Self::SwarmRunCapped`] 边界：** 仅 **预算/轮次触顶**（`BudgetExceeded`）发 `swarm_run_capped`；
    /// 其它终局原因走本事件。
    SwarmRunFinished,
    /// 内部轮次（或等价预算）触顶（`stop_reason` 应为 [`SwarmRunStopReason::BudgetExceeded`]）。
    SwarmRunCapped,
}

fn deserialize_process_event_schema_v0<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    let v = u32::deserialize(deserializer)?;
    if v != PROCESS_EVENT_SCHEMA_VERSION_V0 {
        return Err(serde::de::Error::custom(format!(
            "ProcessEventV0 schemaVersion must be {} for v0 wire (got {v})",
            PROCESS_EVENT_SCHEMA_VERSION_V0
        )));
    }
    Ok(v)
}

/// 工具/错误摘要写入过程事件前的 **单行、限长** 消毒（NFR-S2：降低路径/控制字符进入可观测管道）。
///
/// - 控制字符（含换行）压成空格并 **压平空白**；
/// - 最多保留 `max_chars` 个 Unicode 标量（不含省略号）；超出则截断并加 `"..."`。
#[must_use]
pub fn sanitize_tool_summary_for_process_event(raw: &str, max_chars: usize) -> String {
    let max_chars = max_chars.max(1);
    let collapsed: String = raw
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let mut out: String = collapsed.chars().take(max_chars).collect();
    if collapsed.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProcessEventV0Wire {
    #[serde(deserialize_with = "deserialize_process_event_schema_v0")]
    schema_version: u32,
    name: ProcessEventNameV0,
    message: String,
    #[serde(default)]
    phase_id: Option<String>,
    #[serde(default)]
    correlation_id: Option<String>,
    #[serde(default)]
    tool_name: Option<String>,
    #[serde(default)]
    stop_reason: Option<SwarmRunStopReason>,
}

fn validate_wire_event_invariants(w: &ProcessEventV0Wire) -> Result<(), String> {
    match w.name {
        ProcessEventNameV0::SwarmRunFinished => match w.stop_reason {
            None => Err("swarm_run_finished requires stopReason (done|timeout|error)".to_string()),
            Some(SwarmRunStopReason::BudgetExceeded) => Err(
                "swarm_run_finished must not use stopReason budgetExceeded; use swarm_run_capped"
                    .to_string(),
            ),
            Some(_) => Ok(()),
        },
        ProcessEventNameV0::SwarmRunCapped => match w.stop_reason {
            None => Err("swarm_run_capped requires stopReason budgetExceeded".to_string()),
            Some(SwarmRunStopReason::BudgetExceeded) => Ok(()),
            Some(_) => Err("swarm_run_capped requires stopReason to be budgetExceeded only".to_string()),
        },
        ProcessEventNameV0::SwarmPhaseChanged
        | ProcessEventNameV0::ToolCallStarted
        | ProcessEventNameV0::ToolCallFinished => {
            if w.stop_reason.is_some() {
                Err("stopReason is only valid for swarm_run_finished and swarm_run_capped".to_string())
            } else {
                Ok(())
            }
        }
    }
}

/// 单一过程事件 DTO（v0）。字段保持小而稳定；扩展须 bump `schema_version` 或经 ADR。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessEventV0 {
    pub schema_version: u32,
    pub name: ProcessEventNameV0,
    /// 人可读短句（预览级，非完整工具 I/O）。
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    /// 终局类事件（`swarm_run_*`）携带，与 ADR-E `StopReason` 对齐（NFR-I2 白名单字段）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<SwarmRunStopReason>,
}

impl<'de> Deserialize<'de> for ProcessEventV0 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let w = ProcessEventV0Wire::deserialize(deserializer)?;
        validate_wire_event_invariants(&w).map_err(SerdeDeError::custom)?;
        Ok(ProcessEventV0 {
            schema_version: w.schema_version,
            name: w.name,
            message: w.message,
            phase_id: w.phase_id,
            correlation_id: w.correlation_id,
            tool_name: w.tool_name,
            stop_reason: w.stop_reason,
        })
    }
}

impl ProcessEventV0 {
    #[must_use]
    pub fn swarm_phase_changed(phase_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            schema_version: PROCESS_EVENT_SCHEMA_VERSION_V0,
            name: ProcessEventNameV0::SwarmPhaseChanged,
            message: message.into(),
            phase_id: Some(phase_id.into()),
            correlation_id: None,
            tool_name: None,
            stop_reason: None,
        }
    }

    #[must_use]
    pub fn tool_call_started(
        tool_name: impl Into<String>,
        message: impl Into<String>,
        correlation_id: impl Into<String>,
    ) -> Self {
        let t = tool_name.into();
        Self {
            schema_version: PROCESS_EVENT_SCHEMA_VERSION_V0,
            name: ProcessEventNameV0::ToolCallStarted,
            message: message.into(),
            phase_id: None,
            correlation_id: Some(correlation_id.into()),
            tool_name: Some(t),
            stop_reason: None,
        }
    }

    #[must_use]
    pub fn tool_call_finished(
        tool_name: impl Into<String>,
        message: impl Into<String>,
        correlation_id: impl Into<String>,
    ) -> Self {
        let t = tool_name.into();
        Self {
            schema_version: PROCESS_EVENT_SCHEMA_VERSION_V0,
            name: ProcessEventNameV0::ToolCallFinished,
            message: message.into(),
            phase_id: None,
            correlation_id: Some(correlation_id.into()),
            tool_name: Some(t),
            stop_reason: None,
        }
    }

    /// 终局事件 **`swarm_run_finished`**：`stopReason` 须为 **`done` / `timeout` / `error`**（与 ADR-E、[`process-events-v0.md`](./process-events-v0.md) 一致）。
    ///
    /// **`BudgetExceeded` 不得传入** — 若误传，记录 `warn` 并改为构造 [`Self::swarm_run_capped`]（避免线上误发 `finished`+`budgetExceeded`）。
    #[must_use]
    pub fn swarm_run_finished(stop_reason: SwarmRunStopReason, message: impl Into<String>) -> Self {
        if stop_reason == SwarmRunStopReason::BudgetExceeded {
            tracing::warn!(
                target: "agent_diva_swarm::process_events",
                "swarm_run_finished: BudgetExceeded is invalid; emitting swarm_run_capped per ADR-E"
            );
            return Self::swarm_run_capped(message);
        }
        Self {
            schema_version: PROCESS_EVENT_SCHEMA_VERSION_V0,
            name: ProcessEventNameV0::SwarmRunFinished,
            message: message.into(),
            phase_id: None,
            correlation_id: None,
            tool_name: None,
            stop_reason: Some(stop_reason),
        }
    }

    /// 终局事件 **`swarm_run_capped`**：`stopReason` **固定**为 [`SwarmRunStopReason::BudgetExceeded`]（内部轮次 / 等价预算触顶）。
    #[must_use]
    pub fn swarm_run_capped(message: impl Into<String>) -> Self {
        Self {
            schema_version: PROCESS_EVENT_SCHEMA_VERSION_V0,
            name: ProcessEventNameV0::SwarmRunCapped,
            message: message.into(),
            phase_id: None,
            correlation_id: None,
            tool_name: None,
            stop_reason: Some(SwarmRunStopReason::BudgetExceeded),
        }
    }
}

/// 批处理下发目标（单一出口后的订阅端适配层实现此 trait：如 GUI channel、测试探针）。
pub trait ProcessEventBatchSink: Send + Sync {
    fn deliver_batch(&self, batch: Vec<ProcessEventV0>);
}

/// 节流与批处理配置。
#[derive(Debug, Clone)]
pub struct ProcessEventThrottleConfig {
    /// `swarm_phase_changed` 等可合并事件的最长等待窗口。
    pub flush_interval: Duration,
    /// 任一时刻缓冲内最大条数；超出则强制 flush。
    pub max_batch: usize,
}

impl Default for ProcessEventThrottleConfig {
    fn default() -> Self {
        Self {
            flush_interval: Duration::from_millis(100),
            max_batch: 32,
        }
    }
}

#[derive(Debug)]
struct ThrottleState {
    pending: Vec<ProcessEventV0>,
    last_flush: Instant,
    config: ProcessEventThrottleConfig,
    /// 上次观察到的皮层开关；用于在 **ON→OFF** 时丢弃缓冲，避免再次 ON 后误投递「关皮层期间不应发出」的积压。
    last_cortex_enabled: bool,
}

impl ThrottleState {
    fn new(config: ProcessEventThrottleConfig, cortex_enabled: bool) -> Self {
        Self {
            pending: Vec::new(),
            last_flush: Instant::now(),
            config,
            last_cortex_enabled: cortex_enabled,
        }
    }

    /// 在 `try_emit` / `flush_pending` 入口调用：若刚从开变为关，丢弃 `pending`（不调用 sink）。
    fn sync_cortex_transition_discard_if_off(&mut self, cortex_enabled: bool) {
        if self.last_cortex_enabled && !cortex_enabled {
            let _ = self.drain();
        }
        self.last_cortex_enabled = cortex_enabled;
    }

    fn should_flush_after_push(&self, last_event: &ProcessEventV0) -> bool {
        if self.pending.len() >= self.config.max_batch {
            return true;
        }
        match last_event.name {
            ProcessEventNameV0::ToolCallStarted
            | ProcessEventNameV0::ToolCallFinished
            | ProcessEventNameV0::SwarmRunFinished
            | ProcessEventNameV0::SwarmRunCapped => true,
            ProcessEventNameV0::SwarmPhaseChanged => {
                self.last_flush.elapsed() >= self.config.flush_interval
            }
        }
    }

    fn drain(&mut self) -> Vec<ProcessEventV0> {
        self.last_flush = Instant::now();
        std::mem::take(&mut self.pending)
    }
}

/// **皮层 ON** 时向节流层供稿；**皮层 OFF** 时丢弃（与 Story 1.4 简化语义一致，见 `docs/PROCESS_EVENTS_CORTEX_OFF.md`）。
pub struct ProcessEventPipeline {
    cortex: Arc<CortexRuntime>,
    state: Mutex<ThrottleState>,
    sink: Arc<dyn ProcessEventBatchSink>,
}

impl fmt::Debug for ProcessEventPipeline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProcessEventPipeline")
            .field("cortex_enabled", &self.cortex.snapshot().enabled)
            .field("sink", &"<dyn ProcessEventBatchSink>")
            .finish_non_exhaustive()
    }
}

impl ProcessEventPipeline {
    #[must_use]
    pub fn new(
        cortex: Arc<CortexRuntime>,
        sink: Arc<dyn ProcessEventBatchSink>,
        config: ProcessEventThrottleConfig,
    ) -> Self {
        let cortex_enabled = cortex.snapshot().enabled;
        Self {
            cortex,
            state: Mutex::new(ThrottleState::new(config, cortex_enabled)),
            sink,
        }
    }

    /// 与构造时注入的 [`CortexRuntime`] 共享同一 `Arc`，供编排入口读取皮层开关（须与发射侧一致）。
    #[must_use]
    pub fn cortex_runtime(&self) -> Arc<CortexRuntime> {
        self.cortex.clone()
    }

    /// 尝试发射一条事件：皮层关则为 no-op；否则入缓冲并按策略可能立即 flush。
    pub fn try_emit(&self, event: ProcessEventV0) {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let enabled = self.cortex.snapshot().enabled;
        guard.sync_cortex_transition_discard_if_off(enabled);
        if !enabled {
            return;
        }
        guard.pending.push(event);
        let flush = guard
            .pending
            .last()
            .is_some_and(|last| guard.should_flush_after_push(last));
        if flush {
            let batch = guard.drain();
            drop(guard);
            if !batch.is_empty() {
                self.sink.deliver_batch(batch);
            }
        }
    }

    /// 将当前缓冲全部下发（测试或 turn 结束时调用，避免挂起未达时间窗的 phase 事件）。
    pub fn flush_pending(&self) {
        let batch = {
            let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
            let enabled = self.cortex.snapshot().enabled;
            guard.sync_cortex_transition_discard_if_off(enabled);
            if !enabled {
                return;
            }
            guard.drain()
        };
        if !batch.is_empty() {
            self.sink.deliver_batch(batch);
        }
    }
}

/// 测试与集成用的内存探针。
#[derive(Debug, Default, Clone)]
pub struct ProcessEventRecorder {
    inner: Arc<Mutex<Vec<ProcessEventV0>>>,
}

impl ProcessEventRecorder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn snapshot(&self) -> Vec<ProcessEventV0> {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// 已记录的过程事件 **条数**（跨批次累计；非「批次数」）。
    #[must_use]
    pub fn recorded_event_count(&self) -> usize {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .len()
    }
}

impl ProcessEventBatchSink for ProcessEventRecorder {
    fn deliver_batch(&self, batch: Vec<ProcessEventV0>) {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .extend(batch);
    }
}

/// 将 [`ProcessEventRecorder`] 包成 [`ProcessEventBatchSink`] 的 `Arc`。
#[must_use]
pub fn recorder_sink() -> (Arc<ProcessEventRecorder>, Arc<dyn ProcessEventBatchSink>) {
    let r = Arc::new(ProcessEventRecorder::new());
    let sink: Arc<dyn ProcessEventBatchSink> = r.clone();
    (r, sink)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CortexState;

    struct CountingSink {
        batches: Mutex<Vec<usize>>,
    }

    impl ProcessEventBatchSink for CountingSink {
        fn deliver_batch(&self, batch: Vec<ProcessEventV0>) {
            self.batches
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(batch.len());
        }
    }

    #[test]
    fn cortex_off_drops_events() {
        let cortex = Arc::new(CortexRuntime::from_state(CortexState::with_enabled(false)));
        let (rec, sink) = recorder_sink();
        let pipe = ProcessEventPipeline::new(
            cortex,
            sink,
            ProcessEventThrottleConfig {
                flush_interval: Duration::from_secs(60),
                max_batch: 100,
            },
        );
        pipe.try_emit(ProcessEventV0::swarm_phase_changed("p1", "x"));
        pipe.flush_pending();
        assert_eq!(rec.recorded_event_count(), 0);
    }

    /// 皮层 ON 期间进入缓冲的 phase 事件，在 **关皮层** 后不得在下一次 **开** 再 `flush` 时误投递。
    #[test]
    fn pending_discarded_when_cortex_turns_off_before_reenable() {
        let cortex = Arc::new(CortexRuntime::new());
        let (rec, sink) = recorder_sink();
        let pipe = ProcessEventPipeline::new(
            Arc::clone(&cortex),
            sink,
            ProcessEventThrottleConfig {
                flush_interval: Duration::from_secs(60),
                max_batch: 100,
            },
        );
        pipe.try_emit(ProcessEventV0::swarm_phase_changed("p1", "queued"));
        cortex.set_enabled(false);
        pipe.try_emit(ProcessEventV0::swarm_phase_changed("p2", "ignored"));
        assert_eq!(rec.recorded_event_count(), 0);
        cortex.set_enabled(true);
        pipe.flush_pending();
        assert_eq!(
            rec.recorded_event_count(),
            0,
            "stale pending must not flush after OFF window"
        );
    }

    #[test]
    fn process_event_v0_json_rejects_nonzero_schema_version() {
        let bad = r#"{"schemaVersion":1,"name":"swarm_phase_changed","message":"x"}"#;
        let r: Result<ProcessEventV0, _> = serde_json::from_str(bad);
        assert!(r.is_err());
    }

    #[test]
    fn swarm_run_stop_reason_wire_str_matches_serde_json() {
        for r in [
            SwarmRunStopReason::Done,
            SwarmRunStopReason::BudgetExceeded,
            SwarmRunStopReason::Timeout,
            SwarmRunStopReason::Error,
        ] {
            let s = serde_json::to_string(&r).expect("serde_json");
            let expected = format!("\"{}\"", r.as_stop_reason_wire_str());
            assert_eq!(s, expected, "wire str must match serde_json for {r:?}");
        }
    }

    #[test]
    fn swarm_run_finished_budget_exceeded_coerces_to_capped() {
        let ev = ProcessEventV0::swarm_run_finished(
            SwarmRunStopReason::BudgetExceeded,
            "misuse",
        );
        assert_eq!(ev.name, ProcessEventNameV0::SwarmRunCapped);
        assert_eq!(ev.stop_reason, Some(SwarmRunStopReason::BudgetExceeded));
    }

    #[test]
    fn deserialize_swarm_run_finished_requires_stop_reason() {
        let bad = r#"{"schemaVersion":0,"name":"swarm_run_finished","message":"x"}"#;
        assert!(serde_json::from_str::<ProcessEventV0>(bad).is_err());
    }

    #[test]
    fn deserialize_swarm_run_finished_rejects_budget_exceeded() {
        let bad = r#"{"schemaVersion":0,"name":"swarm_run_finished","message":"x","stopReason":"budgetExceeded"}"#;
        assert!(serde_json::from_str::<ProcessEventV0>(bad).is_err());
    }

    #[test]
    fn deserialize_swarm_run_capped_requires_budget_exceeded() {
        let bad = r#"{"schemaVersion":0,"name":"swarm_run_capped","message":"x","stopReason":"done"}"#;
        assert!(serde_json::from_str::<ProcessEventV0>(bad).is_err());
    }

    #[test]
    fn deserialize_phase_rejects_stop_reason() {
        let bad = r#"{"schemaVersion":0,"name":"swarm_phase_changed","message":"x","stopReason":"done"}"#;
        assert!(serde_json::from_str::<ProcessEventV0>(bad).is_err());
    }

    #[test]
    fn sanitize_tool_summary_flattens_and_truncates() {
        let s = sanitize_tool_summary_for_process_event("line1\nline2\tsecret", 8);
        assert!(s.ends_with("..."));
        assert!(!s.contains('\n'));
        assert!(s.contains("line1"));
    }

    #[test]
    fn high_frequency_phase_merged_until_interval_or_max_batch() {
        let cortex = Arc::new(CortexRuntime::new());
        let counting = Arc::new(CountingSink {
            batches: Mutex::new(Vec::new()),
        });
        let sink: Arc<dyn ProcessEventBatchSink> = counting.clone();
        let pipe = ProcessEventPipeline::new(
            cortex,
            sink,
            ProcessEventThrottleConfig {
                flush_interval: Duration::from_secs(10),
                max_batch: 5,
            },
        );
        for i in 0..12 {
            pipe.try_emit(ProcessEventV0::swarm_phase_changed(
                format!("i{i}"),
                "phase",
            ));
        }
        pipe.flush_pending();
        let sizes = counting
            .batches
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        assert_eq!(
            sizes.iter().sum::<usize>(),
            12,
            "all events delivered: {:?}",
            sizes
        );
        assert!(sizes.iter().all(|&n| n <= 5));
    }

    #[test]
    fn tool_events_trigger_immediate_flush_per_push() {
        let cortex = Arc::new(CortexRuntime::new());
        let rec = Arc::new(ProcessEventRecorder::new());
        let sink: Arc<dyn ProcessEventBatchSink> = rec.clone();
        let pipe = ProcessEventPipeline::new(
            cortex,
            sink,
            ProcessEventThrottleConfig {
                flush_interval: Duration::from_secs(60),
                max_batch: 100,
            },
        );
        pipe.try_emit(ProcessEventV0::tool_call_started(
            "read_file",
            "start",
            "c1",
        ));
        assert!(!rec.snapshot().is_empty());
        pipe.try_emit(ProcessEventV0::tool_call_finished(
            "read_file",
            "done",
            "c1",
        ));
        let names: Vec<_> = rec.snapshot().iter().map(|e| e.name).collect();
        assert!(names.contains(&ProcessEventNameV0::ToolCallStarted));
        assert!(names.contains(&ProcessEventNameV0::ToolCallFinished));
    }
}
