# CHANNEL-EPIC：Super Channel Fabric 架构设计

> 状态：C0 Architecture Frozen
> 日期：2026-08-30
> Epic：`CHANNEL-EPIC`
> 协议：`Neuro-Link v1`
> 参考基线：Octos `5ea987813de4fd2afdd1d78f2106ad2868f0d923`
>（`v2.0.3-rc.9`）
> 约束：Rust 2021、MSRV 1.80、Clean Break、无兼容层

## 1. 决策摘要

`CHANNEL-EPIC` 正式启动，并把此前分立的两个工作流统一到同一个架构：

- Neuro-Link 是 Owner Frontend 使用的**超级通道**，可以完整承载新的 DIVA 前端；
- Telegram、Discord、Feishu、DingTalk、Email、QQ 是面向外部平台的普通频道适配器；
- 两类入口共享消息合同、能力模型、有界队列、关联标识、可靠性和 TCK；
- 两类入口不共享信任语义：外部平台 sender 绝不能因接入统一 Fabric 而成为 Owner；
- 当前 Neuro-Link 长期处于测试阶段，v1 不实现身份、设备配对或权限限制；
- 无认证 Neuro-Link 只能复用 Manager 的固定 loopback listener，不提供远程监听配置；
- 现有 Manager 领域 HTTP handler 原位成为 Versioned Service Binding，不复制第二套 API；
- 旧 Neuro-Link pipe、旧频道 DTO、旧 `ChannelHandler` 和无界频道 bus 在最终切换时全部删除；
- 正式进入 `dev` 的产品树只允许新架构，不允许 shim、双写、双读、协议降级或兼容别名。

本设计采用 Octos 的 `Channel + ChannelManager + bounded bus` 结构作为频道适配主对照，
但不复制其默认 no-op、历史兼容链、Rust 2024/MSRV 1.85 依赖和 DingTalk Webhook 降级。
agent-diva 保留自己的 Session Admission、Manager、Sandbox、Approval、Laputa/BML 和
Workspace 权威边界。

## 2. 目标与非目标

### 2.1 目标

1. 为所有消息入口冻结同一套 typed envelope、内容块、能力和投递结果。
2. 为第一方桌面 GUI、未来 Web/手机前端提供完整双向前端协议。
3. 让流式回复、工具状态、审批、Ask User、Planning、Presentation 和状态恢复具有统一语义。
4. 把所有频道入站、出站和连接任务放入有界、可观测、可取消的运行时。
5. 让每个外部平台明确声明真实能力，并通过同一 TCK 验证。
6. 在不提高 MSRV、不改变领域权威的前提下完成原子迁移。

### 2.2 非目标

- v1 不提供 Neuro-Link 远程访问、身份认证、设备注册或多用户授权。
- 不把 QQ/群聊 sender 提升为 Owner Frontend。
- 不用一个万能 `metadata: JSON` 代替跨频道公共类型。
- 不把 BML、Laputa、Planning、Run、Session 或 Approval 数据复制为第二权威。
- 不新增队列容量、重试次数、能力开关或 Neuro-Link host/port/auth 配置。
- 不迁移六个已退役且默认不编译的频道；它们将在 Clean Break 中删除。
- 不直接引入 Octos workspace、配置系统、runtime、memory 或依赖版本。

## 3. 术语与边界

| 名称 | 定义 | 信任级别 | 主要职责 |
| --- | --- | --- | --- |
| Super Channel Fabric | 统一消息、路由、能力、流控和观测底座 | 中性 | 连接 Agent Runtime、前端和平台适配器 |
| Neuro-Link v1 | Owner Frontend 的超级通道协议 | 测试期本机 Owner 接口 | Conversation、Presentation、Control、State Sync |
| Channel Adapter | 外部平台协议适配器 | 不可信或有限信任 | 平台事件、投递、重连、限速和媒体 |
| Service Binding | Manager 既有领域 HTTP handler 的版本化登记 | 继承领域策略 | Persona、Memory、Planning、Workspace 等 CRUD |
| Fabric Kernel | 有界 ingress/egress、路由、优先级和 receipt | 内部可信 | 不拥有领域数据 |
| Projection Journal | 可重建的终态/幂等/游标投影 | 内部派生数据 | 断线恢复和重复命令抑制 |
| TCK | Technology Compatibility Kit | 测试合同 | 验证协议与适配器能力真实性 |

