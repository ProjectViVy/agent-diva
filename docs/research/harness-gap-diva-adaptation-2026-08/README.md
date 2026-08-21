# Harness Gap：Diva 化适配研究包

> 日期：2026-08-22
> 状态：Research / Proposal，未授权生产实现
> 目标：在当前 Agent Diva 代码和本地参考项目的真实实现上，收敛下一阶段 Harness 研究；只吸收可证明的运行时纪律，不搬运参考项目的产品骨架。

## 结论先行

当前 Diva 不需要重新建一套 Plan、Provider Stream、Approval 或 Prompt Injection
框架：

- Plan Mode 已有 `PlanModeState`、`ToolCapability` fail-closed 矩阵，并在工具执行
  seam 再检查；旧报告中“Plan 只有提示词”的判断已过时。
- Provider 已把流式输出归一为 `LLMStreamEvent`，agent loop 消费 text/reasoning/tool
  delta，Manager 继续向 GUI/CLI 转发；token ledger 也已存在。
- Sandbox 已有独立的审批协调器、Once/Session/Global 授权、Guardian 和平台沙箱；不能
  用 OpenHarness 的简单 permission mode 替换它。
- Core Bus 已有 `AgentEvent` 与 Poke 8 事件广播，但还没有可组合的、可阻断的 Rust
  Trait Hook 管道。
- 同一 `session_key` 的 turn admission 目前只做熔断和小时速率拒绝；消息传输层仍使用
  unbounded channel，没有可观察的有界 FIFO 准入队列和队列满/等待超时语义。

因此下一步研究/设计应聚焦两个相互独立的窄问题：

1. **Hook Kernel**：在已存在的 Bus/Audit/Agent Loop seam 上增加确定性、可观测、默认不
   改写数据的 Hook Registry；
2. **Session Admission**：在 Agent Loop 入口增加 per-session 有界串行准入，不替换现有
   MessageBus，也不改变 Memory/Persona/Laputa 权威。

## 研究证据

- [当前状态与证据](./current-state-and-evidence.md)
- [参考项目比较](./reference-comparison.md)
- [Diva 化适配方案](./diva-adaptation-proposal.md)

参考源码快照（本机 `.workspace`）：

| 项目 | 快照 | 本研究实际查看的能力 |
| --- | --- | --- |
| OpenHarness | `bf5931e` | HookEvent/HookResult、PermissionMode、Plugin 生命周期、流式事件 |
| Claude Code | `7beeb9c6` | 终态/继续原因、token budget continuation、stop hooks、daemon worker |
| ZeroClaw | `d91e08eae` | Trait Hook 的修改/观察分流、优先级、SessionActorQueue、stall watchdog、WASM 权限边界 |
| OpenFang | `acf2587` | typed event envelope、target/payload、风险分级 Approval、schema normalization |
| GenericAgent | `ee5a474` | 仅作 Evolution/上下文密度反例与边界，不作为 Harness API 模板 |

## 研究边界

本包不授权以下工作：

- 引入 OpenHarness 的 Python/HTTP/Prompt Hook；
- 引入 ZeroClaw/OpenFang 的 WASM 插件、全局插件市场或 SSRF host API；
- 重写 Diva 的 Sandbox/Approval/Guardian；
- 把 Hook 变成 BML、Persona、ACTMEM、Evolution 或治理写入口；
- 因为参考项目存在 `toolset`、ACP、remote control、worktree，就直接扩张 Diva 的
  产品面。

## 研究 Gate

本包完成的是“当前事实 + 适配设计”。进入生产前仍需用户确认：

1. 是否接受 Hook 只在 core/runtime seam 提供，而不开放任意脚本执行；
2. 是否接受 session queue 的默认上限、等待超时和队列满错误需要产品配置；
3. 是否把 GUI/CLI 的 hook/queue 观测列为同一迭代的验收项。
