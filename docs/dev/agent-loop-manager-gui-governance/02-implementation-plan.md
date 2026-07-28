# 实施方案

> 本文是实施设计，不代表已批准修改运行时代码。

## 1. 总体策略

采用六个可独立验收的切片。每个切片只在现有 crate 内移动职责，保留外部行为；前一切片没有证据通过，不开始后一切片。

## 2. Phase 0：行为冻结与依赖地图

目标：先证明当前路径，而不是先拆文件。

- 为 chat 普通回复、流式 tool call、stop、Plan 等待审批/批准恢复、session 保存、provider 错误和 context overflow 建立 characterization tests。
- 记录从 [chat_handler](../../../agent-diva-manager/src/handlers.rs:82) 到 [process_inbound_message_inner](../../../agent-diva-agent/src/agent_loop/loop_turn.rs:294) 再到 GUI stream reconciliation 的事件顺序。
- 建立 GUI capability ledger，逐项标记 `MANAGER / LOCAL / DEFERRED / REMOVED`。
- 记录基线：目标文件 LOC、最长函数、crate 依赖和关键旅程耗时。

退出条件：测试能在不读取实现细节的情况下证明当前主旅程与失败语义。

## 3. Phase 1：Agent Loop 收口

### 新增内部文件

| 文件 | 职责 | 估算 |
| --- | --- | ---: |
| `agent-diva-agent/src/agent_loop/turn/mod.rs` | turn 内部入口与共享类型 | 150 行 |
| `turn/admission.rs` | 输入、session、media、budget 预检 | 250 行 |
| `turn/snapshot.rs` | mask/model/plan/tool capability 一次性快照 | 250 行 |
| `turn/context.rs` | memory/context/prompt 装配 | 350 行 |
| `turn/model_step.rs` | provider streaming 与迭代控制 | 450 行 |
| `turn/tool_step.rs` | policy 检查、Registry 执行、tool 事件 | 450 行 |
| `turn/finalize.rs` | token/session/memory/event/final reply | 350 行 |

### 修改文件

- [agent_loop.rs](../../../agent-diva-agent/src/agent_loop.rs:123)：保留 façade；把依赖分组为不可变 services、runtime state、settings，构造 API 暂不破坏。
- [loop_turn.rs](../../../agent-diva-agent/src/agent_loop/loop_turn.rs:294)：逐阶段委托，最终缩为薄编排与兼容测试。
- [loop_runtime_control.rs](../../../agent-diva-agent/src/agent_loop/loop_runtime_control.rs:11)：runtime command 只改变明确状态；phase 变化统一请求 snapshot/tool refresh。
- [tool_assembly.rs](../../../agent-diva-agent/src/tool_assembly.rs:1)：保留工具装配，不承担 turn 生命周期。

### 关键顺序

1. 提取纯 helper 和 DTO，不改变 borrow/async 边界。
2. 引入 `TurnSnapshot`，统一 turn 起点、tool 后和 runtime control 后的 policy phase 推导。
3. 把所有 `ToolRegistry::execute` 的 turn 内调用收口到 `tool_step`。
4. 把事件生成与已提交事实分开；事件失败不得反向伪造 domain 状态。
5. 最后拆 provider iteration 和 finalization。

## 4. Phase 2：Manager 薄化

### 文件布局

将 [handlers.rs](../../../agent-diva-manager/src/handlers.rs:1) 改为 module index，并继续沿用当前已存在的 domain handler 结构：

```text
handlers/
  chat.rs
  session.rs
  config.rs
  skills.rs
  mcp.rs
  files.rs
  cron.rs
  events.rs
  ...existing domain modules
```

同步将 [state.rs](../../../agent-diva-manager/src/state.rs:60) 拆为：

```text
state.rs                 # AppState only
api/chat.rs
api/session.rs
api/config.rs
api/provider.rs
api/skills.rs
api/mcp.rs
api/cron.rs
api/error.rs
```

实现规则：

