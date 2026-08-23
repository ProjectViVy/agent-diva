# agent-diva 外部频道能力与参考实现对照

> 日期：2026-08-23
> 状态：Research / Proposal；待正式立项，未授权生产实现
> 范围：ZeroClaw、Octos、OpenFang 与当前 agent-diva 的 QQ、DingTalk、Feishu 以及共享频道运行时

## 1. 结论

如果只能选一个项目作为 agent-diva 的“主对照”，选择 **Octos**；如果要选一个
频道能力和可靠性的“验收标杆”，选择 **ZeroClaw**；OpenFang 适合作为统一消息内容、
Bridge 和交互生命周期的模式参考。

因此不建议全盘复制某一个项目，建议采用：

```text
Octos：主结构对照（最接近 agent-diva 的 Rust 分层和 Channel/Bus 形状）
  + ZeroClaw：能力与可靠性标杆（QQ、DingTalk、Feishu、限速、监督重连）
  + OpenFang：Bridge/TCK 参考（内容块、反应、typing、并发和适配器测试）
  + agent-diva：保留现有治理边界、Provider、Sandbox、Approval、Laputa/BML
```

这不是说 Octos 的频道实现整体强于 ZeroClaw。准确的判断是：**Octos 更容易成为
agent-diva 的改造基座，ZeroClaw 更适合定义“做到什么程度才算够用”。**

## 2. 调研快照与依据

本次以本地 `.workspace` 快照为准，避免把不同日期的上游代码混在一起：

| 项目 | 本地快照 | 主要代码入口 |
| --- | --- | --- |
| Octos | `5ea9878`，`v2.0.3-rc.9`，2026-08-23 | `.workspace/octos/crates/octos-bus/src/channel.rs`、各 `*_channel.rs` |
| ZeroClaw | `d91e08e`，2026-06-25 | `.workspace/zeroclaw/zeroclaw-api/src/channel.rs`、`.workspace/zeroclaw/zeroclaw-channels/` |
| OpenFang | `acf2587`，`v0.6.9`，2026-05-12 | `.workspace/openfang/crates/openfang-channels/src/` |
| agent-diva | 当前 `dev` 工作树 | `agent-diva-channels/src/`、`agent-diva-core/src/bus/` |

## 3. 总体对照

以下是针对本项目落地的相对判断，不是对项目整体质量的评分；5 分代表更适合作为该
维度的参考。

| 维度 | ZeroClaw | Octos | OpenFang | 判断 |
| --- | ---: | ---: | ---: | --- |
| 与 agent-diva 现有 Rust 分层的贴合度 | 2 | **5** | 3 | Octos 的 `core/bus/gateway/channel` 形状最接近 |
| 共享频道合同完整度 | **5** | 4 | 3 | ZeroClaw 合同最丰富；Octos 已覆盖 edit/delete/typing/reaction/health |
| QQ 直接能力 | **5** | 3 | 0 | ZeroClaw 覆盖面最广；Octos 有 C2C/群 @，但没有完整媒体能力 |
| DingTalk 直接能力 | **5** | 1 | 2 | Octos 当前是机器人 Webhook 文本路径，低于 agent-diva 现有 Stream 实现 |
| Feishu/Lark 直接能力 | **5** | 4 | 2 | Octos 已有 WS/Webhook、媒体、回复、编辑/删除；ZeroClaw 更完整 |
| 通道运行时可靠性 | **5** | 3 | 2 | ZeroClaw 有监督重启、pacing、watchdog；Octos 需要补通用监督层 |
| 可抽取性/改造成本 | 2 | **5** | 3 | ZeroClaw 与自身 config/runtime/infra 耦合较深 |
| Bridge 与多适配器测试模式 | 3 | 4 | **5** | OpenFang 的内容块和 Bridge 组合值得借鉴 |

## 4. Octos 适合作为主对照的原因

### 4.1 结构形状最接近

