# Diva 化适配方案

## 1. 目标

建立一个小而确定的 Harness seam，让 Diva 可以在不碰 BML、Persona、Laputa 权威和
Sandbox 语义的情况下，挂载审计、观测、限流和运行时护栏。

方案名称暂定 **Diva Runtime Hooks + Session Admission**，不是“OpenHarness 插件系统”
或“Claude Code 兼容层”。

## 2. 设计不变量

1. **安全权威不下沉到 Hook**：危险执行仍只由 `ToolStepPolicy`、Sandbox、Approval、
   Guardian 决定；Hook 不能发放授权。
2. **领域写入隔离**：Hook 不得直接调用 BML write、Persona apply、Evolution apply、
   ACTMEM 写入或治理 ledger；需要改变领域状态的行为必须走既有服务接口。
3. **默认只观察**：注册一个 Hook 不改变模型输入、工具参数或 provider model ID；只有
   明确标为 Guard 的实现才可返回 Replace/Block。
4. **可预测**：Guard 按 priority + registration order 串行；Observe 可并行但有上限；
   每次执行都有 timeout、trace/session/tool correlation。
5. **失败可解释**：Hook error、timeout、block、queue full、queue wait timeout 都是
   typed outcome，并进入审计/事件，不伪装成普通模型错误。
6. **跨入口一致**：CLI、GUI、Channel、后台任务都经过同一个 runtime seam；不为 GUI
   单独复制一套 hook。

## 3. MVP 形状（只做设计，不写代码）

### 3.1 Hook domain

建议落点：`agent-diva-core/src/bus/hooks.rs`（不新建独立 crate）。

建议类型：

```text
HookEvent
  SessionStarted | SessionEnded
  TurnAdmitted | TurnRejected
  BeforeProviderCall | AfterProviderCall
  BeforeToolCall | AfterToolCall
  BeforeCompaction | AfterCompaction
  ApprovalRequired | MessageSent

HookKind = Observe | Guard
HookOutcome<T> = Continue(T) | Replace(T) | Block { reason, code }
HookContext = { trace_id, session_key, channel, chat_id, phase, tool_name, call_id }
```

实现接口的约束：

- `HookHandler: Send + Sync`，显式 `name()`、`priority()`、`timeout()`；
- registry 显式注入到 AgentLoop/Manager，不做全局隐式 singleton；
- MVP 只允许 Rust 内部 handler；不允许 command/HTTP/prompt hook；
- Guard 默认只能检查已归一化的 metadata/参数摘要，完整秘密、原始 credential 和
  未脱敏用户内容不直接传给 hook；
- Observe hook 的失败不影响主流程；Guard hook 的失败默认 fail-closed，只有在注册时
  明确 `fail_open` 且事件属于非安全观测面才可继续；
- hook 不能递归触发同一事件，避免自激循环。

第一批内置 handler 只做三类：

1. 审计 correlation：把 `trace_id/session_key/tool_call_id` 补到既有 audit；
2. metrics/latency：记录 provider/tool/queue 时延和 outcome；
3. policy probe：只读地验证计划 phase、取消状态、session admission snapshot，不能
   代替现有 policy。

### 3.2 Session admission domain

建议落点：`agent-diva-core/src/session/admission.rs`，在
`agent-diva-agent/src/agent_loop/turn/admission.rs` 前后接线。

最小状态：`Idle -> Running -> (Idle | Cancelled)`，等待中的请求处于 `Queued`。每个
`session_key`：

- FIFO；
- `max_queue_depth` 默认小值（建议先从 2 开始，以测试/观测再调）；
- `wait_timeout` 有界；
- queue full、timeout、cancel、evict 都有稳定错误码；
- 不同 session 仍可并行；
- session idle TTL 只清理 admission slot，不删除会话历史或 Memory；
- `StopSession` 只取消当前/指定请求，不吞掉队列中其他请求的可观察状态。

接线顺序：

```text
inbound message
  -> admission.try_enqueue(session_key)
  -> acquire guard
  -> existing enforce_turn_admission (circuit + hour rate)
  -> existing Plan/Sandbox/Provider flow
  -> release guard + emit outcome
```

注意：小时/天 token “queue branch”仍是另一项产品决策；本方案只处理同 session 的
并发串行和背压，不把 token budget 熔断改成等待队列。

## 4. 分阶段实施计划

### Phase 0：表征与冻结（文档/测试）

- 为现有 Plan capability matrix、tool seam、Poke 8、provider stream、approval
  coordinator 增加“当前合同”表征测试；
- 定义 HookContext 脱敏字段和 outcome/error code；
- 明确每个事件属于 Observe 还是 Guard，禁止未分类事件进入 registry。

### Phase 1：Hook Kernel（core + agent）

- 新增显式 `HookRegistry` 和顺序/超时执行器；
- 先接 `BeforeToolCall/AfterToolCall/TurnAdmitted/TurnRejected`；
- 只注册三类内置 observe/probe handler；
- 负向测试：hook 不能绕过 Plan、不能把未知 tool 变成允许、不能递归发布自身事件。

### Phase 2：Session Admission（core + agent + manager）

- 实现 per-session bounded FIFO/timeout/idle eviction；
- 在 Manager API、Channel 入站、后台任务入口统一接入；
- 增加 queue depth、wait latency、full/timeout/cancel 的事件和 CLI/GUI 可观察 DTO；
- 负向测试：同 session 不重入，不同 session 不互相阻塞，queue full 不触发模型调用。

### Phase 3：真实入口验收

- CLI：并发发送同一 session 的两条消息，观察 FIFO 和超时；
- GUI：运行中 Stop/Reset 与排队消息，确认状态不串 session；
- Channel：至少一个实时通道验证背压提示；
- Provider：retry/stall 与 queue outcome 同时出现时，事件链可追踪。

### Phase 4：再评估扩展（不承诺）

- 若 Rust Hook Kernel 的边界稳定，再研究受限插件；
- 只有完成 threat model、签名/来源、权限和 sandbox 评审后，才讨论 WASM/HTTP/脚本；
- ACP、remote control、worktree、toolset bundle 不属于本研究包的自动后续。

## 5. 验收与停止条件

必须满足：

- `PlanModeState` 和现有 Sandbox/Approval 测试全部保持通过；
- 同 session 无并发执行，queue full/timeout 可确定复现；
- Hook 失败不会丢失用户消息，也不会静默放行安全 Guard；
- audit 能用 trace_id/session_key/tool_call_id 重建一次 turn 的 admit → provider →
  tool → outcome 链；
- 无新增 BML/Persona/Evolution 写路径；
- 无新全局广播事件面、无任意外部脚本执行面。

若 Phase 1 需要把 Hook 直接塞进 `AgentEvent` 变体、或 Phase 2 需要重写 MessageBus，
应停止并重新评审边界；这代表方案正在复制参考项目骨架。
