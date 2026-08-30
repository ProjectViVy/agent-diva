# Acceptance

- [x] Manager loopback route `/api/neuro-link/v1/ws` 可升级并完成 v1 hello。
- [x] `service/list` 与 `session/open` 返回 revisioned Service Bindings 和 state-sync head。
- [x] Projection Journal 支持 append/replay/ACK、retention 常量、session 清理和幂等键冲突。
- [x] 注入 typed runtime 后 `turn/start` 返回 request/trace/admission，并投影
      `turn/admission` notification。
- [x] 未安装 typed runtime 时返回 `service_unavailable`，不调用旧 bus。
- [ ] AgentLoop/Fabric production runtime wiring；见 C3c backlog。
- [ ] 桌面 GUI 完整 Neuro-Link client migration；见 C4。