“超级通道”描述的是前端能力宽度，不代表更高的核心写权限。Neuro-Link 仍必须调用
Manager/Agent 的正式命令和领域服务，不能直写 Session 文件、RunStore、Planning SQLite、
Laputa ledger 或 BML。

## 4. 总体架构

```text
                       External Platforms
       Telegram Discord Feishu DingTalk Email QQ
            │      │      │       │      │   │
            └──────┴──────┴──┬────┴──────┴───┘
                             │ ChannelAdapter
                             ▼
                    ┌────────────────────┐
                    │ Super Channel      │
                    │ Fabric Kernel      │
                    │ bounded + typed    │
                    └─────────┬──────────┘
                              │ ChannelEnvelopeV1
                              ▼
                    ┌────────────────────┐
                    │ Agent / Manager    │
                    │ Session Admission  │
                    │ Policy / Approval  │
                    └──────┬───────┬─────┘
                           │       │
              domain HTTP │       │ Neuro-Link v1 / JSON-RPC 2.0
              bindings    │       │ WebSocket, loopback only
                           ▼       ▼
                    ┌────────────────────┐
                    │ Desktop GUI first │
                    │ future Web/Mobile │
                    └────────────────────┘
```

### 4.1 模块归属

- `agent-diva-core`
  - 保存跨运行时的消息、地址、内容块、关联标识、投递结果和错误代码；
  - 保留 Session Admission、Session、Run 和领域事件权威；
  - 不依赖具体平台 SDK 或 WebSocket transport。
- `agent-diva-channels`
  - 保存 `ChannelAdapter`、能力矩阵、Adapter Registry、共享 pacing/supervisor；
  - 保存六个现役平台实现；
  - 不再承载 Neuro-Link WebSocket server。
- `agent-diva-manager`
  - 装配 Fabric Kernel、Agent Runtime 和 Adapter Registry；
  - 在现有 loopback listener 暴露 Neuro-Link v1；
  - 投影 Agent/领域事件、维护混合 Projection Journal、登记 Service Bindings。
- `agent-diva-gui`
  - 作为首个 Neuro-Link v1 客户端和 TCK 产品客户端；
  - 不维护 Session、Approval、Planning 或 Memory 的第二真相源。

不为本 Epic 新增 workspace crate。公共类型进入 `agent-diva-core`，频道实现继续属于
`agent-diva-channels`，transport 和领域编排属于 `agent-diva-manager`，与 Octos 的
core/bus/server/channel 分层保持同构，同时避免增加无必要的发布单元。

## 5. 统一消息合同

### 5.1 `ChannelEnvelopeV1`

所有跨 Fabric 消息都必须具备以下语义；平台缺失的字段使用显式 `None`，不得伪造：

```text
ChannelEnvelopeV1
├─ schema_version: 1
├─ envelope_id: UUID
├─ occurred_at: UTC timestamp
├─ direction: ingress | egress | internal_projection
├─ address: ChannelAddress
│  ├─ channel
│  ├─ account_id?
│  ├─ sender_id?
│  ├─ chat_id
│  └─ thread_id?
├─ correlation: Correlation
│  ├─ session_key
│  ├─ request_id?
│  ├─ trace_id?
│  ├─ message_id?
│  ├─ reply_to?
│  └─ sequence?
├─ origin: external_user | owner_frontend | runtime
├─ payload: ChannelPayloadV1
└─ extensions: namespaced platform-only object
```

规则：

