# AgentEvent projection hub

本迭代完成 C3e，将 AgentLoop/Fabric 的关联 AgentEvent 接入 Neuro-Link 的统一
projection stream：

- 新增 `NeuroLinkProjectionHub`，每个 Manager 进程只启动一个 MessageBus 消费/SQLite
  写入/广播泵，避免每个 WebSocket 重复落盘。
- 关联到 `neuro-link` 会话的 conversation、reasoning、tool、planning、provider、context
  与 terminal 事件映射为受限 `ChannelEnvelopeV1`；工具结果正文不进入 projection。
- `durable=false` 的增量只进入实时广播，`durable=true` 的生命周期/状态事件写入
  `super-channel-events.db`，断线由 `state/resume` 恢复。
- WebSocket 在 hello 后订阅共享 hub，只向当前 `session_key` 转发匹配通知；广播落盘成功后
  才发送，游标继续由 Projection Journal 分配。
- `ProjectionJournal::append_event` 保留 transient cursor，同时 replay 仅返回 durable 行；
  初始 cursor `0` 可跨越 transient gap 到达第一条 durable 事件。

C6 仍需完成旧 MessageBus/DTO 的最终 clean break；本迭代将 MessageBus 作为兼容观察面，未
删除既有通道路径。
