# Acceptance

- [x] Production `AppState` 安装 `AgentLoopNeuroLinkRuntime` typed seam。
- [x] `turn/start` 的 admission reply 由 AgentLoop bounded dispatcher 产生，而非伪造 running。
- [x] `turn/cancel` 通过 typed runtime-control `StopSession` 返回真实 control outcome。
- [x] opaque session key、request ID、trace ID 在 ingress 边界保持一致。
- [ ] conversation/tool/presentation 事件的 Journal fan-out（C3e）。
- [ ] 旧 MessageBus/DTO 删除与原子 clean-break（C6）。

