//! 将 [`ProcessEventV0`] 批经 [`MessageBus`] 以 [`AgentEvent::SwarmProcessBatch`] 下发（Story 2.3），与聊天 token 流式 **分轨**。

use agent_diva_core::bus::{AgentEvent, MessageBus};
use agent_diva_swarm::{ProcessEventBatchSink, ProcessEventV0};
use std::sync::{Arc, Mutex};

/// 与当前 turn 的 `(channel, chat_id)` 对齐；在 [`crate::agent_loop::AgentLoop`] 处理单条 inbound 时写入，turn 结束清空。
pub struct BusSwarmProcessSink {
    bus: MessageBus,
    route: Arc<Mutex<Option<(String, String)>>>,
}

impl BusSwarmProcessSink {
    #[must_use]
    pub fn new(bus: MessageBus, route: Arc<Mutex<Option<(String, String)>>>) -> Self {
        Self { bus, route }
    }
}

impl ProcessEventBatchSink for BusSwarmProcessSink {
    fn deliver_batch(&self, batch: Vec<ProcessEventV0>) {
        if batch.is_empty() {
            return;
        }
        let route = self
            .route
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let Some((channel, chat_id)) = route.as_ref() else {
            return;
        };
        let events: Vec<serde_json::Value> = batch
            .into_iter()
            .filter_map(|e| serde_json::to_value(e).ok())
            .collect();
        if events.is_empty() {
            return;
        }
        let _ = self.bus.publish_event(
            channel.clone(),
            chat_id.clone(),
            AgentEvent::SwarmProcessBatch { events },
        );
    }
}
