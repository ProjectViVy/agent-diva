# 当前 Diva 状态与证据

## 1. Plan Mode 已是运行时策略，不是提示词建议

`agent-diva-core/src/planning/policy.rs` 已定义：

- `PlanModeState::{Exploring,Drafting,AwaitingApproval,Executing,Verifying,Closed}`；
- `ToolCapability::{Inspect,PlanningRecord,WorkItem,WorkspaceWrite,Execute,External,Unknown}`；
- `allows()` 的 fail-closed 矩阵：未知能力在任何状态都拒绝；Explore/Plan 只允许检查和
  记录；AwaitingApproval 只允许检查；Execute 才打开写入/执行/外部能力；Verify 收窄为
  检查、记录和执行。

`agent-diva-agent/src/agent_loop/turn/tool_step.rs` 在单一工具执行入口通过
`denial_reason()` 再次检查 phase、plan guard、是否存在持久计划和 reviewer read-only。
这形成“状态投影 + 执行 seam 双重门”，应作为后续 Hook 设计的样板，而不是被新的
permission mode 覆盖。

## 2. Bus 有事件广播，但没有 Trait Hook

`agent-diva-core/src/bus/queue.rs` 当前提供：

- `publish_event()` / `subscribe_events()`：`broadcast::Sender<AgentBusEvent>`；
- `publish_poke_event()` / `subscribe_poke_events()`：独立的 `broadcast::Sender<PokeEvent>`；
- inbound/outbound 仍是 `mpsc::UnboundedSender`。

`PokeEvent` 已覆盖 PokeSend、ChatSend、ChatSent、ChatReceived、ReasoningReceived、
ChatOver、ChatHistoryAdd、TokenUsed，以及配置需重启提示。这是事件事实面，不等于
“某个模块可以在工具前阻断或改写请求”。当前缺的是：注册、优先级、同步/异步语义、
超时、阻断结果和 hook 失败的审计分类。

## 3. Stream 与成本观测已经存在

`agent-diva-providers/src/base.rs` 的 `LLMStreamEvent` 有 TextDelta、ReasoningDelta、
ToolCallDelta、Completed。`agent-diva-agent/src/agent_loop/turn/iteration.rs` 已消费
这些事件、处理取消和内部协议泄漏保护，并发布对应 `AgentEvent`。Manager 的
`runtime.rs` 也消费 Provider stream；`loop_turn.rs` 将 provider usage 写入
`JsonlTokenLedger`，并保留 cache usage 字段。

因此本阶段不做“重建流式引擎/成本追踪”；只研究 Hook 如何观察这些既有事件，不能复制
参考项目的整套 stream API。

## 4. Sandbox / Approval 是 Diva 的既有边界

`agent-diva-sandbox/src/approval_coordinator.rs` 已有 `CommandApprovalCoordinator`、
pending/resolved 状态、超时、Once/Session/Global 授权适配和审计相关联；Guardian、
command rules、filesystem policy 和平台沙箱分在独立模块。

OpenHarness 的 `PermissionMode::{default,plan,full_auto}` 可用于解释“用户看得懂的
模式”，不能代替 Diva 的危险命令审批和 Guardian。新 Hook 不能绕过
`ToolStepPolicy`、Sandbox 或 Approval Coordinator。

## 5. Session admission 的现状是拒绝式，不是排队式

`agent-diva-agent/src/agent_loop/turn/admission.rs` 的 `enforce_turn_admission()` 只在
进入 turn 前检查 rejection circuit 和 `max_actions_per_hour`。它会拒绝新 turn，但
没有 per-session FIFO、队列深度上限、等待超时、重复请求合并或队列状态事件。

`MessageBus` 的 inbound/outbound channel 是 unbounded；这解决了 transport 解耦，却不
能单独证明某个 session 不会无限积压。ZeroClaw 的 SessionActorQueue 说明了可移植的
问题形状，但 Diva 需要把它接在 turn admission，不应替换全局 Bus。

## 6. 旧报告需要废止的判断

6 月三方报告仍可作为线索，但以下判断已被当前代码取代：

- “Diva 没有 Plan Mode 物理限制”——不再成立；
- “Diva 没有 Prompt Injection/PII/Tool Result 防护”——不再成立，当前有
  `security/injection.rs`、`security/pii.rs`、`tool_result_filter.rs` 等模块；
- “Diva 的 stream 只在 provider 层存在”——不再成立，agent loop 已消费 stream；
- “Approval 只有 enum”——不再成立，Sandbox 有协调器与持久治理适配。

后续设计必须引用当前 crate 路径和测试，而不是引用旧完成度百分比。
