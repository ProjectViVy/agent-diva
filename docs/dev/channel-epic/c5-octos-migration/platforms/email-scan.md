# Email：Octos → DIVA 深度扫描报告

> 扫描类型：C5-P2 只读事实扫描。Email 是 IMAP/SMTP 协议通道，不以 HTTP endpoint 数量衡量能力。
> Octos 基线固定为 `5ea987813de4fd2afdd1d78f2106ad2868f0d923`。

每个差距表的 `Decision` 使用 `Port`、`Adapt`、`Retain-DIVA`、`Reject` 或 `Blocked`。

## 证据入口

| 侧 | 文件与定位 |
| --- | --- |
| DIVA | `agent-diva-channels/src/email.rs`：config/校验 56-109，IMAP 177-303，SMTP 308-406，lifecycle 414-570，send 572-688，tests 702-786 |
| DIVA config/policy | `agent-diva-core/src/config/schema.rs`：EmailConfig 873-918/default 947-966；`agent-diva-channels/src/base.rs`：allowlist 73-220；`src/manager.rs`：注册/校验 |
| Octos | `.workspace/octos/crates/octos-bus/src/email_channel.rs`：EmailConfig 20-31，start/send 58-103，imap_poll 105-271，smtp_send 274-314，thread 333-347，self-reply 439-453，tests 474-567 |
| Octos assembly/docs | `.workspace/octos/crates/octos-cli/src/commands/gateway/adapters/email.rs`；`book/src/channels.md` 263-287；`book/src/configuration.md`；`book/src/testing.md` |

## 协议操作与字段语义

| 操作 | Octos 行为 | DIVA 当前 | 目标/决策 |
| --- | --- | --- | --- |
| IMAP connect | TLS TCP → `async_imap::Client` → login | `fetch_messages_blocking`/spawn_blocking 路径 | `Port`：保留 TLS 与 blocking isolation |
| Mailbox | select `INBOX`（Octos）；DIVA 可配置 mailbox | DIVA 支持 mailbox | `Retain-DIVA`：配置 mailbox，不强制 INBOX |
| Poll | `search("UNSEEN")`，fetch `RFC822` | DIVA poll UNSEEN，poll interval 可配置 | `Port`：bounded/cancellation-aware poll |
| Parse headers | `From`、`Subject`、`Message-ID`、`In-Reply-To`、`References` | DIVA 有 header/UID/date metadata，但 thread 不完整 | `Adapt`：统一 header parser，保留 DIVA parser |
| Thread | `References` root → `In-Reply-To` → Message-ID → normalized subject，生成 topic | DIVA 主要 sender/last_message_ids，缺稳定 thread anchor | `Port` 语义，适配到 DIVA metadata/address |
| Body | text/plain，HTML fallback，UTF-8 安全 truncate | HTML-to-text 和 body limit 已有；截断须核查字节切片 | `Adapt`：使用 UTF-8 safe truncate |
| Mark seen | fetch 后 `STORE +FLAGS (\\Seen)` | `mark_seen` 可配置 | `Retain-DIVA + Adapt`：仅 admission 成功后 mark seen |
| Self-reply | `Re:` 且 sender 与自身地址相同时跳过 | DIVA 有 last_subjects/self-reply 逻辑 | `Port`：用 Message-ID/thread + canonical address，防循环 |
| Allowlist | `allowed_senders` 为空则允许所有 | DIVA `allow_from` | `Retain-DIVA`：统一空列表 allow-all，邮箱地址规范化比较 |
| SMTP | port 465 implicit TLS；其他 STARTTLS | DIVA use_ssl/use_tls 可配置 | `Retain-DIVA`：保留全部 TLS 选项和 consent |
| Text/reply send | plain text，subject metadata；DIVA 写 `In-Reply-To/References` | DIVA 支持 subject_prefix、auto_reply、reply headers | `Adapt`：冻结 reply envelope 和 receipt Message-ID |
| Multipart attachments | Octos 当前无附件 | DIVA 支持 multipart outbound attachments | `Retain-DIVA`：保留附件，不退回 Octos 窄能力 |
| Health | Octos 无完整健康探针 | DIVA 有轮询错误日志 | `Adapt`：bounded IMAP NOOP/select + SMTP connect/auth，禁止发送邮件 |

## DIVA 特有能力必须保留

- `consent_granted=false` 时拒绝启动和发送。
- `auto_reply_enabled` 控制是否自动发送回复。
- `subject_prefix`、`from_address`、SMTP/IMAP 用户名回退规则。
- multipart/HTML 发送、附件读取错误和 SMTP rejection 的显式结果。
- `spawn_blocking` 隔离阻塞式 IMAP/SMTP，不得阻塞 async runtime。

## 当前差距与实施决策

| 能力 | DIVA 当前状态 | Octos 证据 | 决策 | 必要 fixture |
| --- | --- | --- | --- | --- |
| IMAP UNSEEN poll | 已有 | 105-222 | `Port` | fake select/search/fetch |
| Consent/auto-reply | 已有且更强 | Octos 无 | `Retain-DIVA` | consent false/auto-reply false |
| Thread topic | 缺稳定 root 语义 | 333-347、532-567 | `Adapt` | References/In-Reply-To/subject fallback |
| Self-reply | 部分 sender/subject 逻辑 | 439-453 | `Port` | own reply and non-reply |
| Mark seen ordering | 需证明 admission 后 | 220+ | `Adapt` | busy leaves unseen |
| UTF-8 body limit | 需检查字节切片 | `truncate_utf8` | `Port` | boundary unicode |
| Multipart attachments | DIVA 已有，Octos 无 | DIVA 347-374 | `Retain-DIVA` | image/audio/file MIME |
| SMTP reply headers | DIVA 已有 | Octos send 83-97 | `Adapt` | In-Reply-To/References roundtrip |
| Health/backpressure | 未形成统一合同 | Octos Channel trait/bus | `Port` | bounded NOOP/connect, cancellation |
| Email interaction | 无平台能力 | Octos 无 | `Reject` | Unsupported with zero side effects |
| Approval/permissions | DIVA governance | Octos 无治理 | `Retain-DIVA` | explicit consent/approval identity |

## 安全、线程和生命周期不变量

1. 未授权发件人、consent 未授权、空 recipient、坏地址、超大 MIME 都是显式错误，不是 `Ok(())`。
2. `Message-ID`、`In-Reply-To`、`References` 必须从原始邮件保存到 outbound reply；不能只按 sender 的最近消息推断。
3. `mark_seen` 只有在 Fabric admission 成功后执行；Fabric busy 时邮件必须可重试。
4. poll cancellation 必须 fence 后续 blocking 结果，不得把取消后的邮件再次投递。
5. 日志不得写密码、完整邮箱、主题正文、附件内容；fixture 使用脱敏 RFC 822。

## 后续实施顺序

1. 冻结 Email address/thread/attachment metadata 和 `Accepted` Message-ID receipt。
2. 统一 blocking boundary、poll health、dedup/mark-seen ordering。
3. 迁移 Octos thread/self-reply 算法，同时保留 DIVA consent、auto-reply 和 multipart。
4. 补 fake IMAP/SMTP、RFC 822、MIME、busy/cancel、health 和 zero-side-effect unsupported fixtures。
