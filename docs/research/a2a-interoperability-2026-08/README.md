# A2A Agent-to-Agent 互操作研究包

> 日期：2026-08-23
> 状态：Research / Proposal，待正式立项，未授权生产实现
> 范围：评估 `.workspace` 参考项目中的 A2A、内部多智能体、任务生命周期与远程控制模式，形成 agent-diva 的多种适配路径。

## 结论先行

建议将 A2A 定位为 agent-diva 的北向智能体互操作层，而不是替换现有
`AgentLoop`、`MessageBus`、`SubagentManager`、Sandbox、Approval、Laputa 或 BML。

推荐路线是：

1. 先抽象 agent-diva 内部统一的 `AgentProfile`、`AgentRun`、`TaskStore`、`Policy`
   和 `Artifact` 生命周期；
2. 在 `agent-diva-manager` 内增加原生 A2A adapter；
3. 短期如需验证跨项目互操作，可先用独立 Sidecar；
4. 借鉴 OpenFang 的 Rust A2A 路由/客户端、ZeroClaw 的 Alias/技能白名单，
   但不整体引入任一项目作为 agent-diva 核心运行时。

## 1. 协议基线

当前应以 A2A v1.0 作为新实现基线。官方规范定义了 Agent Card、Message、Task、
Artifact、异步任务、流式更新、任务查询和取消等基础模型，并支持 HTTP+JSON、JSON-RPC
和 gRPC 等绑定。参考：[A2A v1.0 规范](https://github.com/a2aproject/A2A/blob/main/docs/specification.md)
和[A2A v1.0 发布说明](https://a2a-protocol.org/latest/announcing-1.0/)。

推荐的标准面：

- Discovery：`GET /.well-known/agent-card.json`；Agent Card 使用
  `supportedInterfaces` 声明接口、协议绑定和版本。
- HTTP+JSON：`POST /message:send`、`POST /message:stream`、
  `GET /tasks/{id}`、`POST /tasks/{id}:cancel`。
- JSON-RPC v1.0：`SendMessage`、`SendStreamingMessage`、`GetTask`、
  `ListTasks`、`CancelTask`。
- 长任务可使用 SSE、轮询或 Webhook 推送；客户端凭据通过 HTTP 层传递，
  不嵌入消息载荷。

OpenFang 的 `tasks/send` 和 ZeroClaw 的 `message/send` 应视为兼容/历史接口，不能
直接作为 agent-diva 的唯一规范面。agent-diva 可以在兼容层同时接受它们，但内部必须
归一到同一个 Task 模型。

## 2. 本地参考项目比较

本次以 `.workspace` 本地快照为准：OpenFang `acf2587`，ZeroClaw `d91e08eae`。
本地扫描中真正包含 A2A 实现的主要是这两个项目，其余项目提供的是内部调度或控制面参考。

| 项目 | 可吸收的设计 | 不应直接照搬的部分 |
| --- | --- | --- |
| OpenFang | Rust A2A Agent Card、外部发现、Client/Server、任务查询/取消、路由组织 | 内存 TaskStore、同步执行、简单首 Agent 路由、取消未必停止底层运行、旧方法名 |
| ZeroClaw | `supportedInterfaces`、多 Alias、技能白名单、默认关闭、`public_base_url` | 入站任务缺乏完整持久化生命周期；A2A 任务入口不能无条件绕过鉴权 |
| OpenAkita | Profile、Agent Pool、TaskQueue、并发限制、空闲回收、单跳委托、控制面审批 | Python 内部组织模型，不是网络互操作协议 |
| OpenHarness | Team Registry、后台任务、生命周期、权限、Hooks、Session Resume、Dry-run | 没有 A2A wire contract |
| Pi | 子进程隔离、并发子 Agent、流式输出、用量跟踪、Abort | 更适合本地执行器，不等价于远程 A2A |
| Codex App Server | 双向 JSON-RPC、Thread/Turn/Item、事件通知、审批、认证、背压 | 面向客户端控制面，不是 Agent-to-Agent 语义 |
| Claude ACP Link | WebSocket + 子进程、Session、认证、Heartbeat、远程管理 | ACP 与 A2A 是不同协议，应通过 adapter 互通 |

证据路径：

- `.workspace/openfang/crates/openfang-runtime/src/a2a.rs`
- `.workspace/openfang/crates/openfang-api/src/routes.rs`
- `.workspace/zeroclaw/crates/zeroclaw-gateway/src/a2a.rs`
- `.workspace/zeroclaw/crates/zeroclaw-config/src/multi_agent.rs`
- `.workspace/openakita/src/openakita/agents/{orchestrator.py,task_queue.py,factory.py}`
- `.workspace/OpenHarness/README.zh-CN.md`
- `.workspace/pi/packages/coding-agent/examples/extensions/subagent/README.md`
- `.workspace/codex/codex-rs/app-server/README.md`
- `.workspace/claude-code/packages/acp-link/README.md`

## 3. agent-diva 当前适配基础与缺口

### 已有基础

- `agent-diva-manager` 已有 Axum Gateway、`/api/chat`、SSE 和停止接口。
- `MessageBus` 已有 `InboundMessage`、`AgentBusEvent` 和 session 级事件过滤。
- `AgentLoop` 已有流式输出、工具调用、终态和错误事件。
- `SubagentManager` 已有后台任务、批量执行、并发上限和运行任务可见性。
- Manager 已有受监督的 RunStore/Executor 路径。
- Sandbox、Approval、Laputa/BML 可继续作为所有远程调用的治理边界。

### 主要缺口

- 当前主要是单一默认 Agent，缺少 Agent Profile、Alias 和能力路由。
- 没有标准 Agent Card、外部 Agent 配置和 Agent Card 缓存。
- 没有持久化 A2A Task/Artifact 模型。
- Local Gateway 默认路径绑定 `127.0.0.1`，没有 A2A 专用公开监听与反向代理配置。
- 任务取消尚未映射到可靠的底层 CancellationToken/运行实例。
- 没有面向远程主体的技能白名单、预算、鉴权主体和工具策略。
- 内部 AgentEvent 不能直接等价暴露为 A2A StreamResponse。

A2A 请求必须进入现有 `ManagerCommand::Chat` / `AgentLoop` / MessageBus 流程，不能
直接调用 BML 写 API，也不能绕过 Sandbox、Approval 或 Laputa 治理。

## 4. 多种适配方案

### 方案 A：`agent-diva-manager` 原生 A2A

在 Manager 内增加 `a2a` 模块，直接提供 Agent Card、Send/Get/Cancel/Stream，并把任务
映射到现有 AgentLoop。

- 优点：长期架构最干净，审计、审批、Session、SSE 可共用。
- 缺点：需要自行完成协议版本、持久化、取消、推送和兼容处理。
- 适用：正式产品路线。

### 方案 B：独立 `agent-diva-a2a-bridge` Sidecar

Sidecar 对外提供 A2A，对内调用现有 `/api/chat` 和 `/api/chat/stop`。

- 优点：最快完成 POC，协议变化与核心运行时隔离。
- 缺点：Task、Run、Session、取消和错误容易形成两套语义。
- 适用：第一阶段互操作验证，不建议长期作为唯一架构。

### 方案 C：OpenFang 作为 A2A 前门

用 OpenFang 接收 A2A，再将任务转给 agent-diva。

- 优点：最快得到可工作的 A2A Server/Client 试验环境。
- 缺点：增加第二个 Runtime，容易绕过 agent-diva 的 BML、Laputa 和 Sandbox 治理。
- 适用：实验环境和兼容性测试，不建议正式落地。

### 方案 D：ZeroClaw 风格的多 Alias 暴露

在 agent-diva 内部实现多 Agent Alias、技能白名单、默认关闭和反向代理 URL 管理。

- 优点：适合未来 Persona/Profile 多 Agent 化，最小暴露面清晰。
- 缺点：仍需补齐客户端、持久化 Task、取消和完整鉴权。
- 适用：作为原生 A2A 的配置与安全模型，而不是代码整体迁移。

### 方案 E：内部统一任务模型，再挂 A2A adapter（推荐）

先形成统一的 `AgentProfile`、`AgentRouter`、`AgentRun`、`TaskStore`、`Policy`、
`ArtifactStore` 和 `EventStream`，然后让 GUI、CLI、Subagent、A2A、未来 ACP 都成为
适配器。

- 优点：A2A 不会绑架内部架构，可复用到 CLI、GUI、ACP 和其他远程协议。
- 缺点：前期抽象成本最高，需要先确定 AgentRun/Task 边界。
- 适用：agent-diva 的长期架构。

## 5. 推荐目标结构

```text
A2A HTTP / JSON-RPC
        ↓
Manager A2A Adapter
        ↓
鉴权 / AgentPolicy / Skill Allowlist / Budget
        ↓
持久化 A2A TaskStore / AgentRun
        ↓
AgentProfile Router
        ↓
ManagerCommand::Chat → AgentLoop → MessageBus
        ↓
Sandbox / Approval / Laputa / BML
        ↓
Task / Artifact / Stream / Push
```

建议映射：

| A2A | agent-diva |
| --- | --- |
| `taskId` | 持久化 AgentRun/A2A Task ID |
| `contextId` | `channel=a2a` 加唯一 `chat_id` |
| Message parts | `InboundMessage` 内容、附件和结构化输入 |
| `completed` | `AgentEvent::FinalResponse` |
| `failed` | `AgentEvent::Error` |
| Artifact | 最终文本、文件或结构化结果 |
| CancelTask | TaskStore 状态更新 + CancellationToken + StopChat |
| Streaming | `AssistantDelta` 转换为 A2A StreamResponse |

默认不向远程调用方暴露内部 reasoning、工具原始参数、BML 内容或完整审计细节。

## 6. 建议实施分期

### P0：协议和安全门

- 以 A2A v1.0 HTTP+JSON 为主，JSON-RPC 作为兼容绑定。
- 明确 `agent-card.json`、`supportedInterfaces`、鉴权和技能暴露合同。
- 建立 OpenFang/ZeroClaw 互操作测试样例。

### P1：单 Agent 最小闭环

- `agent-card.json`。
- `message:send`、Task Get、Task Cancel。
- API Key/Bearer 鉴权。
- 持久化 TaskStore。
- 默认只开放文本和低风险技能。

### P2：出站 A2A

- Agent Card 缓存、静态外部 Agent 配置和技能匹配。
- `delegate_to_remote_agent` 受控工具。
- URL allowlist、SSRF 防护、超时、重试、幂等和预算。

### P3：多 Agent 与长任务

- Agent Profile/Alias。
- Skill exposure、SSE Stream、`input-required`、Artifact。
- Webhook Push Notification、恢复和重启后的任务重建。

### P4：生产证明

- 协议 TCK/Inspector、鉴权失败测试、取消竞态测试、重启恢复测试。
- 并发、背压、成本、Artifact 大小和恶意远程输入测试。
- CLI/GUI 任务观察、审计和运维文档。

## 7. 风险与立项门槛

- A2A 默认关闭；未发布 Agent 不可被远程调用；空技能列表不应自动扩大权限。
- Agent Card 只是能力声明，不是权限系统；真实权限必须由 agent-diva Policy/Approval 决定。
- 出站 URL 必须防 SSRF，只允许显式配置的 HTTPS/内网地址。
- TaskStore 不能只放内存；重启后不能丢失任务幂等和终态。
- Cancel 必须尽力停止实际 AgentLoop，不能只修改外部状态。
- 远程 Agent 返回内容按不可信输入处理，不能直接触发高风险工具。
- 不直接引入 OpenFang/ZeroClaw 的内部运行时、插件市场或未审查的远程执行入口。

## 8. 正式立项前需要拍板

1. 是否采用“内部统一任务模型 + 原生 A2A adapter”作为主路线。
2. 第一版是否只支持单 Agent、文本、HTTP+JSON、API Key/Bearer。
3. 是否接受 A2A TaskStore 复用现有 RunStore，还是新增独立 SQLite 表。
4. 多 Agent Alias、远程出站委托、SSE/Webhook 是否拆成后续 Epic。
5. 对外开放范围、默认工具策略、预算和人工审批策略。

本研究包只完成 Research/Proposal，不授权生产实现。正式施工前应按 `LOCK.md` 新建独立
工作区和实现迭代日志。
