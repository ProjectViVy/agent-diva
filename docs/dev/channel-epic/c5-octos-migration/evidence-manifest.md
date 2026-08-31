# C5-P2 能力证据清单

本清单是后续 C5-I/V 的证据索引。`planned` 只表示必须补的证据，不代表能力已实现；只有
fixture、测试和 receipt/error 结果全部存在后才能改为 `verified`。

| ID | Channel | Capability | 当前状态 | 证据计划 | 验收结果 |
| --- | --- | --- | --- | --- | --- |
| TG-01 | Telegram | text/caption ingress | partial | `fixtures/c5/telegram/inbound-text.json` + parser/admission test | typed envelope + message ID |
| TG-02 | Telegram | group mention/reply/command gating | partial | group DM/mention/reply/command fixtures | policy-before-media |
| TG-03 | Telegram | photo/voice/audio/document | partial | Bot API mock `getFile` + bounded download | AttachmentRef MIME/size/digest |
| TG-04 | Telegram | callback/keyboard | partial | `answerCallbackQuery` + keyboard request capture | callback event + safe data |
| TG-05 | Telegram | send/reply/edit/delete/typing | partial | request/response mock | real message ID/receipt |
| TG-06 | Telegram | reconnect/dedup/health | partial | stream end/duplicate/getMe | cancellation and health |
| DC-01 | Discord | Gateway opcode/heartbeat/resume | partial | scripted WS frames | state transition |
| DC-02 | Discord | DM/guild/thread/mention | partial | MESSAGE_CREATE variants | correlation invariant |
| DC-03 | Discord | attachments/multipart | partial | attachment download/send mock | content-addressed refs |
| DC-04 | Discord | edit/delete/reaction/embed | missing/partial | REST request capture + emoji fixtures | no-op forbidden |
| DC-05 | Discord | 429/rate-limit/health | partial | header/body retry_after | `RateLimited` receipt |
| FS-01 | Feishu | region/token/WS | partial | cn/global token + frame | cache and ACK deadline |
| FS-02 | Feishu | webhook signature/AES | missing/partial | valid/missing/bad/encrypted/url verification | fail-closed |
| FS-03 | Feishu | typed image/file/audio/media | partial | resource download fixtures | AttachmentStore |
| FS-04 | Feishu | upload/send/reply/edit/delete | partial | multipart/mock response IDs | bound receipt |
| FS-05 | Feishu | reaction seen/dedup | partial | best-effort reaction + duplicate event | inbound success unaffected |
| DT-01 | DingTalk | Stream OAuth/register/ACK | partial | token/register/WS script | no ACK-and-drop |
| DT-02 | DingTalk | group/private policy | partial | conversation type + allowlist matrix | zero unauthorized side effects |
| DT-03 | DingTalk | media upload/send | partial | `/media/upload` multipart | partial failure explicit |
| DT-04 | DingTalk | HMAC/sessionWebhook | partial | valid/bad signature + TTL cache | compatible side path, no downgrade |
| EM-01 | Email | IMAP UNSEEN/thread headers | partial | RFC822 + fake IMAP | thread root and dedup |
| EM-02 | Email | consent/auto-reply/TLS | implemented/retain | config matrix + no-send probe | denied before network send |
| EM-03 | Email | multipart attachments | implemented/retain | fake SMTP MIME capture | all parts or typed failure |
| EM-04 | Email | mark-seen/health/cancel | partial | busy/cancel/NOOP/select | no early seen |
| QQ-01 | QQ | token/gateway/Identify | partial/blocked intents | mock HTTP + official intents evidence | correct events received |
| QQ-02 | QQ | heartbeat/resume/invalid/cooldown | partial | existing integration + new typed receipt | session state |
| QQ-03 | QQ | C2C ingress/egress | partial | C2C event/send response | message ID/seq |
| QQ-04 | QQ | group ingress/egress | missing | group event/send fixtures | group address and @ policy |
| QQ-05 | QQ | media | missing/blocked | official wire-path research first | unsupported until proven |

## Verification rule

每个 `verified` 条目必须同时提供：

- 深扫报告的源码/symbol 定位。
- 最小脱敏 fixture 或本地 mock server/WS/IMAP/SMTP transcript。
- passing Rust test 名称。
- 预期请求、响应、receipt 或 typed error。
- 对可靠性能力，附状态机和 cancellation 证据。