- `envelope_id` 标识一次 Fabric 事件，不替代平台 `message_id`。
- `message_id` 保留平台或前端消息身份，用于去重、reply/edit/delete。
- `thread_id` 在 turn 创建时冻结并贯穿所有 delta/finalize，禁止从“当前 chat sticky”反查。
- Agent turn 必须带 HQ-03 已建立的 `session_key/request_id/trace_id` 关联。
- `origin` 是运行时来源门禁，不由外部 payload 自报后直接信任。
- `extensions` 只允许命名空间键，例如 `qq.*`、`feishu.*`；公共能力不得塞入其中。

### 5.2 `ChannelPayloadV1`

```text
Message { parts, subject?, locale? }
Typing { state: started | stopped | listening }
Stream { phase: started | delta | finalized | cancelled | failed, parts }
Reaction { operation: add | remove, emoji }
Delete { target_message_id }
Delivery { receipt }
Health { status, diagnosis? }
Control { operation, body }
Presentation { event, body }
Gap { last_durable_cursor, reason }
```

### 5.3 Typed content

`ContentPart` 首版冻结为：

- `Text { text }`
- `Markdown { markdown }`
- `Image { attachment }`
- `Audio { attachment, transcript? }`
- `Video { attachment }`
- `File { attachment }`
- `Location { latitude, longitude, label? }`
- `Card { schema, body }`
- `Reference { uri, title?, media_type? }`

`AttachmentRef` 只保存受控引用、媒体类型、大小、摘要和可选文件名；二进制不得无限内嵌
WebSocket frame。上传/下载复用 Manager 文件服务，并继续执行大小、路径和 MIME 检查。

### 5.4 投递结果

```text
DeliveryReceipt
├─ status: accepted | delivered | rejected | failed | unsupported
├─ channel
├─ chat_id
├─ platform_message_id?
├─ thread_id?
├─ retry_after_ms?
├─ error_code?
└─ diagnosis?
```

发送成功必须返回真实 receipt。平台只确认“已接受”时不得声称 `delivered`；不支持的操作
必须返回 `unsupported`，不得使用 `Ok(())` 静默吞掉。

## 6. 能力合同

`ChannelCapabilities` 是代码声明与运行时 probe 的合并结果，不进入用户配置：

| 组 | 能力 |
| --- | --- |
| ingress | text、markdown、thread、group、direct、typed attachments、dedup ID |
| egress | text、chunking、reply、image、audio、video、file、card |
| interaction | typing、listening、edit、delete、reaction、stream finalize |
| reliability | health、heartbeat、resume、token refresh、pacing、supervised restart |
| limits | max text chars、attachment bytes、supported MIME、rate-limit hints |

能力来源优先级固定为：平台实时 probe > adapter 静态声明。用户不能通过配置把 `false`
强行改成 `true`。Capability Matrix 同时驱动 Manager、GUI 展示、降级策略和 TCK。

降级规则：

1. Markdown 不支持时降为 Text；
2. 可编辑流不支持时只发送 final；
3. Card 不支持时使用明确的 Text/Markdown 摘要；
4. Attachment 类型不支持时返回 `unsupported`，不得静默跳过；
5. Approval/Ask User 不能由不支持交互 UI 的平台自动代答。

## 7. `ChannelAdapter` 与运行时

目标接口采用 Octos 的长期 listener + direct send 形状，但去掉默认成功的可选方法：

```text
ChannelAdapter
├─ name() -> ChannelId
├─ capabilities() -> ChannelCapabilities
├─ start(AdapterContext) -> long-running Result
├─ execute(ChannelCommand) -> DeliveryReceipt
├─ health() -> ChannelHealth
└─ stop() -> Result
```

`ChannelCommand` 是封闭枚举，包含 Send、Typing、Edit、Delete、React、FinalizeStream 和
ProbeHealth。Registry 根据 channel name 路由命令；adapter 必须穷尽匹配，并对不支持能力
返回 typed error。平台 SDK 的 token refresh、WebSocket resume、签名和媒体上传留在 adapter
内部，共享 supervisor/pacing 只管理生命周期和速率，不接管平台协议细节。

### 7.1 Supervisor

每个 listener 运行在独立 supervised task：

