# C5-P2 能力证据清单

本清单是 C5-I/V 的证据索引。`implemented/partial` 表示 native adapter 已有代码和离线
单测，但仍缺完整 wire mock/receipt/admission transcript；`planned` 表示尚未落代码；只有
fixture、测试和 receipt/error 结果全部存在后才能改为 `verified`。`blocked/unsupported`
表示依据 D-013/D-014 明确不宣称该能力。

| ID | Channel | Capability | 当前状态 | 证据计划 | 验收结果 |
| --- | --- | --- | --- | --- | --- |
| TG-01 | Telegram | text/caption ingress | implemented/partial | `fixtures/c5/telegram/inbound-text.json` + parser/admission test | typed envelope + message ID |
| TG-02 | Telegram | group mention/reply/command gating | partial | group DM/mention/reply/command fixtures | policy-before-media |
| TG-03 | Telegram | photo/voice/audio/document | implemented/partial | Bot API mock `getFile` + bounded download | AttachmentRef MIME/size/digest |
| TG-04 | Telegram | callback/keyboard | implemented/partial | `answerCallbackQuery` + keyboard request capture | callback event + safe data |
| TG-05 | Telegram | send/reply/edit/delete/typing | implemented/partial | request/response mock | real message ID/receipt |
| TG-06 | Telegram | reconnect/dedup/health | partial | stream end/duplicate/getMe | cancellation and health |
| DC-01 | Discord | Gateway opcode/heartbeat/resume | implemented/partial | scripted WS frames | state transition |
| DC-02 | Discord | DM/guild/thread/mention | implemented/partial | MESSAGE_CREATE variants | correlation invariant |
| DC-03 | Discord | attachments/multipart | implemented/partial | attachment download/send mock | content-addressed refs |
| DC-04 | Discord | edit/delete/reaction/embed | implemented/partial | REST request capture + emoji fixtures | no-op forbidden |
| DC-05 | Discord | 429/rate-limit/health | implemented/partial | header/body retry_after | `RateLimited` receipt |
| FS-01 | Feishu | region/token/WS | implemented/partial | cn/global token + frame | cache and ACK deadline |
| FS-02 | Feishu | webhook signature/AES | implemented/partial | valid/missing/bad/encrypted/url verification | fail-closed |
| FS-03 | Feishu | typed image/file/audio/media | implemented/partial | resource download fixtures | AttachmentStore |
| FS-04 | Feishu | upload/send/reply/edit/delete | implemented/partial | multipart/mock response IDs | bound receipt |
| FS-05 | Feishu | reaction seen/dedup | implemented/partial | best-effort reaction + duplicate event | inbound success unaffected |
| DT-01 | DingTalk | Stream OAuth/register/ACK | implemented/partial | token/register/WS script | no ACK-and-drop |
| DT-02 | DingTalk | group/private policy | implemented/partial | conversation type + allowlist matrix | zero unauthorized side effects |
| DT-03 | DingTalk | media upload/send | implemented/partial | `/media/upload` multipart | partial failure explicit |
| DT-04 | DingTalk | HMAC/sessionWebhook | implemented/partial | valid/bad signature + TTL cache | compatible side path, no downgrade |
| EM-01 | Email | IMAP UNSEEN/thread headers | implemented/partial | RFC822 + fake IMAP | thread root and dedup |
| EM-02 | Email | consent/auto-reply/TLS | implemented/retain | config matrix + no-send probe | denied before network send |
| EM-03 | Email | multipart attachments | implemented/retain | fake SMTP MIME capture | all parts or typed failure |
| EM-04 | Email | mark-seen/health/cancel | implemented/partial | busy/cancel/NOOP/select | no early seen |
| QQ-01 | QQ | token/gateway/Identify | implemented/partial/blocked intents | mock HTTP + official intents evidence | correct events received |
| QQ-02 | QQ | heartbeat/resume/invalid/cooldown | implemented/partial | existing integration + new typed receipt | session state |
| QQ-03 | QQ | C2C ingress/egress | implemented/partial | C2C event/send response | message ID/seq |
| QQ-04 | QQ | group ingress/egress | implemented/partial | group event/send fixtures | group address and @ policy |
| QQ-05 | QQ | media | blocked/unsupported | official wire-path research first | unsupported until proven |

## Gate 2 implementation commits

These commits are implementation evidence only; they do not close C5-V:

| Channel | Commit | Native module |
| --- | --- | --- |
| Discord | `42db4e25` | `src/adapters/discord.rs` |
| Email | `e0b36c02` (worker `d385aec4`) | `src/adapters/email.rs` |
| DingTalk | `f3bdd3ad` (worker `cbb57d72`) | `src/adapters/dingtalk.rs` |
| Feishu | `8088ce58` (worker `acf22081`) + factory `ca3b0cc6` | `src/adapters/feishu.rs` |
| Telegram | `10563a7a` | `src/adapters/telegram.rs` |
| QQ | `10563a7a` + `9171e9a4` | `src/adapters/qq.rs` |

The shared constructor/factory and TCK changes are in `10563a7a`; the C6 Manager must not be
cut over until the proof rows above are upgraded to `verified` and the QQ blocked decisions are
resolved or explicitly retained as product exclusions.

## Verification rule

每个 `verified` 条目必须同时提供：

- 深扫报告的源码/symbol 定位。
- 最小脱敏 fixture 或本地 mock server/WS/IMAP/SMTP transcript。
- passing Rust test 名称。
- 预期请求、响应、receipt 或 typed error。
- 对可靠性能力，附状态机和 cancellation 证据。
