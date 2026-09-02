# C5-P2/C5-I 子 agent 任务卡与交接协议

本文件把“扫描 → 方案 → 实施 → 验证”固定为可执行任务。每个频道只能由对应 owner
修改自己的 adapter、fixture 和扫描报告；共享契约、矩阵、ledger、Cargo、lib.rs 和 Manager
装配由 Lead 独占。

## 任务卡通用合同

每个频道 agent 必须按以下顺序完成：

1. 读取根 `AGENTS.md`、`LOCK.md`、C5 README、架构 ADR、自己的 `*-scan.md` 和 `*.md` 规格。
2. 在 Octos 固定 SHA 上复核所有 symbol、endpoint、字段、错误、重试和测试；若行号漂移，更新证据。
3. 生成频道 gap/decision 列表，不能把 Partial 写成 Implemented。
4. 只在共享契约 Gate 1 完成后修改产品代码。
5. 为每个 target-true 能力添加 fixture、mock 请求断言和 receipt/error 断言。
6. 运行频道测试、clippy、diff check；提交英文 Conventional Commit。
7. 交回 commit、测试命令/结果、未决风险、回滚点和下一步，不 merge、不 push。

## 共享 Gate 1：Lead ownership

Lead 独占以下文件/模块：

- `agent-diva-channels/src/adapter.rs`
- `agent-diva-channels/src/adapters/mod.rs`、`src/lib.rs`
- 共享 runtime/services、capability evidence/TCK、全局 manifest
- `agent-diva-channels/Cargo.toml`、Manager C6 assembly
- `03-capability-gap-matrix.md`、`endpoint-ledger.md`、`decision-log.md`

Gate 1 必须冻结：typed envelope、ChannelAddress、Correlation、ContentPart、AttachmentRef、
DeliveryReceipt、ChannelCommand、ChannelError、capability set、health、bounded admission、
dedup timing、permission-before-media、approval identity。共享契约变更只能通过
`decision-requests.md`，受影响 agent 必须暂停共享部分。

## 六个独立频道任务卡

| Agent | 产品 ownership | 必做实现 | 必做 fixture/验收 |
| --- | --- | --- | --- |
| Telegram | `adapters/telegram.rs` 与 `tests/fixtures/c5/telegram/**` | polling/reconnect、message/caption、photo/voice/audio/document、mention/reply/command、callback/keyboard、send/reply/edit/delete/typing、getMe | Bot API request capture、HTML fallback、caption 1024、media timeout、callback ACK、thread binding、cancel |
| Discord | `adapters/discord.rs` 与 `tests/fixtures/c5/discord/**` | Gateway opcode/heartbeat/resume、MESSAGE_CREATE、DM/guild/thread/mention、attachments、send/edit/delete/reaction/embed/typing、429 | gateway state machine、dedup-after-admission、multipart、emoji parse、Retry-After header/body、invalid session |
| Feishu | `adapters/feishu.rs` 与 `tests/fixtures/c5/feishu/**` | cn/global region、tenant token、protobuf WS/ACK、webhook signature/AES、typed media、send/reply/edit/delete、card；保留 reaction seen | frame/heartbeat/deadline、URL verification、fail-closed signature、encrypted event、multipart、region、dedup |
| DingTalk | `adapters/dingtalk.rs` 与 `tests/fixtures/c5/dingtalk/**` | DIVA Stream/OAuth/group/private/media；Octos HMAC/session cache 仅作为兼容能力 | Stream ACK/reconnect、HMAC valid/bad、session TTL、policy-before-media、media partial、3600 boundary；禁止降级 |
| Email | `adapters/email.rs` 与 `tests/fixtures/c5/email/**` | IMAP poll/mark-seen、thread headers、self-reply、consent/auto-reply/TLS/multipart、SMTP receipt、health | RFC822 plain/html/reply/multipart、fake IMAP/SMTP、busy mark-seen、UTF8 truncate、cancellation、unsupported zero-side-effect |
| QQ | `adapters/qq.rs` 与 `tests/fixtures/c5/qq/**` | token/gateway、Identify/Resume/heartbeat/backoff、C2C+group event/send、msg_seq/msg_id/receipt、allowlist；媒体仅有证据才实现 | official intents proof、group/C2C, duplicate, resume/invalid/cooldown, response ID, group send, media decision, live harness |

## 执行波次与合并顺序

资源受限时，每波最多三个频道 agent，但 ownership 仍保持一频道一 agent：

1. Wave A：Telegram、Discord、Feishu。
2. Wave B：DingTalk、Email、QQ。
3. Wave C：Lead 汇总、全局 TCK、交叉审查和文档收口。

Gate 1 之后的固定合并顺序：

1. shared adapter/services contract
2. Telegram
3. Discord
4. Feishu
5. DingTalk
6. Email
7. QQ
8. global evidence/TCK/live harness
9. C5-I/V logs and TODO closeout

每次合并前 Lead 检查 `git diff --check`、ownership、provenance 和 evidence manifest；任何
频道提交触碰共享文件或其他频道文件都拒绝合并。

## 交叉审查

实现完成后重新分派只读 reviewer：

- Telegram agent 审 Feishu/DingTalk 的 ACK、媒体和 dedup。
- Discord agent 审 Email/QQ 的 blocking、resume、附件和 receipt。
- Feishu agent 审 Telegram/Discord 的 thread、rate-limit 和 lifecycle。
- Lead 审共享契约、权限/审批、BML/Laputa 边界、C6 不变量。