- 正常退出也视为需要诊断的终止；
- panic 被捕获并投影为 `Down`，不得杀死 Gateway；
- 使用有上限的指数退避和 jitter 重启；
- 平台明确要求固定 heartbeat/resume 时以平台合同优先；
- stop/cancel 打断 sleep、网络读取和重连；
- 连续失败进入 `Degraded/Down`，不无限刷日志。

退避、队列和阈值是代码常量并由负载测试冻结，不增加配置键。

### 7.2 Pacing

- 所有发送先经过每 adapter 的 pacing lane；
- 平台 `Retry-After` 优先于本地退避；
- 文本分片在 pacing 前完成，并保持同一 logical message/thread；
- 失败分片停止后续分片并返回部分投递诊断；
- retry 只允许幂等或带平台幂等键的操作。

## 8. 有界队列与背压

Fabric 中不得出现 `mpsc::unbounded_channel`。所有 lane 使用固定代码容量，并在 C1 的负载
表征后冻结具体数值；容量不是用户配置，也不能直接复制 Octos 的 4096。

逻辑 lane：

| Lane | 内容 | 饱和行为 |
| --- | --- | --- |
| control | cancel、stop、approval answer、ACK | 保留容量；不得被 delta 挤占 |
| ingress | 用户消息、附件引用 | await bounded admission；超时返回 busy |
| durable event | accepted、terminal、approval、error | 不静默丢弃；写 journal 后发送 |
| transient event | delta、typing、progress、heartbeat | 合并同 key 最新值；必要时发送 Gap |
| adapter egress | 平台发送命令 | pacing + bounded wait；返回 retry hint |

与 Session Admission 的关系：Fabric 只负责 transport admission；消息进入 Agent 后仍由
per-session bounded queue 决定 queued/started/rejected。Fabric 必须原样投影
`SessionAdmissionObservation`，不得把“进入 Fabric”误报为“开始生成”。

取消沿 control lane 直接关联 `session_key/request_id`，优先于同请求尚未发送的 delta；
取消完成后晚到 delta 必须被 generation/request fence 丢弃。

## 9. Neuro-Link v1

### 9.1 Transport

- Endpoint：`ws://127.0.0.1:<manager-port>/api/neuro-link/v1/ws`。
- Listener：复用 Manager 已有 loopback listener，不新增 host/port。
- Wire：UTF-8 JSON、JSON-RPC 2.0、最大 frame 由代码常量限制。
- Schema：`neuro-link/v1` JSON Schema 是 Rust 与 TypeScript 的唯一 wire 权威。
- Version：客户端必须声明精确 v1；不匹配立即返回 `protocol_version_mismatch` 并关闭。
- Auth：测试阶段无身份、配对、token 或权限限制；非 loopback 入口不存在。

`frontend_instance_id` 由客户端每次进程启动生成，只用于诊断、连接排重和 event echo，
不作为 principal、device 或授权依据。v1 schema 不包含伪造的 `principal_id/device_id`。

### 9.2 Commands

| Method | 作用 |
| --- | --- |
| `protocol/hello` | 精确版本与客户端 capability 交换 |
| `service/list` | 返回已登记 HTTP Service Bindings |
| `session/open` | 打开 session 并携带可选 durable cursor |
| `turn/start` | 提交 typed content，返回 request/trace/admission |
| `turn/cancel` | 按 session/request 取消 queued 或 running turn |
| `event/ack` | 确认最后处理的 durable cursor |
| `state/resume` | 按 cursor 请求增量或受限快照 |

v1 不使用 `authenticate`、`device/register` 或权限协商方法。

### 9.3 Notifications

| Method | 语义 |
| --- | --- |
| `session/opened` | Session 信息、当前 cursor、服务目录 |
| `turn/admission` | queued/started/rejected/finished 真实阶段 |
| `conversation/stream` | started/delta/finalized/cancelled/failed |
| `tool/lifecycle` | tool start/progress/finish 的安全投影 |
| `approval/required` | 领域 Approval 引用，不复制 ledger payload |
| `question/required` | Ask User 结构化问题 |
| `planning/changed` | Plan/TODO 状态投影 |
| `presentation/event` | speaking/expression/subtitle 等语义事件 |
| `state/gap` | 易失事件缺口，要求 resume/snapshot |
| `service/changed` | Service Binding 目录变化 |

