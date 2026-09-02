# DingTalk C5-V Gate 3 evidence — fixed-Octos audit

本页是 DT-01、DT-03、DT-04 的只读审计记录。审计没有修改 adapter、fixture、测试、公共契约、manifest、JSON、TODO 或锁文件；三行均按证据规则保留 `partial`。

## 审计基线

- DIVA worktree：`C:\Users\Administrator\Desktop\morediva\agent-diva-audit-dingtalk`，branch `c5-audit-dingtalk`，HEAD `2b3ef682`。
- Octos checkout：`C:\Users\Administrator\Desktop\morediva\.workspace\octos`，已核对 HEAD 为 `5ea987813de4fd2afdd1d78f2106ad2868f0d923`（`v2.0.3-rc.9`）。
- Octos 相对路径均相对于该 checkout；主要文件为 `crates/octos-bus/src/dingtalk_channel.rs` 和 `crates/octos-bus/src/channel.rs`。
- 既有验证记录 `docs/logs/2026-09-channel-epic/v0.2.2-c5-capability-evidence/verification.md:11-20` 记载 DingTalk adapter 10 项测试通过、channel all-target 测试和 clippy 通过；本次审计未重新执行任何测试。

## 冻结矩阵映射

`agent-diva-channels/tests/channel_adapter_shared_tck.rs:277-295` 的 DingTalk snapshot 目标为：

- DT-01 相关：`ReliabilityHealth`、`ReliabilityHeartbeat`、`ReliabilityTokenRefresh`、`ReliabilityPacing`、`ReliabilitySupervisedRestart`；`ReliabilityResume` 明确不支持。
- DT-03 相关：`EgressText`、`EgressMarkdown`、`EgressImage`、`EgressAudio`、`EgressVideo`、`EgressFile`；`EgressChunking` 明确不支持，文本上限为 3,600 字符、附件上限为 20 MiB。
- DT-04 是跨切面 HMAC/sessionWebhook 安全审计，没有对应的 `ChannelCapability` atom；它不能被当作额外 capability 或 webhook fallback 宣称。

## 审计结论表

| 行 | Octos 固定源码锚点 | DIVA 对照 | 现有证据 | 最终 disposition |
| --- | --- | --- | --- | --- |
| DT-01 | `dingtalk_channel.rs:88-128` `handle_webhook`；`:330-394` `start_webhook`/`Channel::start`；`:434-437` `stop`；`channel.rs:21-40` trait lifecycle | `DingTalkAdapter::register_stream`、`run_stream_once`、`handle_stream_message`、`start`、`stop`、`health`（`src/adapters/dingtalk.rs:572-630,1129-1433`） | `stream-callback.json`、`token-response.json`；`stream_register_callback_ack_cancel_and_dedup_are_wire_bound`；另有 token cache/401 test | `partial`：reconnect/backoff、health/stop/start loop 和 heartbeat wire proof 缺失；其中主动 heartbeat 若是 capability 要求则还有 implementation gap |
| DT-03 | `dingtalk_channel.rs:396-423` `send` 仅 text；`:430-432` `max_message_length`；`channel.rs:78-83` `send_with_id` 默认无 ID | `authenticated_multipart`、`send_attachment`、`execute_send`（`src/adapters/dingtalk.rs:550-570,924-1127`） | `media-upload-response.json`、`group-send-response.json`；`media_upload_and_send_preserve_typed_part_and_partial_failure` | `partial`：image/private/429 证据存在；audio/video/file、group media、完整 MIME/receipt/cancel 证据缺失，media 401 refresh 和 idempotency 还有 implementation gap |
| DT-04 | `dingtalk_channel.rs:34-79` HMAC/URL；`:169-206` `target_webhook`；`:250-328` `parse_event`；`:330-380` `start_webhook`；`channel.rs:241-247` health default | `dingtalk_signature`、`verify_dingtalk_signature`、`cache_session_webhook`、`cached_session_webhook`（`src/adapters/dingtalk.rs:829-863,1620-1657`）；没有 `signed_webhook_url`/`target_webhook`/`start_webhook`，这是禁止 fallback 的有意差异 | `stream-callback.json`；`signature_matches_octos_shape_and_fails_closed`、fixture parse test | `partial`：HMAC 纯函数等价且 fail-closed；TTL/过期和真实 signed callback transcript 缺失；缺口属于 evidence gap，不把被拒绝的 webhook fallback 当成实现缺失 |

