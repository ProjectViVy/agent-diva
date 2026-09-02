# C5-V/C5-Q capability evidence manifest

本清单对应 `agent-diva-channels/tests/fixtures/c5/capability-evidence.json` 和六份
`platforms/*-gate3.md`。`verified` 只表示该行的窄能力组已经同时具备源码 symbol、脱敏
fixture/local transport、passing Rust test、精确请求/响应或 Fabric 结果及所需生命周期
断言；不能从一行推导同频道其它能力已完成。`partial` 表示仍缺一项或多项证据，
`blocked/unsupported` 表示按决策明确不宣称。

本轮 C5-V 审计已逐项复核 21 行 `partial`：每行都对照当前 DIVA adapter、对应 Gate3
审计页和固定 Octos SHA `5ea987813de4fd2afdd1d78f2106ad2868f0d923`。缺口前缀含义为：
`[implementation_gap]` 表示源码行为本身仍不足或有风险；`[evidence_gap]` 表示代码
路径可能存在但尚无本轮要求的精确 wire/lifecycle 证据；`[intentional_boundary]` 表示
按已冻结决策不应借用 Octos 的另一条路径补齐。21 行状态在本轮均保持 `partial`。

| ID | Channel | Capability group | 状态 | Fixture / test evidence | 当前精确结果与缺口 |
| --- | --- | --- | --- | --- | --- |
| TG-01 | Telegram | text/caption ingress | `verified` | `telegram/inbound-text.json`; `wire_telegram_ingress_preserves_identity_media_and_callback_ack` | typed envelope 保留 chat/sender/thread/reply/message ID；getUpdates long-poll lifecycle 仍在 TG-06 |
| TG-02 | Telegram | group mention/reply/command gating | `partial` | `telegram/inbound-group-media.json`; same ingress test | `[implementation_gap]` `process_message` 仅 sender allowlist，未实现 Octos 的 DM/reply-to-bot/@mention/command 群响应门控；`[evidence_gap]` 缺 getUpdates offset/timeout/cancel/busy 与完整群/reply/mention transcript |
| TG-03 | Telegram | photo/voice/audio/document | `partial` | `inbound-group-media.json`, `api-responses.json`; ingress/multipart tests | `[implementation_gap]` 超限下载静默 `Ok(None)`，未形成显式 MIME/Content-Length/30s timeout 失败语义，附件来源 ID 语义也与 Octos 不完全一致；`[evidence_gap]` 缺 voice/audio/video/document 逐类 getFile/download/multipart、损坏读回与 AttachmentStore 证据 |
| TG-04 | Telegram | callback/keyboard | `partial` | `callback-query.json`; `wire_telegram_ingress_preserves_identity_media_and_callback_ack` | `[implementation_gap]` native adapter 无 keyboard/`reply_markup` outbound，callback ACK 不是 Octos callback-first 顺序且未授权 callback 不 ACK；`[evidence_gap]` 缺 ACK body/thread/duplicate/failure wire transcript |
| TG-05 | Telegram | send/reply/edit/delete/typing | `partial` | `api-responses.json`; `wire_telegram_egress_uses_html_reply_real_id_and_plain_fallback`, `wire_telegram_multipart_and_rate_limit_are_truthful` | `[implementation_gap]` action refresh、partial receipt 和 keyboard/media coverage 未形成完整语义；`[evidence_gap]` HTML/plain fallback、reply/真实 ID、multipart/429 已有窄证，缺 edit/delete、typing refresh、finalize、全媒体组合、Retry-After 与部分分片失败的完整 wire 证据 |
| TG-06 | Telegram | reconnect/dedup/health | `partial` | `api-responses.json`; `start/get_updates/probe_health` | `[implementation_gap]` `HashSet<i64>` 缺 Octos TTL/容量/forget 语义，批次失败可能推进 offset，health 失败不转 Down；`[evidence_gap]` 缺 getUpdates offset/timeout/cancel、backoff/reconnect、dedup 与 getMe health 生命周期 transcript |
| DC-01 | Discord | Gateway opcode/heartbeat/resume | `partial` | `discord/gateway-frames.json`; `wire_discord_gateway_handles_hello_identify_ready_heartbeat_and_cancel` | `[evidence_gap]` HELLO/IDENTIFY/READY/heartbeat ACK/cancel 有窄证；缺 op6 RESUME、op7 RECONNECT、op9 INVALID_SESSION、ACK timeout、close-code、cooldown 与 stop 的 scripted transcript；初始 HELLO 等待的取消边界也未闭合 |
| DC-02 | Discord | DM/guild/thread/mention | `partial` | gateway fixture; incoming parser/gateway tests | `[implementation_gap]` `guild_id` 限制可能误拒 DM，mention 未绑定 bot ID，seen 去重不是 Octos TTL/LRU/atomic reserve；`[evidence_gap]` 缺 DM/guild allow-deny、bot filter、mention、duplicate 全矩阵 |
| DC-03 | Discord | attachments/multipart | `partial` | `discord/rest-responses.json`; `wire_discord_rest_preserves_reply_embed_and_attachment_receipts` | `[implementation_gap]` inbound 下载未先检查 HTTP status，multipart reply 使用 `X-Reply-To` 而非标准 `message_reference`；`[evidence_gap]` 缺 image/audio/video/file 逐类上传/下载、MIME/size/损坏/多附件/IngressStore transcript |
| DC-04 | Discord | edit/delete/reaction/embed | `verified` | `discord/rest-responses.json`; `wire_discord_rest_preserves_reply_embed_and_attachment_receipts` | embed/reply POST、PATCH、DELETE、reaction、typing exact path/status 及真实 ID 已断言 |
| DC-05 | Discord | 429/rate-limit/health | `partial` | `discord/rest-responses.json`; `wire_discord_rate_limit_and_unsupported_listening_are_explicit` | `[evidence_gap]` 429 body 1.25s 优先于 header 9、返回 1250ms 与 Listening unsupported 零副作用已有精确证据；缺 health、403 permission、malformed/timeout/reconnect/retry transcript；`[implementation_gap]` 403 仍映射为通用 `discord_http` |
| FS-01 | Feishu | region/token/WS | `partial` | `feishu/protobuf-frame.json`, `region-endpoints.json`; protocol/token tests | `[implementation_gap]` `get_access_token/get_websocket_url` 不接收 cancellation；`[evidence_gap]` protobuf ACK `biz_rt=0`、region mapping、401 refresh 与部分 WS 生命周期已有证据，缺完整 heartbeat/reconnect/close/stop wire、token single-flight 压力、429/malformed、health 与 token wait cancel transcript；Octos 主动 ping/2 秒 reconnect 不替代 DIVA 证据 |
| FS-02 | Feishu | webhook signature/AES | `verified` | `webhook-url-verification.json`, `webhook-encrypted-event.json`; webhook/AES tests | URL challenge 可校验；缺 headers、bad signature/base64/padding 均 fail-closed |
| FS-03 | Feishu | typed image/file/audio/media | `partial` | `media-resource.json`; `media_fixture_is_stored_as_typed_attachment_after_authentication` | `[implementation_gap]` Feishu resource response 没有可比对的平台 digest/signature；`[evidence_gap]` authenticated image → bounded typed AttachmentRef 与 shared store 有窄证，缺 file/audio/video/sticker、MIME/size/timeout/cancel、SHA readback/corruption 的平台 wire 证据，shared `channel.rs`/Octos `media.rs` 不能替代 Feishu proof |
| FS-04 | Feishu | upload/send/reply/edit/delete | `partial` | `send-responses.json`; send/reply test | `[implementation_gap]` `execute_delete` 只检查 HTTP status，未检查 Feishu JSON `code != 0`；`[evidence_gap]` 缺 multipart upload、edit/delete/finalize、429/malformed/partial-failure 的完整 response transcript；audio/video 是明确 unsupported |
| FS-05 | Feishu | reaction seen/dedup | `partial` | `protobuf-event.json`; duplicate test | `[evidence_gap]` admission-before-dedup 与重复只 admission 一次已有窄证；缺 reaction failure spy、并发 pending/busy/cancel、TTL、replay/collision transcript；seen marker 不等于通用 reaction 成功 |
| DT-01 | DingTalk | Stream OAuth/register/ACK | `partial` | `dingtalk/stream-callback.json`, `token-response.json`; stream wire test | `[implementation_gap]` 当前主路径只有 WS Ping/Pong 响应，未证明 ReliabilityHeartbeat 所需的主动 heartbeat timer/frame；`[evidence_gap]` 缺 register 429、WS close 后 re-register、backoff/health/stop 与多连接 transcript；Octos webhook 路径不替代 DIVA Stream proof |
| DT-02 | DingTalk | group/private policy | `verified` | `stream-callback.json`; policy/media ingress tests | allowlist 在 media URL 前；group typed envelope admission 已断言 |
| DT-03 | DingTalk | media upload/send | `partial` | `media-upload-response.json`, `group-send-response.json`; media test | `[implementation_gap]` multipart 401/403 路径未闭合 token refresh，upload/send 未发送 `idempotency_key`；`[evidence_gap]` 已有 image/private/upload/real ID/429 窄证，缺 audio/video/file 逐类、完整 receipt/cancel/timeout/partial-failure wire |
| DT-04 | DingTalk | HMAC/sessionWebhook | `partial` | `stream-callback.json`; signature test | `[evidence_gap]` valid/bad/empty HMAC 的 fail-closed helper 形状与 Octos 对齐；缺 TTL expiry/cache miss/eviction 与 signed callback HTTP transcript；`[intentional_boundary]` 不接 webhook fallback 是 D-008 冻结边界，不是遗漏实现 |
| EM-01 | Email | IMAP UNSEEN/thread headers | `verified` | `email/plain.eml`, `reply.eml`; parser/fake IMAP tests | Message-ID 优先、UID fallback、References/In-Reply-To/thread root、Fabric admission 已断言 |
| EM-02 | Email | consent/auto-reply/TLS | `partial` | plain/reply EML; policy/unsupported tests | `[implementation_gap]` native IMAP 路径未读取 `imap_use_ssl`；`[evidence_gap]` allowlist/self-reply/unsupported 的前置拒绝已有窄证，缺 consent/auto-reply/empty recipient/malformed address/零传输顺序/TLS failure 与 raw transaction matrix；blocking cancel 仍缺 wire proof |
| EM-03 | Email | multipart attachments | `partial` | `email/multipart.eml`; parser/fake SMTP test | `[implementation_gap]` invalid MIME 会静默落到 `application/octet-stream`；`[evidence_gap]` 当前仅 image fixture，缺 audio/video/file/bad MIME/size 与 raw SMTP multipart wire；`smtp-fixture-1` 是 adapter seam receipt，不是 SMTP server receipt |
| EM-04 | Email | mark-seen/health/cancel | `partial` | reply EML; fake IMAP poll test | `[implementation_gap]` ProbeHealth 仅本地 accepted marker，mark-seen 失败不重试，blocking task 可 detach；`[evidence_gap]` 代码顺序为 `STORE \\Seen` 在 Fabric admission 后，但缺 raw IMAP ordering spy、busy/cancel/reconnect/STORE failure transcript |
| QQ-01 | QQ | token/gateway/Identify | `partial` | `qq/token-response.json`, `gateway-response.json`, `official-intents.json`; QQ gateway test | `[implementation_gap]` 若 QQ spec 要求 auth-expiry refresh-and-safe-retry，该路径当前缺失；`[evidence_gap]` token/Bearer discovery/Hello→Identify 有 local shape，缺完整 Identify intents、Bearer、expiry/429/refresh 的 native wire；`[intentional_boundary]` D-013 仍无官方事件投递证明，DIVA intents 与 Octos 差异不能由 Octos 推断覆盖 |
| QQ-02 | QQ | heartbeat/resume/invalid/cooldown | `partial` | `qq/gateway-frames.json`; QQ gateway test | `[implementation_gap]` `awaiting_heartbeat_ack` 跨 reconnect 未显式复位，且无 invalid-session 专属 cooldown/attempt ceiling；`[evidence_gap]` 缺 native RESUME/RESUMED/op7/op9/reconnect/backoff/health/stop/pending-wait cancellation transcript，legacy handler 测试不替代 native adapter 证据 |
| QQ-03 | QQ | C2C ingress/egress | `verified` | `qq/c2c-message.json`, `send-responses.json`; QQ gateway/outbound tests | C2C identity、admission dedup、`msg_seq`、reply `msg_id`、真实 response ID 已断言 |
| QQ-04 | QQ | group ingress/egress | `verified` | `qq/group-message.json`, `send-responses.json`; QQ gateway/outbound tests | group/member open ID、显式 group routing、`msg_seq`、真实 response ID 已断言 |
| QQ-05 | QQ | media | `blocked/unsupported` | `qq/official-intents.json`; `unsupported_media_fails_before_transport` | media/card 在首次 HTTP 前 typed reject；D-014 无官方 endpoint+wire proof，保持 blocked |

## Verification rule

每个 `verified` 条目必须同时提供：

- Octos/DIVA 源码 symbol 与 endpoint 定位；
- 最小脱敏 fixture 或私有 local mock server/WS/IMAP/SMTP transport transcript；
- passing Rust test 名称；
- 精确请求、响应、receipt 或 typed error；
- 对 health、dedup、retry、timeout、cancel 的状态/生命周期证据。

当前已验证的是窄行，不代表 C5-V/C5-Q 已完成。全量 workspace 门禁和 QQ ignored live
smoke 仍是退出条件；C6 Manager 切换保持禁止。
