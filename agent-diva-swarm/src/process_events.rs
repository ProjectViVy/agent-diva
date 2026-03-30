//! 最小过程事件（FR2 发射侧，Story 1.5）：白名单 DTO、皮层门控与节流/批处理出口。
//!
//! # 默认节流策略（NFR-P2）
//!
//! - **`swarm_phase_changed`**：进入 **批处理缓冲**；在 **100ms** 合并窗口内或达到 **`max_batch`（默认 32）** 条时 **整批** 下发到下游。
//! - **`tool_call_started` / `tool_call_finished`**：**立即随当前缓冲一并 flush**，避免工具里程碑被长时间延迟；仍不阻塞调用方（仅持锁推送向量）。
//!
//! 热路径只做 `Mutex` 保护与 `Vec` 推送；下游 `deliver_batch` 须保持轻量（例如 channel `try_send` 或追加队列）。Tauri `emit` 应在异步线程调用，避免长时间占 UI 线程（NFR-P1/P2）。

use crate::CortexRuntime;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 与 `CortexState` / 1.3 契约一致的 schema 版本字段语义（v0 = 0）。
pub const PROCESS_EVENT_SCHEMA_VERSION_V0: u32 = 0;

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
}

/// 单一过程事件 DTO（v0）。字段保持小而稳定；扩展须 bump `schema_version` 或经 ADR。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}

impl ThrottleState {
    fn new(config: ProcessEventThrottleConfig) -> Self {
        Self {
            pending: Vec::new(),
            last_flush: Instant::now(),
            config,
        }
    }

    fn should_flush_after_push(&self, last_event: &ProcessEventV0) -> bool {
        if self.pending.len() >= self.config.max_batch {
            return true;
        }
        match last_event.name {
            ProcessEventNameV0::ToolCallStarted | ProcessEventNameV0::ToolCallFinished => true,
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
        Self {
            cortex,
            state: Mutex::new(ThrottleState::new(config)),
            sink,
        }
    }

    /// 尝试发射一条事件：皮层关则为 no-op；否则入缓冲并按策略可能立即 flush。
    pub fn try_emit(&self, event: ProcessEventV0) {
        if !self.cortex.snapshot().enabled {
            return;
        }
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        guard.pending.push(event);
        let last = guard.pending.last().expect("just pushed");
        let flush = guard.should_flush_after_push(last);
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
        if !self.cortex.snapshot().enabled {
            return;
        }
        let batch = {
            let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
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

    pub fn batch_count(&self) -> usize {
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
        assert_eq!(rec.batch_count(), 0);
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