## DT-01 — Stream OAuth/register/ACK

### Octos 与 DIVA symbol 对照

| 项目 | 固定 Octos 形状 | 当前 DIVA 形状 |
| --- | --- | --- |
| 入站 listener | `DingTalkChannel::start_webhook` 在 `dingtalk_channel.rs:330-380` 创建 `POST /dingtalk/webhook`，绑定 `0.0.0.0:8650`，把 JSON 放入 channel；`Channel::start` 在 `:389-394` 只调用它 | `DingTalkAdapter::start` 在 `src/adapters/dingtalk.rs:1317-1357` 循环 register → WS → backoff；`register_stream` 在 `:572-630` 是 native Stream 主路径 |
| register/auth | Octos 文件没有 OAuth 或 Stream register；`start_webhook` 没有平台连接注册 | DIVA `register_stream` 对 `POST /v1.0/gateway/connections/open` 发送 `clientId`、`clientSecret`、`localIp: 127.0.0.1`、EVENT `*` 和 CALLBACK `/v1.0/im/bot/messages/get` subscription、`ua: agent-diva/0.9.9`（`:572-588`）；成功响应要求 `endpoint` 与 `ticket`（`:601-628`） |
| callback/ACK | Octos webhook 成功只返回 HTTP JSON `{"msgtype":"text","text":{"content":"ok"}}`（`:126-127`）；没有 Stream opcode/ACK。签名错误返回 HTTP 401，坏 JSON 返回 400（`:104-123`） | `run_stream_once` 将 `ticket` 作为 WS query（`:1160-1175`）；`handle_stream_message` 在 `admit_event` 成功后才返回 `Ack`（`:1129-1157`），随后发 JSON ACK `code:200`、`message:"OK"`、原 callback `headers.messageId`、`contentType:"application/json"`、`data:"{}"`（`:1204-1228`）。DingTalk 这里是文本帧/JSON response，不是 Discord 式 opcode。 |
| ping/heartbeat | Octos `start_webhook` 没有 heartbeat；仅以 500 ms 轮询 shutdown flag（`:348-358`） | DIVA 对 WebSocket `Ping` 回 `Pong`（`:1233-1244`），但当前文件没有主动 DingTalk heartbeat timer/frame；capability snapshot 仍声明 `ReliabilityHeartbeat`（`:1293-1297`） |

### 现有 fixture、测试和已记录结果

- `agent-diva-channels/tests/fixtures/c5/dingtalk/token-response.json` 是 `{ "accessToken": "redacted-token", "expireIn": 3600 }`；`stream-callback.json` 是 `specVersion=1`、`type=CALLBACK`、topic `/v1.0/im/bot/messages/get` 的脱敏 callback 形状。
- `stream_register_callback_ack_cancel_and_dedup_are_wire_bound`（`src/adapters/dingtalk.rs:2023-2081`）通过 `FakeHttp` 记录 register path/body，内置 WS server 发同一 callback 两次；断言只产生一次 Fabric envelope、chat/sender/message ID、收到两份 `code=200` ACK，之后 cancel 使 `run_stream_once` 返回成功。
- `shipped_stream_and_media_fixtures_are_parseable`（`:2141-2159`）只解析 fixture 并检查 callback topic、token 字段和 media response 字段；它没有把 `stream-callback.json` 作为实际 WS transcript 驱动。
- `token_is_cached_and_group_payload_uses_open_conversation_id`（`:1843-1872`）和 `authenticated_send_refreshes_once_after_401`（`:1874-1915`）提供补充 OAuth cache/401 证据；既有记录显示 DingTalk 模块共 10 项测试通过，但没有独立的 `start` 多连接测试。

### 生命周期、错误和边界