所有通知都带 `session_key/request_id/trace_id` 中适用的字段。`conversation/finalized`、
`cancelled`、`failed` 对同一 request 只能出现一个终态。

## 10. Versioned Service Bindings

Neuro-Link 不把所有 CRUD 塞进 WebSocket。现有 Manager 领域 handler 原位登记为唯一
Service Binding：

```text
ServiceBinding
├─ service_id
├─ schema_version
├─ methods[]
├─ http_base
└─ required_frontend_capability?
```

首版目录包含 Session、Workspace、Persona、Memory、Planning、Approval、Ask User、Files、
Skills、Providers、Cron、AutoDream、Audit 和 Health。登记层只描述已有 handler，不创建
`/v1` 复制路由，不新增 DTO shim。handler 的领域校验、审计和错误码继续是唯一实现。

旧 `/api/chat`、`/api/chat/stop` 和 `/api/events` 是旧实时前端路径，不属于领域 CRUD；
桌面 GUI 切换后删除。其他 HTTP API 只有在确认未消费且不属于 Service Binding 时才删除。

## 11. State Sync 与 Projection Journal

采用混合日志：

- durable：command idempotency、turn accepted、唯一终态、approval/question 引用、cancel、
  service catalog change、最新 snapshot cursor；
- transient：text/reasoning delta、typing、heartbeat、频繁 progress；
- authoritative snapshot：从 Session、Run、Planning、Approval 等现有权威即时组合；
- journal：位于 Manager data root 的独立 `super-channel-events.db`，不得放入 `.laputa/`。

Journal 只保存恢复必需的最小投影和引用，不保存附件二进制、provider secret、原始审批
敏感参数或完整 BML 内容。删除/重置 Session 时同步清除对应投影；即使 journal 丢失，也能
从领域权威生成新 snapshot，只损失已过期的增量 replay。

Cursor 由 `{stream, sequence}` 构成。客户端 cursor：

- 在保留窗口内：返回 durable 增量，并从 snapshot 补 transient 最终状态；
- 太旧或未来值：返回 `cursor_out_of_range` 和当前 head，客户端重新 `session/open`；
- 属于其他 session：返回 `cursor_invalid`，不得跨 session 重放。

## 12. Presentation 与 Mate 迁移

删除 `OLV_AVATAR_CHAT_ID`、`OLV_AVATAR_ROLE`、`neuro_link_pipe=speak` 和特殊 outbound
bridge。Agent/Manager 只发布语义化 Presentation Event：

- `assistant.speaking.started/completed`
- `assistant.thinking.started/completed`
- `assistant.waiting_for_approval`
- `assistant.tool.started/completed`
- `persona.expression_hint`
- `subtitle.updated/cleared`

GUI/Mate 根据 capability 自行选择 VRM 动作、TTS、字幕或纯文本。Core 不知道具体动画名、
Avatar chat ID 或渲染进程。

## 13. 频道配置政策

1. 不新增 generic queue、retry、capability、supervisor 或 Neuro-Link 配置。
2. 现役平台保留确实被消费、或 Octos 同类适配使用的凭据/策略字段。
3. 删除未消费字段、退役平台字段和 `neuro_link/neuro-link/generic_pipe` 旧别名。
4. `allow_from` 等现有访问策略若确实执行则保留；空列表语义必须在 TCK 中固定。
5. 平台 endpoint 只有在上游支持切换或离线测试注入需要时才保留；不得新增“万能 base URL”。
6. Provider model-ID safety rule不受本 Epic 影响。

## 14. 现役频道迁移基线

