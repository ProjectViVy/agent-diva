# Neuro-Link：完整前端接入面

## 1. 定位修正

本轮讨论中曾出现“把 Neuro-Link 作为 PEN Link 传输”的早期推测。经用户明确纠正，
该推测作废：

> Neuro-Link 是理论上的超级通道，可以完整承担 agent-diva 的新前端；它不是 PEN 的
> 专用传输实现。

PEN 可以使用 stdio、HTTP、WebSocket、MCP、A2A、WASM 等不同驱动；Neuro-Link 则定义
前端与 DIVA Core 之间的身份、交互、状态和控制关系。

## 2. 为什么仍可称为重量级 Channel

Neuro-Link 仍具有双向、异步、持续连接和会话属性，因此“Channel”作为产品分类可以
保留。但它属于高信任 `Owner Interface Channel`，与社交平台 Channel 有本质差异：

| 维度 | 普通平台 Channel | Neuro-Link |
| --- | --- | --- |
| 主体 | 平台 sender/chat | 用户、设备、前端实例 |
| 信任 | 默认不可信/有限信任 | 强认证后的 Owner Frontend |
| 内容 | 消息和附件 | 消息、状态、表达、控制、工作区 |
| 生命周期 | 平台投递/重连 | 设备登录、同步、恢复、能力协商 |
| 权限 | Channel allowlist | 设备、用户、会话和操作级策略 |
| 前端替代 | 否 | 可以实现完整 DIVA 前端 |

## 3. 推荐协议分层

完整前端不等于把所有 Manager API 复制成一个 WebSocket 枚举。推荐：

~~~text
Neuro-Link Connection
├─ Identity / Device Binding
├─ Capability Negotiation
├─ Conversation Stream
├─ Presentation Stream
├─ Control Stream
├─ State / Event Stream
├─ File / Media Transfer
└─ Versioned Service Bindings
   ├─ Persona Service
   ├─ Memory Service
   ├─ Planning Service
   ├─ Evolution Service
   └─ Workbench Service
~~~

实时流使用 Neuro-Link 多路复用；持久化 CRUD 仍由各领域 Service 负责。Neuro-Link
负责发现、认证、相关性、事件游标和统一错误，不成为第二套业务权威。

## 4. 最小消息合同

公共 envelope 至少需要：

~~~text
protocol_version
connection_id
frontend_instance_id
device_id
principal_id
session_id
stream_id
message_id
correlation_id
sequence
deadline
capability
payload
~~~

控制消息至少包括：

- `hello / challenge / authenticate`；
- `capabilities / service_bindings`；
- `request / response / event`；
- `stream_open / stream_chunk / stream_close`；
- `cancel / ack`；
- `heartbeat`；
- `snapshot_request / snapshot / resume`；
- `error`。

## 5. 前端能力协商

前端不应假设所有设备都能显示同一种内容。连接时应声明：

- 文本、Markdown、结构化卡片和附件能力；
- 音频播放、TTS、字幕和口型；
- 2D/3D Mirror、动作和表达提示；
- 麦克风、摄像头、位置、拖放；
- 审批和 Ask User 交互；
- Persona/Memory/Evolution/Planning 工作区支持；
- 后台通知、离线缓存和本地加密能力。

Core 根据能力选择表达降级：没有 3D Mirror 时仍可以输出文本；没有麦克风时不发送
录音请求；不能展示审批的前端只能收到“需要在受信任前端处理”的状态。

## 6. 状态同步

Neuro-Link 必须以“断线是常态”设计：

- 每条事件有单调 sequence 或 durable cursor；
- 前端重连携带最后确认游标；
- Core 返回增量或受限快照；
- optimistic UI 不能成为权威；
- 终态、审批、取消和工具执行不能因重连重复；
- 文件和媒体使用独立传输/引用，不能无限塞入事件帧；
- 队列有界，并明确 overflow、降级和重新同步语义。

## 7. 当前原型与迁移

当前 `agent-diva-channels/src/neuro_link.rs` 已包含 register、msg、delta/reply 和 Avatar
特殊路由，只能视为实验性 pipe。正式路线应：

1. 保留旧实现作为开发原型，不承诺兼容；
2. 冻结 v1 前端合同与威胁模型；
3. 建立独立 Neuro-Link Gateway/Service；
4. 让当前 GUI 或一个最小 Web 客户端成为首个 TCK 客户端；
5. 迁移 Mate 的 `speak` 特例到 Presentation Stream；
6. 通过同一合同验证手机/桌面第二前端；
7. 最后决定旧 ChannelHandler 包装是否保留。

## 8. 安全最低线

- 默认只监听 loopback；远程访问必须显式启用；
- loopback 也需要挑战认证和短期会话令牌；
- 设备身份与用户主体分离；
- 前端权限不是“连接成功即管理员”；
- Approval、Memory、Persona 等高风险操作要求独立 capability；
- 记录设备、前端、Session、动作和结果审计；
- 支持设备撤销、会话终止和全局 Kill Switch；
- 禁止客户端自报 sender/role 后直接获得信任。