- timeout/admission：DIVA `STREAM_ADMISSION_DEADLINE` 为 2 秒（`:44-46`、`:806-810`）；生产 reqwest client timeout 为 30 秒（`:347-352`）。Octos webhook 没有对应 request/admission deadline。
- cancel/stop：`run_stream_once` 同时 select `context.cancel` 和 `self.stopped`（`:1177-1183`）；`wait_backoff` 同样可取消（`:1259-1265`）；`stop` 只调用 `self.stopped.cancel()`（`:1430-1433`）。没有测试证明 `start`、register 等待、WS reader、backoff 和 health transition 的完整 stop 链路。
- retry/reconnect：register/WS 失败映射为 retryable execution error；register 的 429 使用 `RateLimited`，并由 `start` 以 5 秒起步、指数增长、最多 60 秒的 backoff 重连（`:1317-1355`）。401/403 的 token refresh 是 `authenticated_json` 的一次重试（`:515-547`）。缺少多次连接、backoff 序列和失败后重新 register 的 fixture transcript，因此这是 evidence gap。
- dedup/admission：`handle_stream_message` 先检查 update header ID，再检查 event ID；`mark_processed` 在 Fabric admission 成功后执行（`:1143-1157`、`:806-812`），busy 映射为 `fabric_busy` 且不 ACK、不提交 marker（`:1452-1465`）。`DedupState` 上限 1,000（`:232-255`）；现有 test 仅覆盖同一 callback 重放，不覆盖 eviction、header/event ID 不同或 busy 重放。
- health：registration/stream 错误设为 `Degraded`，成功退出设为 `Healthy`（`:1323-1355`）；`ProbeHealth` 调用 `/v1.0/robot/info`，成功设 `Healthy`、失败设 `Down`（`:1388-1407`）。Octos `Channel::health_check` 默认 `Unknown`（`channel.rs:241-247`）。没有 DT-01 health wire/test transcript。

### DT-01 剩余缺口与判定

1. `evidence_gap`：缺少 register 429/transport failure、WS close 后重新 register、backoff timing、health `Degraded → Healthy/Down`、stop 同时终止所有等待的 scripted transcript。
2. `evidence_gap`：缺少系统 heartbeat、ping/pong 或平台主动 heartbeat 的实际 WS fixture；当前测试名虽覆盖 callback/ACK/cancel，但没有发送 Ping。
3. `implementation_gap`（取决于冻结 `ReliabilityHeartbeat` 的语义）：DIVA 只处理收到的 WebSocket Ping，不产生主动 DingTalk heartbeat frame/timer；Octos 固定源码也没有 Stream heartbeat，不能用 Octos 补足该证明。
4. `blocked/unsupported`：`ReliabilityResume` 不在 DIVA capability snapshot；不能从 Octos webhook 推断 resume token。该项不是 DT-01 的待验证成功能力。

因此 DT-01 保持 `partial`，不能升级为 `verified`。

## DT-03 — media upload/send

### Octos 与 DIVA symbol 对照

| 项目 | 固定 Octos 形状 | 当前 DIVA 形状 |
| --- | --- | --- |
| text/limit | `DingTalkChannel::send` 在 `dingtalk_channel.rs:396-414` 通过 webhook 发 `POST` JSON `{"msgtype":"text","text":{"content":...}}`，按 `ChunkConfig { max_chars: 3600 }` 分片；`DingTalkChannel::max_message_length` 在 `:430-432` 返回 3,600 | DIVA `MAX_TEXT_CHARS=3_600`（`src/adapters/dingtalk.rs:41`），`send_text` 超限返回 `Execution{code:"text_too_long", retryable:false}`（`:865-877`）；capability 声明 `EgressChunking` false、上限 3,600（`:1299-1300`），与冻结矩阵一致 |
| media | Octos 没有 upload/send media endpoint；`send` 对非空 media 只 warning `not supported by the text robot path` 后仍返回 `Ok(())`（`:416-423`） | DIVA `send_attachment` 先从 AttachmentStore 读回并 validate（`:924-974`），再通过 `/media/upload` multipart 上传，并按 kind 映射 `image`/`voice`/`file`（`:955-974`）；随后发送 native group/private OpenAPI message（`:989-1026`） |
| receipt | Octos `Channel::send_with_id` 默认调用 `send` 后返回 `None`（`channel.rs:78-83`），不能证明平台 ID | DIVA 解析 `messageId/message_id/msgId/processQueryKey/taskId`（`src/adapters/dingtalk.rs:1493-1504`），以 `accepted_receipt` 返回真实 ID；缺 ID 时 Accepted 但不伪造 ID（`:1020-1026`） |