| 频道 | 必须保留 | 本 Epic 必补 | 主参考 |
| --- | --- | --- | --- |
| Telegram | polling、媒体、HTML/Markdown | ID、reply/edit/delete、typing、health、TCK | Octos |
| Discord | Gateway、DM/Guild、媒体 | thread binding、reaction、edit/delete、health | Octos |
| Feishu | Protobuf WS、心跳、图片/卡片 | reply、edit/delete、typed media、mock server | Octos + 当前 Diva |
| DingTalk | Stream WS、群/私聊、媒体 | supervisor、pacing、health、receipt | 当前 Diva + ZeroClaw 研究 |
| Email | IMAP/SMTP、consent、polling | typed subject/file、dedup、health、backpressure | Octos + 当前 Diva |
| QQ | 官方 WS、C2C、resume/dedup | 群消息、message ID、typed media、receipt | Octos 结构 + 后续能力 TCK |

任何增强只使用平台已要求的凭据或现有有效策略，不借能力补齐增加通用用户配置。

## 15. Clean Break 迁移

### 15.1 原子切换原则

实现阶段使用隔离 worktree/branch。C1～C5 可以分批提交，但 `dev` 只在以下条件同时满足后
接收一次原子合并：

1. 桌面 GUI 已完全使用 Neuro-Link v1 和 Service Bindings；
2. 六个现役频道全部实现新 `ChannelAdapter`；
3. 全量 TCK、workspace gates 和真实桌面 smoke 通过；
4. 旧生产路径和退役源码已经删除；
5. `rg` clean-break gate 证明旧符号不在产品、配置、GUI、CI 或脚本中。

不得为了分批上线建立 old-to-new/new-to-old adapter、DTO conversion shim、双发 bus 或配置
fallback。开发分支允许旧文件在删除批次前尚存，但新实现不调用旧实现。

### 15.2 删除清单

- `NeuroLinkHandler`、旧 register/msg/delta/reply pipe 和独立 TCP listener；
- `NeuroLinkConfig`、`generic_pipe`、`neuro_link` alias 和频道设置卡片；
- `OLV_AVATAR_CHAT_ID`、`OLV_AVATAR_ROLE`、`neuro_link_pipe=speak`；
- `ChannelHandler`、`ChannelHandlerPtr`、旧 `InboundMessage/OutboundMessage`；
- MessageBus 的 unbounded ingress/egress 与 callback subscriber；
- 旧 `/api/chat`、`/api/chat/stop`、`/api/events` SSE；
- Slack、WhatsApp、Matrix、IRC、Mattermost、Nextcloud Talk 源码、配置和 Cargo feature；
- 只服务旧 Neuro-Link/退役频道的脚本、测试、文案和文档。

历史决策记录和归档文档不改写，但必须明确标注为历史，不参与 clean-break 产品扫描。

## 16. 故障模型与可观测性

| 故障 | 对外结果 | 恢复 |
| --- | --- | --- |
| Fabric ingress 满 | `busy` + retry hint | 客户端重试；不创建幽灵 turn |
| transient lane 满 | 合并并发出 `state/gap` | 客户端 resume/snapshot |
| durable journal 写失败 | 命令失败或连接降级 | 不发送无法证明的终态 |
| adapter listener 退出 | Health Down/Degraded | supervisor 有界重启 |
| 平台 rate limit | typed receipt + retry_after | pacing lane 延后 |
| WS 断线 | Session/turn 继续由权威运行 | cursor resume |
| GUI 重复 command | 返回原 idempotent outcome | 不重复执行 |
| 晚到 delta | generation/request fence 丢弃 | 保持唯一终态 |
| schema/version 不匹配 | 明确错误并关闭 | 升级客户端，不降级协议 |

日志至少包含 `channel/session_key/request_id/trace_id/envelope_id` 中适用的标识，禁止记录
凭据、完整附件、原始敏感审批参数和连续媒体。Health、queue depth、drop/coalesce、retry、
reconnect、delivery latency 和 journal lag 提供结构化指标。

## 17. TCK 与验收

### 17.1 Protocol TCK

- JSON Schema 正反 fixture、未知字段政策和 exact-version rejection；
- hello、session/open、turn/start、cancel、ACK、resume；
- 唯一终态、idempotency、cursor range、跨 session 拒绝；
- frame/attachment size、错误码和敏感字段脱敏。