- 保持 [server.rs](../../../agent-diva-manager/src/server.rs:1) 的现有 route 和 method；
- `Manager` 保持公开 façade，内部逐步委托 domain service；
- chat/session handler 不直接操作 provider、store 或 plan 状态；
- `serde_json::Value` 只允许用于真正开放的 JSON payload 或兼容边界，新代码优先 typed DTO；
- SSE 映射集中在 `handlers/events.rs`，不在 chat handler 复制。

## 5. Phase 3：GUI API 与 Host 治理

### 前端 API

建立：

```text
src/api/index.ts
src/api/domains/chat.ts
src/api/domains/session.ts
src/api/domains/plan.ts
src/api/domains/config.ts
src/api/domains/provider.ts
src/api/domains/skills.ts
src/api/domains/mcp.ts
src/api/domains/cron.ts
src/api/domains/evolution.ts
src/api/domains/host.ts
src/api/domains/pet.ts
src/api/capabilities.ts
```

[desktop.ts](../../../agent-diva-gui/src/api/desktop.ts:131) 在迁移期仅 re-export；组件禁止新增直接 `invoke`。每个 domain adapter 明确自己的 capability class 和 transport。

### Tauri Host

把 [commands.rs](../../../agent-diva-gui/src-tauri/src/commands.rs:253) 分为：

- `commands/local/*`：窗口、prefs、pet、logs、wipe、gateway lifecycle；
- `commands/manager/*`：当前 `/api` 的薄代理与 event bridge；
- `dto/*`：仅 Host 独有 DTO；Manager DTO 不在多个 command 模块重复定义。

第一阶段不把前端改成直连 HTTP；这样可保留 [AgentState](../../../agent-diva-gui/src-tauri/src/app_state.rs:7) 的动态端口与 release/debug 行为。只有在 CORS、CSP、认证、端口发现和打包 smoke 均有方案后，才可另立故事评估直连。

## 6. Phase 4：GUI 状态与组件拆分

从 [App.vue](../../../agent-diva-gui/src/App.vue:192) 提取：

- `useChatSession`：session list/cache/load/delete/title；
- `useChatStream`：placeholder、tool event、cancel、final reconciliation；
- `usePlanProjection`：server projection、approval in-flight、refresh；
- `useProviderConfiguration`：model/provider config 映射；
- `useGatewayHealth`：startup/health/recovery；
- `useAppNavigation`：纯 UI 选择。

规则：

- composable 不渲染，component 不访问 Tauri；
- approve/deny/execute 后必须重新获取 Manager projection；
- 本地可保存 selection、draft、loading，不可自行把 plan 标成 completed/failed；
- `ChatView.vue` 只接收渲染模型和发出 intent，不再拥有第二套业务状态机。

## 7. Phase 5：协议去重

先做契约测试，不立即引入代码生成依赖：

1. Rust Manager DTO 输出稳定 JSON fixture；
2. GUI 用 TypeScript schema/guard 读取同一 fixture；
3. CI 检查 required fields、enum 和 snake/camel 映射；
4. 证据稳定后，再 ADR 评估 `specta`、`ts-rs` 或 JSON Schema codegen。

优先处理已确认重复：

- `PlanRuntimeState` / `PlanApprovalResult`；
- `SkillDto` / `FileAttachmentDto`；
- `McpServerDto` / `McpConnectionStatusDto`；
- provider catalog/test payload；
- `RuntimeConfigSnapshot` 与 Mentle tool response。

## 8. Phase 6：清理与门禁

- 删除已无调用的兼容 re-export、重复 DTO 和 dead command；
- 增加结构门禁：组件内无新增 `invoke`，业务 domain 不进入 LOCAL command；
- 更新架构索引和 owner；
- 只有所有主旅程 smoke 通过后，才删除旧文件壳。

## 9. 明确不做

- 不创建新分支/工作树作为实现前提；
- 不回迁 deep 的 crate；
- 不把当前 API 一次性替换为 `/v1`；
- 不改变配置和持久化 schema；
- 不以“文件拆小”替代责任收敛；
- 不在同一阶段同时重写 AgentLoop、Manager 和 GUI。