### 精确请求、响应和已有 wire 证据

- upload request：`ReqwestDingTalkHttp::post_multipart`（`:161-190`）构造 `POST https://oapi.dingtalk.com/media/upload?access_token={token}&type={image|voice|file}`，multipart field 为 `media`，携带 filename、声明 MIME 和 bytes。测试 seam `FakeHttp::post_multipart` 在 `:1736-1754` 记录 path、token、kind、filename、MIME 和 bytes。
- image send request：`send_attachment` 对 image 使用 `msgKey: "sampleImageMsg"`、`msgParam: {"photoURL": media_id}`；group 使用 `robotCode + openConversationId`，direct 使用 `robotCode + userIds`，发送 endpoint 分别为 `/v1.0/robot/groupMessages/send` 和 `/v1.0/robot/oToMessages/batchSend`（`:989-1020`）。
- audio/video/file code paths 存在：均使用 `msgKey: \"sampleFileMsg\"`，`fileType` 分别为 `voice`、`video`、`file`，`mediaId` 和 filename 放入 msgParam（`:995-1008`）；但没有每种类型的 wire test。
- success response：要求 upload response 有 `media_id` 或 `mediaId`（`:975-988`），send response 可含 `messageId` 等；最终 receipt 保留平台 ID。
- error response：HTTP 429 转为 `AdapterError::RateLimited{retry_after}`（`:539-543`、`:561-564`）；其它非 2xx 转为 `Execution`，operation code 带 HTTP status（`:1436-1450`）。attachment store/size/validation/upload decode 失败分别返回 typed execution errors（`:930-953`、`:981-987`）。
- partial failure：`media_upload_and_send_preserve_typed_part_and_partial_failure`（`:1918-2020`）用 `[1,2,3]` 的 `image/png` fixture attachment 断言 `/media/upload`、kind `image`、filename `fixture.png`、bytes、private send `sampleImageMsg`、media ID 和最终 `media-message-1` receipt；第二个 script 在 upload 成功后 send 返回 429/7 秒，断言 `RateLimited(7s)` 且没有成功 receipt。该 test 是 direct image，不是 group 或其它 media kind。
- fixture 关系：`media-upload-response.json` 与 `group-send-response.json` 分别只有脱敏 `media_id` 和 `messageId` shape；上述 media test 使用 inline scripted `Value`，fixture 由 `shipped_stream_and_media_fixtures_are_parseable` 单独 parse，不能算完整 multipart HTTP transcript。

### 生命周期、重试、去重、health 和边界

- timeout/cancel：生产 reqwest client 的 HTTP timeout 是 30 秒（`:347-352`），但 `ChannelCommand::execute`/`send_attachment` 没有接收 cancellation token；上传或发送正在等待时没有本地 cancel 证明。DIVA `start` 的 backoff 可 cancel，但这不等于 outbound media cancel。
- retry：`authenticated_json` 对 401/403 会 invalidate token、重新取 token、重发一次（`:515-547`）；`authenticated_multipart` 对 401/403 不做同样 refresh，只把非 2xx 映射为 error（`:550-570`）。429 只返回 `RateLimited`，没有自动重试；已有 test 只证明 send 429 的 typed error。
- idempotency/dedup：`execute` 接收 `idempotency_key`，但 `execute_send`/`send_attachment` 不把它放入 DingTalk request body/header，也没有 outbound dedup store。对 upload 成功、send 失败的重试不能由该 adapter 保证幂等；这是 implementation gap，不能用 inbound dedup 代替。
- partial ordering：`execute_send` 会先 flush 累积 text，再逐个发送 media（`:1045-1117`）；多个 media 时 `receipt` 只保留最后一个发送结果，先前已成功的 side effect 不可回滚。这部分没有多 part/部分失败 transcript。
- size/MIME：capability 上限为 20 MiB，send 前检查 reference size、读回 bytes 长度和 stored validation（`:930-953`）；supported MIME 列表在 `:1301-1313`。现有测试没有断言 MIME 字段、20 MiB boundary、unsupported MIME、group media 或 audio/video/file 的请求。
- health/reconnect/dedup：media 本身是 one-shot HTTP，不产生 stream dedup；全 adapter 的 `ProbeHealth` `/v1.0/robot/info` 和 Stream reconnect 位于 `:1317-1407`，DT-03 没有独立 health/transport retry transcript。

