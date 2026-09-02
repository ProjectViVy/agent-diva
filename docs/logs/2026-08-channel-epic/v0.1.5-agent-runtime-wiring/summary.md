# AgentLoop typed admission wiring

本迭代把 C3 Gateway 的 typed `NeuroLinkRuntime` seam 安装到 production bootstrap：

- `RuntimeControlCommand::StartChannelTurn` 携带 boxed `ChannelEnvelopeV1`，在 AgentLoop
  root select loop 中快速转交 per-session bounded dispatcher；不会阻塞 root loop 等待 provider。
- Manager 的 `AgentLoopNeuroLinkRuntime` 等待 queued/running admission reply，并校验
  `session_key/request_id/trace_id` 三元 correlation；cancel 走同一 typed control lane。
- typed content parts 在 AgentLoop 边界转换为当前 turn worker 输入；opaque session key
  通过内部 metadata override 保持不被 `channel:chat_id` 重写。

事件 fan-out 仍明确列为 C3e：本次不伪造 conversation/final/tool projection，也不删除旧
MessageBus/DTO；C6 clean break 前完成最终迁移。