Octos 把 `octos-core`、`octos-bus`、`octos-agent`、`octos-server` 和 gateway adapter
分开；频道通过统一 `Channel` trait 注册到 `ChannelManager`。这与 agent-diva 的
`agent-diva-core`、`agent-diva-agent`、`agent-diva-manager`、`agent-diva-channels`
边界可以建立一一对应关系，不需要把另一个项目的整套 Agent Runtime 搬进来。

Octos 的共享 Channel 合同已经包含：

- `start`、`send`、`stop`、allowlist、平台消息长度和自动分片；
- typing/listening；
- `send_with_id`、编辑、删除、流式消息 finalize；
- reaction、embed、原始 SSE；
- `health_check` 以及 `Healthy/Degraded/Down/Unknown` 状态；
- 明确的 thread binding，用于避免并发流式回复串线。

它的 Bus 使用容量为 4096 的 bounded `mpsc`，而 agent-diva 当前 outbound bus 是
unbounded。这是可以局部吸收的运行时经验，但不能直接照搬容量值；最终应根据
agent-diva 的背压、任务取消和 GUI/CLI 消费模型重新定标。

### 4.2 已经有当前问题涉及的三个频道

Octos 同时有 `qq-bot`、`dingtalk`、`feishu` 适配器，并有 channel-level 测试，因此
可以作为逐频道合同迁移的近邻样本。尤其 Feishu 适配器的 mock Axum server 测试，适合
转化为 agent-diva 的 wire-contract 测试模板。

### 4.3 但不能把 Octos 当前频道代码当成能力终点

- Octos QQ 已支持官方 WebSocket、C2C、群 @ 消息、去重、消息 ID、断线重连和会话恢复，
  但发送侧对 `media` 明确记录为“暂不支持并跳过”，当前文件也没有完整 Guild 路径。
- Octos DingTalk 是自定义机器人 Webhook/`sessionWebhook` 文本路径，带签名校验和会话
  Webhook 缓存，但没有 agent-diva 当前 Stream 适配器那样的长连接能力，出站媒体也明确
  不支持。
- Octos Feishu 已有 WS/Webhook、签名/加密校验、图片/文件上传、Interactive Markdown
  卡片、reply endpoint、消息 ID、编辑、删除和较完整测试；但没有看到通道级
  reaction add/remove 实现，也不能视为已经具备 ZeroClaw 的审批卡片、draft lifecycle
  和完整交互能力。
- `ChannelManager::start_all` 会启动 listener 并记录 listener 退出，但通用管理器本身
  没有 ZeroClaw 那种统一的监督重启和退避策略；部分频道内部有自己的 reconnect loop，
  两者不能混为一谈。

结论是：**用 Octos 设计共享接口和接线方式，用 ZeroClaw 补能力，不要把 Octos 的
DingTalk 适配器倒退移植到 agent-diva。**

## 5. 三个项目在关键频道上的具体差异

### 5.1 QQ

当前 agent-diva QQ 适配器对群组/Guild 事件采取显式拒绝，主要缺口是外部通道覆盖面；
这正是最应该用 ZeroClaw 做 capability oracle 的地方。

| 实现 | 已确认能力 | 明显限制 |
| --- | --- | --- |
| agent-diva | 官方 OpenAPI/WebSocket 基础、C2C、重连/去重基础 | 当前不接收群/Guild；需要补统一 message ID、媒体和 richer reply contract |
| Octos | C2C、群 @、官方 WS、dedup、message ID、heartbeat、Resume/reconnect、测试 | 出站媒体明确跳过；未见完整 Guild 路径 |
| ZeroClaw | C2C、群、Guild、媒体类型、上传缓存、语音转写、token retry、proxy、seq/resume、测试 | 代码和运行时耦合较重，直接移植成本高 |
| OpenFang | 没有实际 QQ adapter | 不适合作为 QQ 主参考 |

