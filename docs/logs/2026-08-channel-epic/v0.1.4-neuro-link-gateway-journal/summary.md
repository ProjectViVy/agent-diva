# Neuro-Link v1 Gateway / Projection Journal

本迭代完成 C3 的 Gateway、Projection Journal 与 Service Catalog 基础设施：

- Manager 在固定 loopback listener 暴露 `/api/neuro-link/v1/ws`，执行 JSON-RPC
  `protocol/hello`、`service/list`、`session/open`、`turn/start`、`turn/cancel`、
  `event/ack` 和 `state/resume`。
- 新增 profile-local `super-channel-events.db`，提供 stream cursor、ACK、replay、
  retention、session 清理和 command idempotency。
- Rust core、JSON Schema、GUI TypeScript 同步 `RpcId=null`、稳定错误码、catalog revision、
  state-sync、admission 和 projection result 类型。
- Service Catalog 登记 Neuro-Link 以及现有 Session、Workspace、Persona、Memory、Planning、
  Approval、Ask User、Files、Skills、Providers、Cron、AutoDream、Audit、Health HTTP 服务，
  不复制领域 handler。
- Gateway 只依赖可注入的 typed `NeuroLinkRuntime`；未安装 runtime 时返回明确的
  `service_unavailable`，不会偷偷转发到旧 MessageBus。

本迭代不切换桌面 GUI 实时链路，也不删除旧 SSE/pipe；这些属于后续 C4/C6 的原子切换范围。

