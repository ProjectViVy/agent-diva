# Agent Loop / Manager / GUI 原位治理架构

> 状态：目标架构与实施边界，尚未授权运行时代码改造。
> 基线：`agent-diva-pro@66e0deb4` 的当前设计；工作区内未提交代码仅用于发现最新热点，不视为本文交付。
> 参考证据：`refactor/deep-governance@95dd9833`，只提取原则，不移植其 clean-break 实现。

## 1. 结论

deep-governance 失败在产品替代策略，不是所有设计原则都错误。它通过删除旧树、另建 kernel/runtime/state/manager v1 获得了整洁的 authority spine，却同时丢失多轮 Agent Loop、`tool_calls`、完整 sandbox、Provider/Channel 生态和真实 GUI。其自身评估记录了约 `+44k / -233k`、Rust 体量约为 pro 的 `1/9`，因此不能作为当前产品的实现基线。

本轮采用 **原位绞杀式治理（in-place strangler refactor）**：

- 保留现有 crate、公开 API、`/api/*`、存储格式、事件和用户旅程。
- 先刻画行为，再在现有模块内部建立 seam，逐域替换巨型函数和重复协议。
- 不建立第二套 kernel、runtime、Manager v1、状态库或 Tool Gateway。
- 深分支仅贡献四个约束：单一副作用路径、Manager 薄宿主、GUI Host 边界、未接通能力 fail-closed。

## 2. 当前问题画像

### 2.1 Agent Loop

`AgentLoop` 同时持有 provider、session、context、tool registry、subagent、memory、runtime control、mask、token budget 等状态，见 [agent_loop.rs](../../../agent-diva-agent/src/agent_loop.rs:123)。`process_inbound_message_inner` 从消息接入一路处理到最终持久化，入口见 [loop_turn.rs](../../../agent-diva-agent/src/agent_loop/loop_turn.rs:294)，同一实现还承担：

- mask/model 决议；
- session、媒体、memory prefetch 和 context 装配；
- plan phase 与动态工具重建；
- provider 流式循环和 tool call 执行；
- token 记账、压缩、事件投影、空回复兜底和保存。

当前已出现“同一 policy phase 在 turn 起点、tool 后重建、runtime-control 后分别推导”的修复痕迹；共享 helper [policy_phase_for](../../../agent-diva-agent/src/agent_loop.rs:233) 是可复用 seam，但调用链仍散落。`ChatPlanUpdate` 事件只广播不持久化，见 [loop_turn.rs](../../../agent-diva-agent/src/agent_loop/loop_turn.rs:198)，也说明事件、事实和 UI 投影尚未完全分层。

### 2.2 Manager

Manager 的主要问题不是缺少新框架，而是职责聚集：

- [handlers.rs](../../../agent-diva-manager/src/handlers.rs:58) 从 chat/session 一直承载 config、skills、MCP、files、cron 和 SSE 映射；
- [state.rs](../../../agent-diva-manager/src/state.rs:60) 同时放置 `AppState`、runtime command 和大量 HTTP DTO；
- [runtime.rs](../../../agent-diva-manager/src/runtime.rs:291) 同时负责 provider、tool config、cron、AgentLoop 和 gateway 启动；
- [manager.rs](../../../agent-diva-manager/src/manager.rs:26) 既是 façade，又承担 provider/config/runtime 管理。

已有 `handlers/{audit,autodream,health,laputa,logs,planning,todo,token_stats}.rs` 和 `runtime/{bootstrap,shutdown,task_runtime}.rs`，说明最安全的方向是继续按现有模块化路线拆分，而不是重写 transport。

### 2.3 GUI 与 Tauri

[App.vue](../../../agent-diva-gui/src/App.vue:192) 持有聊天、连接、session、plan、provider、config、工具和启动状态；同一文件还做 DTO 映射、session cache、stream reconciliation、plan 状态机和 gateway 健康检查。

[commands.rs](../../../agent-diva-gui/src-tauri/src/commands.rs:253) 同时包含 Host 本地能力、Manager HTTP 代理、DTO、流事件翻译和大量业务命令；[desktop.ts](../../../agent-diva-gui/src/api/desktop.ts:4) 再镜像一遍这些 DTO。Tauri 已经通过 [app_state.rs](../../../agent-diva-gui/src-tauri/src/app_state.rs:7) 代理当前 Manager `/api`，而 [lib.rs](../../../agent-diva-gui/src-tauri/src/lib.rs:81) 还负责嵌入式 gateway 装配和桌面生命周期。

问题不是“用了 invoke”，而是 **业务 API、Host 本地能力、状态投影和 UI 编排没有明确分区**。

## 3. 目标架构

```text
Vue views/components
        |
GUI domain composables + one API barrel
        |
typed transport adapters
  |                     |
Manager /api         Tauri LOCAL only
  |                     |
thin handlers       window/prefs/pet/log/lifecycle
        |
existing Manager domain services
        |
AgentLoop façade
        |
TurnCoordinator
  -> admission/session
  -> context/memory
  -> model step
  -> governed tool step
  -> persistence/events/finalization
        |
existing ToolRegistry + Plan policy + Sandbox
```

### 3.1 Agent Loop 边界

保留 `AgentLoop` 作为公开 façade 和长生命周期依赖容器。新增的内部类型只表达现有流程：

- `TurnRequest`：规范化的本轮输入、trace、取消信号；
- `TurnSnapshot`：model、mask、plan phase、session key、预算等一次性决议；
- `TurnCoordinator`：按固定阶段编排，不直接实现每个阶段；
- `ModelStepRunner`：provider streaming 与 tool-call 迭代；
- `GovernedToolStep`：唯一调用现有 `ToolRegistry` 的 turn 内 seam，并复用现有 Plan/Mask/Sandbox 约束；
- `TurnFinalizer`：保存 session、token、memory sync、事件与最终 reply。

