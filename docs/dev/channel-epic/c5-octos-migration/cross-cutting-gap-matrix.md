# C5-P2 跨频道能力差距矩阵

本表描述“当前可执行代码”与“C5 目标”，不是把目标误写成已完成。状态含义：

- `Implemented`：当前代码和测试已有可复核路径。
- `Partial`：存在局部路径，但缺少统一 envelope、失败语义或完整证据。
- `Missing`：当前没有真实路径。
- `Retain-DIVA`：DIVA 现有能力优于 Octos，迁移时必须保留。
- `Blocked`：目标需要先确认外部协议或共享契约。

## 跨频道主矩阵

| 能力 | Telegram | Discord | Feishu/Lark | DingTalk | Email | QQ | 统一目标与验收 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 文本入站 | Partial | Implemented | Partial | Partial | Implemented | Partial | 六频道 typed envelope + message ID fixture |
| 图片/附件入站 | Partial | Partial | Partial | Partial | Partial | Missing | 权限前下载、AttachmentStore、MIME/size/digest fixture |
| 图片识别链路 | Missing as typed vision | Missing as typed vision | Marker only | Missing | Missing | Missing | attachment → context → provider vision capability；不支持时 typed error |
| 群聊入站 | Partial | Partial | Partial | Partial | Inapplicable | Missing | chat type/group ID/sender/mention/policy fixture |
| 群聊出站 | Partial | Partial | Partial | Implemented Stream | Inapplicable | Missing | command 冻结 group address，真实 receipt |
| DM/allowlist | Implemented | Partial | Implemented | Implemented policy | Implemented | Implemented C2C | 空 allow_from allow-all；非空严格限制 |
| mention/command gating | Partial | Partial | Platform policy | Platform policy | Inapplicable | Group @ missing | 未授权不得 admission/media；命令不绕过 allowlist |
| reply/thread correlation | Partial | Partial | Partial | Partial | Partial | Partial C2C | message_id/reply_to/thread_id 不串线，property fixture |
| Dedup timing | Partial | Partial | Partial | Partial | Partial | Partial | Fabric admission 成功后提交 marker |
| text chunking/limits | Partial | Partial | Partial | Partial | Partial | Partial | boundary-1/boundary/boundary+1 + receipt |
| send/reply message ID | Partial | Partial | Partial | Partial | Partial | Missing response parsing | 成功解析真实 ID，否则 Accepted 无假 ID |
| edit/delete | Partial | Missing/partial | Partial | Missing | Inapplicable | Inapplicable | false capability 零 transport side effect |
| reaction | Missing | Partial | Retain-DIVA seen marker | Missing | Inapplicable | Missing | command 与 best-effort seen 分离 |
| cards/embed/keyboard | Partial keyboard | Partial embed | Partial card | Missing | Inapplicable | Missing | schema/size/error fixture |
| typing/listening | Partial typing | Partial typing | Missing | Missing | Inapplicable | Inapplicable | 可取消 action task，无 detached 任务 |
| media upload/egress | Partial | Partial | Partial | Retain-DIVA | Retain-DIVA multipart | Missing | 每项成功/失败明确，禁止 log-and-skip |
| token refresh | Static token | Static token | Partial tenant token | Partial OAuth | Inapplicable | Partial access token | single-flight、安全重试一次、脱敏 |
| heartbeat/reconnect/resume | Polling reconnect | Partial Gateway | Partial WS | Stream | Polling | Implemented WS | 状态机、health、cancel fixture |
| webhook signature/encryption | Inapplicable | Inapplicable | Missing/partial | Partial HMAC | Inapplicable | Inapplicable | fail-closed、URL verification 例外 |
| approval routing | DIVA only | DIVA only | DIVA only | DIVA only | DIVA only | DIVA only | Manager/Sandbox/Ask User；外部身份不可伪造 |
| health/backpressure | Partial | Partial | Partial | Partial | Missing | Partial | bounded admission + Healthy/Degraded/Down/Unknown |

## 用户明确点名的四项重点

### 图片识别

当前六个 legacy handler 多数把媒体当作 `Vec<String>` 路径或 marker；这不等于可供 provider
识别。后续实现必须先建立 content-addressed `ChannelAttachmentStore`，再由 context assembly
生成 typed image content。每个频道至少需要一份真实 MIME、大小、digest、下载失败和 provider
不支持 vision 的 fixture。

### 群聊

Telegram、Discord、Feishu、DingTalk 和 QQ 必须分别记录群 ID、sender、mention、reply/thread；
Email 标记为不适用。QQ 当前群事件明确拒绝，必须保持 `Missing`，直到 group inbound/outbound
fixture 和真实 smoke 完成；不能把已有 C2C 测试当成群聊证据。

### 审批

Octos channel 代码没有 DIVA 的治理语义。审批统一由 Manager/Sandbox/Ask User 产生，频道只负责
展示和收集明确 decision。卡片不可用时采用受限文本 token；过期、重复、伪造身份和拒绝都必须
有审计结果，任何外部 payload 不能直接声称 owner。

### 权限管理

`allow_from` 的可执行语义是空列表允许所有；各频道 policy 再做额外限制。权限判断必须早于媒体
下载、Fabric admission、审批和工具调用。Discord guild/channel、DingTalk dm/group、Telegram
mention、QQ group/C2C 都必须拥有独立测试，而不是共享一个 allowlist 单测。

## 证据规则

每个 `Implemented` 或目标 `T` 都必须链接到：

1. 频道深扫报告中的源码/symbol 行。
2. `tests/fixtures/c5/<channel>/` 或等价 mock fixture。
3. Rust 测试名和预期 receipt/error。
4. 若是实时可靠性能力，再链接 heartbeat/reconnect/health 证据。

缺少任一项就降为 `Partial` 或 `Blocked`，不得为了关闭矩阵而改成 `Implemented`。