### 17.2 Fabric TCK

- FIFO ingress、跨 session 并发、同 session 顺序；
- control lane 不被 delta 饱和阻塞；
- transient coalesce、Gap、durable 不丢失；
- adapter panic/退出/重连、停止和 shutdown drain；
- pacing、Retry-After、部分分片失败和 receipt。

### 17.3 Adapter TCK

每个现役频道必须对 capability matrix 的每个 `true` 能力给出离线 fixture 或 mock HTTP/WS
证明；`false` 必须返回 typed unsupported。禁止仅通过检查方法存在来宣称能力。

### 17.4 产品 E2E

1. 桌面 GUI 连接 v1、打开既有 Session、发送消息并看到真实 admission/delta/final。
2. queued 与 running turn 都可取消，GUI 不出现晚到内容。
3. GUI 断线重连后恢复唯一终态、审批和 Planning 状态。
4. Mate 由 Presentation Event 驱动 speak/subtitle，不存在特殊 chat ID。
5. 至少一个真实平台频道完成入站、流式降级/编辑和最终 receipt。
6. 非 loopback 无监听；旧 pipe fixture 必须失败。

标准门禁：`just fmt-check`、`just check`、`just test`、GUI tests/build、Tauri/桌面 smoke、
MSRV 1.80 probe 和 clean-break `rg` gate。

## 18. 实施 WBS

| 批次 | 交付 | 合并门禁 |
| --- | --- | --- |
| C0 | 本架构、决策冻结、Epic 启动 | 文档无待决策、引用可恢复 |
| C1 | JSON Schema、核心 typed contracts、characterization | schema/TCK、旧行为基线 |
| C2 | bounded Fabric Kernel、receipt、capability、supervisor | queue/fault tests |
| C3 | Neuro-Link v1、journal、Service Binding catalog | protocol TCK、loopback smoke |
| C4 | 桌面 GUI 首个客户端、Presentation | GUI tests/build、桌面 smoke |
| C5 | 六现役 adapter 迁移与 capability TCK | 每频道 fixture、真实样板 |
| C6 | 删除旧树与六退役频道、全量门禁、原子合并 | clean-break + release acceptance |

C1～C6 在独立 `feat/channel-epic` worktree 开发。任何批次不得单独把双轨产品状态合入
`dev`；只有 C6 满足时进行原子合并。

## 19. 不变量

1. Neuro-Link 是超级通道，不再是普通 `ChannelHandler`。
2. 外部平台永远不会因统一 envelope 获得 Owner 权限。
3. 所有频道队列有界，所有饱和行为可观察。
4. 能力来自 adapter 声明/probe，不来自用户强制配置。
5. 每个 request 只有一个终态，thread/request 关联不可从 sticky 状态猜测。
6. Journal 是可重建投影，不是领域权威，也不进入 BML/Laputa。
7. v1 不认证但只能 loopback；远程访问不在本 Epic。
8. JSON Schema 是 wire 权威，Rust/TypeScript 都必须通过同一 TCK。
9. 不提高 MSRV，不引入 Octos workspace 依赖。
10. 最终产品树没有旧协议、旧 DTO、旧 bus、退役频道或兼容层。

## 20. 参考

- 本地 Octos：`C:\Users\Administrator\Desktop\morediva\.workspace\octos`，commit
  `5ea987813de4fd2afdd1d78f2106ad2868f0d923`；重点参考
  `crates/octos-bus/src/channel.rs`、`bus.rs`、`qq_bot_channel.rs`、
  `feishu_channel.rs`、`dingtalk_channel.rs`。
- [外部频道能力对照](../../research/channel-capability-reference-2026-08/README.md)
- [Neuro-Link 前端超级通道研究](../../research/diva-workbench-pen-mirror-neurolink-2026-08/neurolink-front-end-fabric.md)
- [Workbench 总体架构边界](../../research/diva-workbench-pen-mirror-neurolink-2026-08/architecture.md)
- [Session Admission Operator Guide](../harness-session-admission/operator-guide.md)