QQ 的推荐路线是：以 Octos 的群/C2C 事件和 session key 作为最小结构样本，按 ZeroClaw
的能力矩阵补 Guild、媒体、重试、代理和语音，不把 QQ 特殊字段继续塞进无类型 metadata。

### 5.2 DingTalk

DingTalk 是一个容易误判的点。Octos 并不是当前问题的最佳参照：它的实现更像一个稳定
的文本机器人 Webhook，而 agent-diva 当前已经有 Stream 长连接、群/私聊、去重以及媒体
上传/发送路径。ZeroClaw 才适合用来对照通用连接治理、健康检查、重试和 Markdown 合同。

| 实现 | 已确认能力 | 结论 |
| --- | --- | --- |
| agent-diva | Stream WS、HTTP 发送、群/私聊、去重、Markdown、图片/文件/视频路径 | 当前 transport/media 基础不应被 Octos 替换掉 |
| Octos | Webhook 入站、签名校验、`sessionWebhook` 缓存、文本分片 | 可借鉴验签/会话缓存，不能作为 DingTalk 能力基线 |
| ZeroClaw | Stream + HTTP、群/私聊、Markdown、health/proxy/retry 等通用能力 | 作为可靠性和合同标杆 |
| OpenFang | 基础 Robot Webhook 与单独 Stream adapter，主要是文本 | 可借鉴 Stream 重连和 adapter 组织，能力仍偏窄 |

因此 DingTalk 的适配原则是 **保留 agent-diva 当前 Stream 主路径，抽取通用能力合同**，
而不是向 Octos 的文本 Webhook 路径收敛。

### 5.3 Feishu/Lark

Feishu 是 Octos 最有参考价值的频道之一：它同时处理 WS 长连接和 Webhook，保留平台
message ID，能通过 reply endpoint 建立线程，发送图片/文件，并提供编辑/删除测试。
agent-diva 当前 Protobuf WS、心跳、去重、图片和交互卡片基础可以继续保留。

ZeroClaw 适合作为后续增强标杆，重点看 draft patch/finalize、reaction add/remove、审批
卡片、token refresh/retry 和更完整的媒体生命周期。OpenFang 的通用内容块/生命周期模型
适合补充跨频道抽象，但不是 Feishu API 细节的主参考。

## 6. 对 agent-diva 的适配方案

### 方案 A：Octos-first，推荐作为近期主路线

在 `agent-diva-channels` 内部吸收 Octos 的 Channel 合同形状和 bounded bus 思路，保留
agent-diva 现有的 Manager、AgentLoop、Sandbox、Approval、Laputa/BML；逐个替换/扩展
入站出站 envelope。

适合：希望以较小架构扰动补齐 QQ/Feishu，并保持当前 agent-diva 治理边界。

主要工作：

1. 先扩展统一消息合同：`message_id`、`thread_id`、`origin`、typed attachment、
   delivery status、capabilities；
2. 为 Channel 增加 typing、edit/delete、reaction、health、stream finalize 等可选能力；
3. 将 outbound bus 从无界队列改为有界/可观测的策略队列，保留明确的 overflow 行为；
4. QQ 以 ZeroClaw 的 capability matrix 补群/Guild、媒体、token retry 和代理；
5. Feishu 吸收 Octos 的 reply/edit/delete/mock server，并以 ZeroClaw 补 draft/approval；
6. DingTalk 保留当前 Stream 和媒体路径，只迁移共享 supervisor、pacing、health 合同。

风险：Octos 是 Rust 2024/MSRV 1.85，而 agent-diva 当前工作区 MSRV 为 1.80；应移植
设计和小段逻辑，不直接引入其 workspace 依赖或提高 agent-diva MSRV。

### 方案 B：ZeroClaw-first，频道优先

以 ZeroClaw 的 `ChannelMessage`/`SendMessage`、`MediaAttachment`、`PacedChannel`、
supervised listener 和 capability trait 为目标，直接重构 agent-diva 的频道公共层。