这不是 deep 分支 `ToolExecutionGateway` 的回迁。第一阶段只把当前真实执行路径收口到一个函数/模块；Sandbox、Policy 和 Registry 仍是现有权威。

### 3.2 Manager 边界

保留现有 Axum router、路径和 DTO 兼容。按域拆为：

- `handlers/chat.rs`、`session.rs`、`config.rs`、`skills.rs`、`mcp.rs`、`cron.rs`；
- `api_error.rs`：统一 transport error 映射；
- `services/*`：仅在 handler 目前含有真实业务编排时提取；
- `runtime/bootstrap.rs`：唯一 composition root；`Manager` 继续作为兼容 façade。

Handler 只做反序列化、调用、错误映射和响应序列化；不直接拼装 AgentLoop、打开 store 或推导 domain terminal state。

### 3.3 GUI / Host 边界

前端只从一个 barrel 导入 API，但 barrel 后面按 domain 分文件。迁移期允许 `desktop.ts` 作为兼容 re-export，不要求一次改完所有 import。

能力分为：

- `MANAGER`：chat、session、plan、provider、tools、skills、MCP、cron、Laputa、AutoDream、stats、audit；
- `LOCAL`：window、tray、splash、GUI prefs、pet assets、本地日志、gateway 进程生命周期；
- `DEFERRED`：尚无真实后端能力，调用必须抛出明确错误或隐藏入口；
- `REMOVED`：死路径和重复协议，禁止假成功。

当前阶段继续使用 Tauri 的 Manager 代理以保持端口发现、桌面打包和兼容性；先拆域和去重复，再单独评估前端直连 loopback Manager。不会为了追随 deep 分支强行引入 `/v1`。

## 4. 可复用与拒绝移植清单

| deep 设计 | 本方案裁决 | 适配方式 |
| --- | --- | --- |
| 单一 authority spine | 保留原则 | 收口现有工具执行 seam，不新建 Gateway |
| Manager command/query 分离 | 保留语义 | handler 写/读职责分开，不换现有 `/api` |
| GUI `BOUND/LOCAL/DEFER/CUT` | 保留 | 形成当前能力清单与 fail-closed adapter |
| Tauri Host local-only | 渐进采用 | 先分类、再迁移业务代理，保留嵌入式启动 |
| server-owned domain state | 保留 | UI mutation 后重取权威 projection |
| revision/CAS approval | 保留 | 复用当前 Plan revision，不另建 store |
| clean-break 删除旧树 | 拒绝 | 零大爆炸替换 |
| 新 kernel/runtime/state crates | 拒绝 | 当前 crate 内建 seam |
| `/v1` 四路由替换全部 `/api` | 拒绝 | API 兼容优先 |
| format-v7 与离线迁移 | 拒绝 | 不改变当前存储格式 |
| 简化单轮 model complete | 拒绝 | 现有多轮 tool loop 是必须保留的产品能力 |

## 5. 架构不变量

1. 同一用户请求只进入一个 AgentLoop turn，不允许 GUI、Manager 或 Tauri 各自实现 agent 逻辑。
2. 工具副作用仍必须经过现有 Registry、Plan/Mask policy 与 Sandbox；重构不得新开直达工具路径。
3. Manager 是 transport + lifecycle + service composition，不是第二份 domain store。
4. GUI 的 terminal domain state 必须来自 Manager/session/plan 投影；本地状态只负责显示和 in-flight 体验。
5. Tauri 的 LOCAL 能力不得承载 Plan、Policy、Session 或 Memory 业务规则。
6. 每个阶段必须先有 characterization tests，再移动代码。
7. 每个阶段可独立回滚，禁止双写和长期 feature flag 双轨。

## 6. deep 分支证据处置

以下路径均指 `refactor/deep-governance@95dd9833`，不是当前主线事实：

| 来源 | 提取结论 | 处置 |
| --- | --- | --- |
| `docs/architecture/clean-break-rewrite.md` | authority source 数量比 LOC 更重要 | 保留度量原则；拒绝 destructive reset |
| `docs/architecture/clean-break-r2-execution-gateway.md` | effectful call 应有唯一入口、denial/idempotency/unknown outcome | 保留验收语义；不移植 Gateway crate |
| `docs/architecture/clean-break-r3-authority-spine.md` | Plan approval 不替代逐 action policy | 保留 |
| `docs/architecture/clean-break-r6-manager-host.md` | Manager 不应拥有 domain state 和 tool execution | 适配到当前 Manager façade |
| `docs/architecture/clean-break-r6-api.md` | command/query 与 committed projection 分离 | 采用内部职责分离；拒绝强切 `/v1` |
| `docs/architecture/clean-break-r6-gui.md` | GUI 不本地伪造 terminal fact | 保留 |
| `docs/architecture/gui-host-boundary.md` | Tauri LOCAL 与产品业务分界 | 渐进采用 |
| `docs/research/deep-governance-reintegration-2026-07/summary.md` | deep 仅为 experimental spine，缺多轮 loop/tool_calls/sandbox/真实 GUI | 作为失败判定的主要证据 |
| `docs/research/deep-governance-reintegration-2026-07/gui-api-governance-inventory.md` | capability 分桶和假成功禁令 | 适配到当前 `/api` 与 invoke surface |
| `docs/research/deep-governance-reintegration-2026-07/hitl-spine-s0.md` | HITL 是 interaction surface，不是第二权限链 | 保留原则；实现另行授权 |

明确不复制 deep 分支的完成声明、工期结论、format-v7、删除证明或 `0.6.0-experimental` 发布状态。