### DT-03 剩余缺口与判定

1. `evidence_gap`：为 `Audio`/`Video`/`File` 各补真实 fixture/request assertion/response ID；同时覆盖 group `openConversationId` media send。
2. `evidence_gap`：补 upload MIME/header/query、大小边界、缺/坏 `media_id`、send 缺 ID、部分 multi-part failure 和最终 receipt 语义的 wire transcript；现有两个 JSON fixture 只是 response shape。
3. `evidence_gap`：补 outbound cancellation、health 失败和 transport timeout 的可复现记录。
4. `implementation_gap`：`authenticated_multipart` 缺 401/403 token refresh；`idempotency_key` 未进入 upload/send，无法证明重试幂等。
5. `blocked/unsupported` 不适用于已声明的四种 media atom；不能因为 Octos text-only 就把 DIVA 的 media 改成 silent success。Octos 的 text-only 行为本身是迁移边界，不能成为 DIVA 证据。

因此 DT-03 保持 `partial`，不能升级为 `verified`。

## DT-04 — HMAC/sessionWebhook safety

### Octos 与 DIVA symbol 对照

| 项目 | 固定 Octos 形状 | 当前 DIVA 形状 |
| --- | --- | --- |
| signature | `dingtalk_signature`/`verify_dingtalk_signature` 在 `dingtalk_channel.rs:34-71`：签名串为 `timestamp + \"\\\\n\" + secret`，HMAC-SHA256 后 standard Base64；空 timestamp/signature 直接 false，比较使用 constant-time `ct_eq` | `dingtalk_signature`/`verify_dingtalk_signature` 在 `src/adapters/dingtalk.rs:1620-1648` 使用同样 key normalization、签名串、Base64 和空值 fail-closed；`constant_time_equal` 在 `:1650-1657` 做长度并入差异的逐字节比较 |
| signed URL | Octos `signed_webhook_url` 在 `:73-79` 先 `Url::parse`，再 append `timestamp` 与 `sign` query；非法 URL 返回 error | DIVA 没有 `signed_webhook_url`。Gate2/迁移决策只采纳 HMAC/session 纯 helper，不接入 Octos text-only webhook，因此这是有意 `Reject`，不是本行应补的 fallback |
| target/cache | Octos `target_webhook` 在 `:169-206` 按 metadata session URL → chat_id URL → cached session URL → configured webhook 选择，并对 configured URL 加签；Octos cache 是无 TTL 的 `HashMap`（`:130-156`） | DIVA `cache_session_webhook`/`cached_session_webhook` 在 `src/adapters/dingtalk.rs:829-863` 只接受 HTTPS DingTalk host，容量最多 1,000，TTL 3,600 秒；`send_text` 只读取并丢弃 cached value（`:879`），继续走 native OpenAPI，不把 webhook 当发送 fallback |
| inbound webhook | Octos `handle_webhook` 在 `:88-128` 校验 headers `timestamp` + `sign`/`signature`，失败 HTTP 401；合法 JSON 入队，坏 JSON HTTP 400；`start_webhook` 在 `:330-380` 提供 `/dingtalk/webhook` | DIVA 没有 `handle_webhook`/`start_webhook`；入站仅是已注册 Stream callback。`parse_event`（`:632-693`）从 Stream data 提取并校验可信 session URL，`admit_event` 在 `:800-805` 将其放入 envelope/cache |

### 现有 fixture、测试和已记录结果

