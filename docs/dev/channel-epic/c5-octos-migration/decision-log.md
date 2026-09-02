# C5-P2 迁移决策日志

状态词：`resolved` 表示本阶段已冻结；`blocked` 表示实施前必须补证据，但不允许通过猜测继续。

| ID | 状态 | 决策 | 依据 | 影响/验收 |
| --- | --- | --- | --- | --- |
| D-001 | resolved | 只迁移 Telegram、Discord、Feishu、DingTalk、Email、QQ；退休频道只做 Clean Break inventory | C5 范围与当前 Manager | 六频道各有独立报告和 owner |
| D-002 | resolved | 外部 wire 行为尽量原样；内部服从 DIVA Fabric/Manager/Sandbox/Approval/BML 边界 | C5 ADR-1…ADR-9 | 不复制 Octos bus/manager/runtime/deps |
| D-003 | resolved | 所有 channel 采用 permission-before-media、admission-before-dedup-commit | DIVA 安全边界、Octos event/receipt 证据 | 未授权和 Fabric busy 零副作用 fixture |
| D-004 | resolved | typed attachment 统一进入 content-addressed AttachmentStore | DIVA target architecture | image/file/audio 必须有 MIME/size/digest/receipt |
| D-005 | resolved | 图片识别由 provider vision capability 决定；不支持时 typed error | 用户点名图片识别缺口 | 不能用 URL/绝对路径伪装 vision |
| D-006 | resolved | 群聊必须按平台独立实现 mention、group policy、sender/chat ID | Telegram/Discord/Feishu/DingTalk/QQ 差异 | QQ 当前拒绝群消息不能当完成证据 |
| D-007 | resolved | 审批只由 DIVA Manager/Sandbox/Ask User 产生；频道只呈现和回传 decision | Octos 无 DIVA governance | 外部身份不可伪造 owner；过期/重复均有审计 |
| D-008 | resolved | DingTalk 保留 DIVA Stream、OAuth、媒体和 dm/group policy；只借 Octos HMAC/session cache | Octos webhook 明显更弱 | 禁止降级为 text-only webhook |
| D-009 | resolved | Feishu 保留 DIVA protobuf WS、ACK、heartbeat、reaction seen、card/table；补 Octos region/webhook/media/edit/delete | 两侧能力互补 | reaction seen 不等于通用 reaction command |
| D-010 | resolved | Email 保留 consent、auto-reply、TLS、multipart；采用 Octos thread/self-reply 语义 | DIVA 产品能力更强，Octos parsing 更规范 | mark_seen 只在 admission 后执行 |
| D-011 | resolved | Telegram 保留 `/start`、`/reset`、`/help`、`/stop`；补 mention、caption/media、callback、keyboard、bound edit/delete | DIVA command compatibility + Octos contract | `/stop` 不再依赖硬编码 localhost |
| D-012 | resolved | Discord 不引入 Serenity，仅以现有 HTTP/WS transport 实现 Octos 能力 | DIVA 现有 transport 可用 | Gateway/REST fixture 必须覆盖 reaction/embed/edit/delete |
| D-013 | blocked | QQ Identify intents 采用哪组位图必须以官方 event delivery fixture 决定，不能直接照搬 | DIVA `(1<<25)|(1<<12)` 与 Octos `(1<<25)|(1<<30)` 不一致 | QQ agent 先完成官方验证；未完成前 matrix 状态为 Blocked |
| D-014 | blocked | QQ media 是否有可用官方 Bot API wire path | Octos text-only、DIVA 当前缺失 | 未有 endpoint+fixture 前保持 Unsupported，不得假成功 |
| D-015 | resolved | 成功发送默认 `Accepted`；只有平台明确确认才称 `Delivered` | DIVA receipt ADR | 返回真实 message ID；没有 ID 不伪造 |
| D-016 | resolved | 共享 endpoint 只能通过 adapter 私有 test injection；禁止产品级全局环境覆盖 | 并行测试和安全要求 | 每个 mock server 私有、可并发 |

## C5-V audit record (2026-09-02)

- Six native adapters now have channel-owned wire fixtures/tests and Gate 3 evidence pages;
  the shared TCK freezes the capability snapshots and checks unsupported commands before any
  platform transport call.
- `capability-evidence.json` contains 29 machine-auditable rows. Only rows with complete
  fixture, test, request, response and receipt/error evidence are `verified`; the remaining
  implemented rows stay `partial`.
- QQ C2C/group wire behavior, deduplication, heartbeat and truthful outbound IDs have local
  evidence, but D-013 (official intents delivery proof) and D-014 (official media wire path)
  remain `blocked`. The ignored live harness is present but no external credentials or platform
  permission were available in this worktree.
- No shared protocol field, channel configuration key, global endpoint override, Manager
  production assembly or C6 clean-break change was introduced. C5-V/C5-Q therefore remain open.
- To make the required Rust 1.80 channel probe reproducible, `348d42a1` pins compatible
  resolutions in `Cargo.lock` only; no `Cargo.toml` dependency constraint or MSRV was changed.

## C5-V partial capability audit record (2026-09-02)

- The 21 rows that were `partial` in the previous manifest were audited one by one against the
  current DIVA adapter, its Gate3 page, and Octos
  `5ea987813de4fd2afdd1d78f2106ad2868f0d923`. The six channel audit commits are `fc1f5a40`
  (Telegram), `1a9e1065` (Discord), `e1009c40` (Feishu), `d9fc146d` (DingTalk), `c2851602`
  (Email), and `0c5d2b8e` (QQ).
- No row was upgraded: the manifest remains 7 `verified`, 21 `partial`, and 1
  `blocked/unsupported`. Each Gate3 page now distinguishes `[implementation_gap]` from
  `[evidence_gap]` and records when an Octos path is only a reference rather than DIVA proof.
- Confirmed implementation gaps include Telegram group response gating and oversized-download
  semantics, Discord DM/mention/dedup/download/reply semantics, Feishu JSON error validation,
  DingTalk active heartbeat plus multipart token-refresh/idempotency behavior, Email IMAP SSL/MIME/
  health semantics, and QQ heartbeat reset/invalid-session cooldown behavior. The complete row-level
  disposition remains in `evidence-manifest.md` and `capability-evidence.json`.
- Feishu admission-before-dedup and DingTalk permission-before-media are confirmed code paths, but
  the corresponding platform wire/lifecycle evidence is still incomplete. Octos webhook, shared
  media, and shared dedup helpers are not counted as proof for a different DIVA transport path.
- No shared contract/configuration request was opened. This audit changed only documentation and
  provenance metadata; it did not change adapters, fixtures, tests, Cargo files, Manager assembly,
  C6, `dev`, or push anything. No compile, test, or live-network command was run.

## blocked 项的处理协议

QQ 的 D-013/D-014 不阻塞其它频道实施。QQ agent 必须提供：官方文档/源码证据、mock gateway 或
HTTP fixture、预期事件/响应和失败语义；协调 agent 将本表更新为 `resolved` 或保持 `blocked`，
并同步 capability matrix、evidence manifest 和 TODOLIST。禁止在代码中以“先试一下”替代决策。
