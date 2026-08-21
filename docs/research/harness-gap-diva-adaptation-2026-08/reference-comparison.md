# 参考项目比较：只抽取机制，不复制产品

## 1. 能力级比较

| 参考 | 真实机制 | Diva 可吸收的原则 | Diva 明确不照搬 |
| --- | --- | --- | --- |
| OpenHarness (`bf5931e`) | `HookEvent` 事件枚举；`HookResult` 可阻断；Command/HTTP/Prompt/Agent 多种 Hook；PermissionMode；插件贡献 skills/commands/tools/hooks/MCP | 事件命名、阻断结果、matcher/timeout/blocked reason 的可观测性 | 任意 shell/HTTP/LLM Hook、Python 动态插件、简单 permission mode 替代 Sandbox |
| ZeroClaw (`d91e08eae`) | `HookHandler` 把 modifying hook 按 priority 串行，把 void hook 异步观察；`SessionActorQueue` 每 session semaphore + FIFO 深度/超时/TTL；stall watchdog；Extism/WASM host 权限 | 修改型与观察型分流、确定性优先级、session 有界准入、超时与 stall 观测 | WASM、宿主 HTTP、插件权限清单和它的 provider/channel 生态 |
| OpenFang (`acf2587`) | typed `EventId`/`EventTarget`/`EventPayload`；Approval 有风险级别、字段长度和 10–300s timeout；工具 schema provider normalization | 事件 envelope、target/payload 分层、边界字段长度、风险/超时的显式类型 | “Agent OS” 全局事件系统、跨 agent broadcast、整套 schema 兼容层 |
| Claude Code (`7beeb9c6`) | terminal reason / continuation reason；token budget 90% continuation + diminishing returns；stop hooks；daemon worker 以 spawn mode/capacity/timeout 管理远程 session | 把继续/停止原因结构化，把 budget/timeout 作为 runtime 状态而非文案；后台 worker 需要 capacity | Claude 专有 CLI、worktree/remote control、内存提取/AutoDream 行为 |
| GenericAgent (`ee5a474`) | 轻量工具、working anchor、任务后结晶、Prompt 约束与硬编码节奏并存 | 只在“密度/活动锚/下一轮减负”层面作研究参照 | L0–L4 文件布局、任意 file patch 权威、把 Evolution 当通用 Harness |

## 2. 关键推论

### 2.1 Hook 的核心不是“多十个事件”，而是修改权限

OpenHarness 有很多 Hook 类型，但它允许外部命令、HTTP 和二次 LLM 判断，导致 hook 本身
成为新的输入/网络/提示词攻击面。ZeroClaw 的分流更适合 Diva：

- **Observe**：只读、可并行、失败告警，不改变本轮请求；
- **Guard**：按优先级串行，可 `Continue` / `Replace` / `Block`，默认无外部 I/O，
  超时或错误按 hook policy 处理并记审计。

### 2.2 Session queue 是独立于 Bus 的运行时约束

ZeroClaw 的 `SessionActorQueue` 只负责同一 session 的串行访问、排队深度和 timeout，
不同 session 仍可并行。这个边界与 Diva 已有 `MessageBus` 兼容：

```text
MessageBus (transport)
  -> SessionAdmission (per-session bounded FIFO)
     -> AgentLoop::admit_turn
        -> planning / sandbox / provider
```

不要为了获得队列而把全局 Bus 改成 actor runtime；那会把渠道、计划、审批和会话状态
全部耦合。

### 2.3 Typed event envelope 值得借鉴，全球广播不值得

OpenFang 的 `EventTarget`/`EventPayload` 能让事件带身份、目标和类型，不必依赖字符串
拼接。Diva 可以为 Hook context 增加 `trace_id/session_key/phase`，但继续保留当前
`AgentBusEvent { channel, chat_id, event }` 的外部事件合同。新增 envelope 只服务于
内部 Hook，不向 GUI 一次性暴露整个 OpenFang event universe。

### 2.4 “继续/停止”要成为证据，而不是模型文案

Claude Code 的 terminal/continue reason 和 token budget decision 值得借鉴。Diva 已有
预算、熔断、取消、stream stall 等分散状态；Hook/Session 研究应把 admission result、
queue full、timeout、guard block、provider retry 等原因结构化发到 Audit/EventBus，
而不是只打印一行 log。