适合：近期目标是尽快达到 QQ/DingTalk/Feishu 的生产级能力，并能接受较大的公共层改造。

优点是能力上限最高，缺点是 ZeroClaw 的 config、allowlist、runtime、session 和 infra
边界与 agent-diva 不同，直接移植会带入不需要的生命周期、配置和安全假设。建议只采纳
合同和独立算法，不复制其运行时。

### 方案 C：OpenFang-first，Bridge 优先

以 OpenFang 的 `ChannelContent`（Text/Image/File/Voice/Location/Multipart）、Bridge 的
并发、typing refresh、lifecycle reaction、rate limit 和 adapter TCK 为公共抽象，再由
agent-diva 自己实现 QQ/DingTalk/Feishu。

适合：未来要先统一“消息如何表达、发送中如何反馈、附件/反应如何管理”，且频道数量会
快速增长的阶段。

缺点是没有实际 QQ adapter，DingTalk/Feishu API 细节较薄，Bridge 的 listener 结束恢复也
不能直接视为生产级 supervisor。

### 方案 D：Diva-native 三源组合，推荐作为最终形态

不把任何参考项目作为上游基座，而是拆成四个可验证合同：

| 合同 | 主要参考 | agent-diva 归属 |
| --- | --- | --- |
| Channel/Bus/adapter 接线 | Octos | `agent-diva-channels` + `agent-diva-core::bus` |
| 能力、限速、重连、token retry | ZeroClaw | `agent-diva-channels` 的 shared runtime |
| 内容块、生命周期反应、TCK | OpenFang | `agent-diva-channels` 测试与 typed message |
| 治理、权限、审批、持久化 | agent-diva 当前实现 | Manager/Sandbox/Approval/Laputa/BML |

这是长期最稳的方案，但前置工作多，必须先把消息合同和能力矩阵冻结，再分频道施工。

## 7. 建议的能力基线与验收顺序

不要以“代码行数”或“支持频道数量”作为完成标准，建议建立每个频道的 capability
matrix/TCK：

1. 入站：文本、平台 `message_id`、sender/chat/thread、群/私聊、来源标记、去重；
2. 出站：文本分片、reply-to、媒体附件、失败可诊断、平台错误映射；
3. 交互：typing/listening、edit/delete、reaction、card/approval、stream finalize；
4. 可靠性：连接心跳、断线重连、token refresh、指数退避、限速/pacing、队列背压；
5. 安全：allowlist、签名/加密校验、代理/URL allowlist、日志脱敏；
6. 测试：离线协议 fixture、mock HTTP/WS、重连/重复事件/并发流式回复测试。

建议顺序：先公共 envelope 与 TCK，再 QQ 群/C2C 和 Feishu reply/media，随后补
DingTalk Stream 的 supervisor/pacing/health，最后再做 Guild、draft、审批卡片和语音。

## 8. 与 A2A 方案的关系

频道能力升级不是 A2A 的替代品，但两者共享以下底层合同：`message_id`、`thread_id`、
typed parts/attachments、stream event、task/run 状态、取消和 delivery receipt。若先把
频道增强做成仅 QQ/DingTalk 的私有字段，后续 A2A 会再次出现 envelope 分裂。

所以本研究建议把频道公共合同设计为未来 A2A adapter 可复用的内部消息层，但继续遵守
现有边界：A2A/频道入口必须经过 Manager、Sandbox、Approval、Laputa/BML 治理，不能直接
调用 BML 写 API。

## 9. 暂不做的事情

- 不直接复制任何参考项目的 workspace、配置系统、memory/store 或 agent runtime；
- 不因为 Octos 使用 Rust 2024 就把 agent-diva MSRV 直接升到 1.85；
- 不把 Octos 的 DingTalk 文本 Webhook 当成 agent-diva 的回退实现；
- 不在本研究阶段修改生产 channel 代码；正式施工前按 `LOCK.md` 新建独立实现任务。