- `stream-callback.json` 含脱敏 `sessionWebhook`：`https://oapi.dingtalk.com/robot/sendBySession?session=redacted`；它证明字段 shape，不携带可验证的 signed callback header/query。
- `signature_matches_octos_shape_and_fails_closed`（`src/adapters/dingtalk.rs:1820-1829`）断言同一固定 timestamp/secret 生成的 signature 可验证、`bad` 失败、空 timestamp 失败。它是纯函数测试，不调用 HTTP webhook，也没有独立签名 fixture。
- `shipped_stream_and_media_fixtures_are_parseable`（`:2141-2159`）只验证 stream/token/media JSON 可解析，不能证明 `target_webhook` 选择、URL signing 或 callback header 到 handler 的全链路。
- 既有记录 `verification.md:11-20` 显示该测试集合通过；本次不将 recorded pass 扩张解释为 TTL 或 signed callback 证据。

### 安全、生命周期和边界

- HMAC：DIVA 与 Octos 的纯计算形状一致；DIVA 空 timestamp/signature 返回 false，错误诊断不包含 secret。DIVA Stream 主路径没有 HMAC header，因为它不是 Octos webhook handler。
- URL trust：Octos `target_webhook` 会直接接受 metadata session URL 和 `http://`/`https://` chat ID（`:169-182`），`parse_event` 只要求 sessionWebhook 非空（`:280-289`）；DIVA 则在 `parse_event` 和 cache helper 使用 `is_trusted_dingtalk_url`，仅允许 HTTPS 且 host 为 `dingtalk.com` 或其子域（`:1483-1491`、`:674-678`、`:829-831`）。这是 DIVA 的安全收紧。
- TTL/eviction：DIVA cache entry expires after 3,600 秒，读取时清理过期 entry，并在 1,000 个 conversation 后淘汰一个旧 key（`:837-862`）。没有可控 clock 或 test 证明过期后不能再取到 URL，也没有 eviction transcript。
- ordering：`admit_event` 在 Fabric admission 之前 cache session URL（`:800-805`），而 dedup marker 在 admission 成功后才提交（`:806-812`）。如果 Fabric busy，`fabric_busy` 不 ACK/不提交 dedup，但当前代码没有针对“busy 后 session cache 是否应回滚”的证据。
- target/fallback：DIVA 当前 `send_text` 通过 `/v1.0/robot/groupMessages/send` 或 `/v1.0/robot/oToMessages/batchSend` 发送；cache 变量不改变 target。由此不会复制 Octos 的无 TTL、任意 metadata URL 或 text-only webhook fallback。若未来要求完整 Octos webhook compatibility，应先提交 decision request，不能在此行静默添加。
- timeout/cancel/retry/dedup/health/reconnect：DT-04 是纯 HMAC/cache 辅助，不拥有独立网络生命周期；Stream 的 timeout/cancel/retry/dedup/health 归 DT-01，OpenAPI error/receipt 归 DT-03。Octos `Channel::health_check` 默认 `Unknown`（`channel.rs:241-247`），也没有为 HMAC helper 提供健康探针。

### DT-04 剩余缺口与判定

1. `evidence_gap`：补 TTL expiry、cache miss/eviction 和 trusted/untrusted URL 的直接断言；现有代码有 TTL/trust 分支，但没有相应通过记录。
2. `evidence_gap`：补真实形状的 signed callback transcript（脱敏 timestamp/signature/header 与 HTTP 401/400/accepted response），或明确记录该 webhook path 被 C5 决策拒绝而不纳入 DIVA 主路径。
3. `evidence_gap`：补 `verify_dingtalk_signature` 的长 secret、signature 长度不等、`sign`/`signature` header 选择等边界证明；Octos 只有 `sign` 优先、`signature` fallback 的 handler 代码（`:94-104`）。
4. `implementation_gap` 不应从“没有 `signed_webhook_url`/`start_webhook`”推导：按 D-008 和本页迁移边界，DIVA 明确保留 Stream/OpenAPI、拒绝 Octos text-only webhook fallback；若产品改要 webhook compatibility，需新的决策请求。

因此 DT-04 保持 `partial`，不能升级为 `verified`。

## 审计退出声明

- DT-01、DT-03、DT-04 均已有可复现的局部代码/fixture/test 证据，但没有满足“代码 + fixture + 请求断言 + 响应/receipt 或 typed error + 所需生命周期”的完整 manifest 规则。
- 本轮没有声称任何未执行的测试、编译、clippy、live network 或平台验证已通过；没有修改其他文件。
