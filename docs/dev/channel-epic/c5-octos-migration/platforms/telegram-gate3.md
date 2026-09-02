# Telegram C5-V Gate 3 evidence — partial capability audit

状态：`partial`。本页是 Telegram 审计 worker 对 TG-02～TG-06 的只读复核，基于
DIVA 当前 `feat/channel-epic@2b3ef682` 与固定 Octos checkout：

```text
Octos: C:\Users\Administrator\Desktop\morediva\.workspace\octos
SHA:   5ea987813de4fd2afdd1d78f2106ad2868f0d923
```

本轮只修改本文件；没有编译、运行测试、执行 clippy、访问 live network 或修改 adapter、
fixture、测试、manifest、JSON、TODO、LOCK。下文的“既有通过记录”来自
`docs/logs/2026-09-channel-epic/v0.2.2-c5-capability-evidence/verification.md`，本 worker
没有重新执行这些命令。

## Audit summary

| ID | 冻结矩阵目标 | Octos stable source anchor | DIVA native anchor | 结论 |
| --- | --- | --- | --- | --- |
| TG-02 | `IngressGroup=T`, `IngressDirect=T`, `IngressThread=T`; command/mention gating 是目标行为 | `crates/octos-bus/src/telegram_channel.rs:80-98, 261-331` | `agent-diva-channels/src/adapters/telegram.rs:504-621` | `partial` — `implementation_gap` + `evidence_gap` |
| TG-03 | `IngressTypedAttachments=T`, `EgressImage/Audio/Video/File=T` | `crates/octos-bus/src/telegram_channel.rs:123-138, 333-408, 525-617`; `crates/octos-bus/src/media.rs:24-92` | `agent-diva-channels/src/adapters/telegram.rs:533-596, 624-696, 790-825` | `partial` — `implementation_gap` + `evidence_gap` |
| TG-04 | `EgressCard=F`; callback ACK/keyboard wire behavior remains separately required, without advertising cards | `crates/octos-bus/src/telegram_channel.rs:213-243, 449-496` | `agent-diva-channels/src/adapters/telegram.rs:438-501`; no native keyboard parser | `partial` — `implementation_gap` + `evidence_gap` |
| TG-05 | `EgressText/Markdown/Chunking/Reply/Image/Audio/Video/File=T`; `InteractionTyping/Listening/Edit/Delete/StreamFinalize=T` | `crates/octos-bus/src/telegram_channel.rs:141-172, 525-637, 648-697, 700-763`; `crates/octos-bus/src/channel.rs:71-162` | `agent-diva-channels/src/adapters/telegram.rs:699-917, 979-1017` | `partial` — `implementation_gap` + `evidence_gap` |
| TG-06 | `IngressDedupId=T`, `ReliabilityHealth/Pacing/SupervisedRestart=T`; `ReliabilityHeartbeat/Resume/TokenRefresh=F` | `crates/octos-bus/src/telegram_channel.rs:261-282, 290-313, 503-523, 769-773`; `crates/octos-bus/src/dedup.rs:19-74` | `agent-diva-channels/src/adapters/telegram.rs:433-455, 935-976, 919-923, 1032-1040` | `partial` — `implementation_gap` + `evidence_gap` |

没有任何一行满足“源码、fixture、passing test、精确请求/响应及所需生命周期全部齐全”的
`verified` 规则；manifest/JSON 应继续保持现有状态。

## TG-02 — group mention/reply/command gating

- **Octos 对照：** `crates/octos-bus/src/telegram_channel.rs` 的
  `should_respond_in_group`（80-98）对 DM/non-gated group 放行，对 gated group 只接受
  reply-to-bot、`@username` 或以 `/` 开头的 command；`start` 的 message 分支
  （261-331）在媒体下载前执行该 policy，并将 mention 从模型文本中移除。sender compound
  ID 的组装与入站消息 ID 位于 415-447。长轮询入口是同文件的 `polling_default`（261-283）。
  `crates/octos-bus/src/channel.rs:21-22` 同时把 `start` 定义为持续运行的 listener。
- **DIVA 对照：** `TelegramAdapter::process_message`（`agent-diva-channels/src/adapters/telegram.rs:504-621`）
  只在 509-520 做 sender ID/username allowlist；它不读取 `TelegramChat.kind` 来决定 group
  gating，不检查 `@bot`、reply-to-bot 或 `/command`，也没有 bot username/mention 配置。
  `chat.kind` 只在 611-615 写入 extension；`message_thread_id` 和 reply message ID 在
  600-609 进入 address/correlation。媒体读取发生在 533-596，allowlist 之前的权限顺序是
  正确的，但 mention/command policy 缺失。
- **既有 fixture/test：** `agent-diva-channels/tests/fixtures/c5/telegram/inbound-group-media.json`
  是脱敏的 `update_id=70002`、message `92`、sender `43`、supergroup `-1001234`、
  `message_thread_id=17`、reply `88`、caption、photo 和 document。既有测试
  `adapters::telegram::tests::wire_telegram_ingress_preserves_identity_media_and_callback_ack`
  （`src/adapters/telegram.rs:1217-1312`）断言 chat/thread/reply 和 typed parts；既有验证
  记录报告 Telegram adapter 共 7 tests passed，但没有 mention/no-mention、reply-to-bot、
  `/command` 或 `getUpdates` listener transcript。
- **请求/响应/receipt：** 本行的 mock 只覆盖由 `process_update` 直接触发的 `getFile`/文件
  下载请求；没有 `POST /bottest-token/getUpdates` 的请求断言，也没有 Fabric busy/cancel
  结果。现有 ingress 成功产出 `ChannelEnvelopeV1`，但群 gating 的拒绝路径没有 typed
  result/receipt 证据。
- **生命周期判定：** `get_updates` 在 `433-436` 固定发送 `offset`、`timeout=25` 和
  `allowed_updates=["message","callback_query"]`；其 listener 取消/重试属于 TG-06，未在
  本行证明。DIVA 的 `process_message` 本身是 `implementation_gap`（目标 policy 未实现），
  同时是 `evidence_gap`（全量矩阵和长轮询 wire 未测），所以保留 `partial`。

## TG-03 — typed media ingress and media egress

- **Octos 对照：** `TelegramChannel::download_telegram_file`
  （`crates/octos-bus/src/telegram_channel.rs:123-138`）先调用 `get_file(file_id)`，再以
  file path 构造 `/file/bot{token}/{path}`，文件名使用 Telegram `unique_id` 加扩展名。
  `start` 的媒体分支（333-408）覆盖 photo、voice、audio、document；该固定 Octos 文件没有
  video 分支。发送分流在 525-617：`.ogg/.oga/.opus` → `send_voice`，`.mp3/.wav/.m4a` →
  `send_audio`，其余 → `send_document`；首个媒体可带 caption/reply。共享
  `crates/octos-bus/src/media.rs:24-92` 的 `download_media` 在读取前检查 HTTP 成功和
  Content-Length，并对流体施加 cap，超限不写入目标文件；默认 cap 是 50 MiB（10-21）。
- **DIVA 对照：** `process_message`（`agent-diva-channels/src/adapters/telegram.rs:533-596`）
  选最大 photo，并解析 voice/audio/video/document；`fetch_attachment`（624-696）先按
  声明 size 做 20 MiB 检查，再调用 `getFile` 和文件 GET，最后调用
  `AdapterServices.attachments.put`。但超限分支在 632-633、662-663 返回 `Ok(None)`，不是
  `AttachmentStoreError::TooLarge` 或其它 typed failure；没有 Content-Length 预检、MIME
  响应校验，也没有显式的单媒体 30 秒 timeout。附件的 `platform_message_id` 在 673-675
  填的是 `file_id`，不是携带媒体的 message ID；sender_id 在 675 为 `None`。
  `execute_send`（717-825）把 Image/Audio/Video/File 分别映射到 `sendPhoto`/`sendAudio`/
  `sendVideo`/`sendDocument` multipart，但 native adapter 没有 Octos 所需的
  `send_voice` 分流；`ContentPart::Audio` 无 voice/audio 区别。
- **既有 fixture/test：** `inbound-group-media.json` 仅覆盖 photo + document；
  `api-responses.json` 仅提供静态 `get_file`、`send_message`、429 数据，而且命名的 ingress
  /multipart 测试使用内联 response literal，并没有加载该 JSON。既有测试
  `wire_telegram_ingress_preserves_identity_media_and_callback_ack`（1217-1312）断言
  `getFile`/photo GET/document GET 的顺序、thread/reply 和 parts；
  `wire_telegram_multipart_and_rate_limit_are_truthful`（1364-1438）只断言一个
  `sendPhoto` multipart 的路径、文件名、caption 和 response `message_id=94`。既有 verification
  记录报告 Telegram adapter 7 tests passed，但没有 voice/audio/video/document 逐类矩阵、
  timeout/oversize/MIME/corrupt read-back 或 AttachmentStore `get` 验证。
- **请求/响应/receipt：** 已有 ingress mock 请求为 `POST /bottest-token/getFile`、
  `GET /file/bottest-token/photos/photo.png` 和 document 对应路径，均返回 200；outbound
  mock 为 `POST /bottest-token/sendPhoto` multipart，JSON response 为
  `{"ok":true,"result":{"message_id":94}}`，receipt 的 platform ID 为 `94`。测试用的
  Telegram `Store::get`（`src/adapters/telegram.rs:1159-1167`）返回空 bytes，multipart
  测试未断言真实 attachment bytes、MIME、大小、SHA-256 或读回损坏，因此不能作为完整
  content-addressed wire proof。
- **生命周期判定：** allowlist 在下载前，且成功 admission 之前不会发布 envelope，这一点
  是保留证据；但超限 silent `Ok(None)`、响应 MIME 未验证、媒体来源 ID 不正确、缺 voice
  分支和逐类失败/存储读回均为 `implementation_gap` 或 `evidence_gap`。仍为 `partial`。

## TG-04 — callback ACK and inline keyboard

- **Octos 对照：** `parse_inline_keyboard`（`crates/octos-bus/src/telegram_channel.rs:213-243`）
  读取 `metadata.inline_keyboard` 的二维 rows，每个按钮要求 `text` 与 `callback_data`，
  生成 `InlineKeyboardMarkup`；`start` 的 callback 分支（449-496）先调用
  `answer_callback_query(callback.id)`，然后在 allowlist 通过后把 callback data、source
  chat/message ID 发布为独立 inbound event。Octos shared channel contract 对 metadata-aware
  edit 的默认边界在 `crates/octos-bus/src/channel.rs:153-163`。
- **DIVA 对照：** `process_update`（`agent-diva-channels/src/adapters/telegram.rs:438-455`）
  以 update ID 去重；`process_callback`（457-501）保留 callback ID/data、source message ID
  和 thread ID，并在 Fabric admission 成功后才调用 `POST /bot{token}/answerCallbackQuery`
  （495-500）。但 native `TelegramAdapter` 中没有 `parse_inline_keyboard`，
  `execute_send`（717-851）也没有读取 envelope metadata 或发送 `reply_markup`；因此
  outbound inline keyboard 是 `implementation_gap`。此外 callback 无 source message（462-464）
  或未通过 allowlist（465-467）会直接 return，不执行 ACK，与 Octos 的 callback-first ACK
  顺序不同。
- **既有 fixture/test：** `agent-diva-channels/tests/fixtures/c5/telegram/callback-query.json`
  提供 callback `callback-9`、sender `42`、private chat `42`、source message `91` 和
  data `approve:request-9`。既有测试
  `wire_telegram_ingress_preserves_identity_media_and_callback_ack`（1290-1311）断言
  callback envelope 的 `telegram.callback_id`/`telegram.callback_data`，以及最后一个
  request 的 `POST /bottest-token/answerCallbackQuery` 路径；它没有断言 request body 的
  `callback_query_id`、source message/thread、重复 callback 或 keyboard outbound。
- **请求/响应/receipt：** adapter 代码构造的 ACK body 是
  `{"callback_query_id":"callback-9"}`；inline keyboard 应有的 `reply_markup` 请求没有
  任何 native fixture。既有 mock ACK response 是 `{"ok":true,"result":true}`，但 callback
  path 没有单独 DeliveryReceipt；Fabric admission 失败时也没有 ACK response 证据。
- **生命周期判定：** update-level dedup 只有实现路径，没有 callback duplicate/busy/cancel
  证明；authorized callback 是 admission-before-ACK，ACK failure 时 update 不提交 seen，
  但 admission 已成功时重放可能再次 admission。由于 keyboard 缺失、部分 ACK policy 与
  request/body/重复矩阵缺失，保持 `partial`。

## TG-05 — send/reply/edit/delete/typing and stream finalization

- **Octos 对照：** `send_html_with_fallback`（`crates/octos-bus/src/telegram_channel.rs:141-172`）
  首次以 HTML + 可选 `ReplyParameters` 发送，解析错误后以 plain text 重试；`send`
  （525-637）处理媒体和首项 caption/reply。Octos `Channel` 的 `send_with_id` 默认在
  `crates/octos-bus/src/channel.rs:78-83` 只委托 `send` 并返回 `None`；edit/delete/finalize
  contract 在 85-162。Telegram 具体 action 是 `send_chat_action` 的 typing/record voice
  （648-667），edit/delete 是 700-763。
- **DIVA 对照：** `send_text_message`（`agent-diva-channels/src/adapters/telegram.rs:699-715`）
  将 `telegram_parse_error` 的 HTML 请求重试为无 `parse_mode` 的 `sendMessage`；
  `execute_send`（717-851）按 4096 字符分片，只给第一块/第一项媒体加
  `correlation.reply_to`，按真实 response `message_id` 生成 receipt。`execute_edit`/
  `execute_delete`（853-886）发送 `editMessageText`/`deleteMessage`；
  `execute_typing`（888-917）将 Started/Listening 映射为 `typing`/`record_audio`，Stopped
  直接返回 Accepted、无网络调用；`FinalizeStream`（`979-1017`）有绑定 message ID 时
  edit，否则重新 send。
- **既有 fixture/test：** `wire_telegram_egress_uses_html_reply_real_id_and_plain_fallback`
  （`src/adapters/telegram.rs:1314-1362`）断言两次 `POST /bottest-token/sendMessage`：第一次
  包含 `parse_mode=HTML` 与 `reply_parameters.message_id=91`，返回 400
  `can't parse entities`；第二次不含 parse mode、text 为 `hello`，返回 message ID `93`，
  receipt 为 `Accepted/93`。`wire_telegram_multipart_and_rate_limit_are_truthful`（1364-1438）
  证明 `sendPhoto` ID `94` 与 body 中 caption/file name；同一测试证明 429 body
  `parameters.retry_after=2` 映射 `RateLimited(2s)`。既有 verification 记录报告 Telegram
  adapter 7 tests passed。
- **请求/响应/receipt：** 已证明的确切 wire 是上述 `sendMessage` HTML→plain fallback、
  `sendPhoto` multipart 和 JSON response IDs `93/94`，以及 429 JSON；没有
  `sendVoice`/`sendAudio`/`sendVideo`/`sendDocument` 的逐类请求/响应。`call_json`/
  `call_multipart`（334-430）只读取 JSON body 的 `retry_after`，不读取 HTTP `Retry-After`
  header。多 chunk/多媒体中间成功后失败时直接返回 typed `Execution`，没有 partial-delivery
  receipt 或已发送 ID 的审计结果。
- **生命周期判定：** edit/delete/typing/finalize 都只有代码路径，没有本地 transport spy
  transcript；typing/listening 是一次性 action，没有 Octos 风格的可取消刷新任务；没有
  partial chunk failure、幂等重试或跨 thread edit 证明。故存在 `implementation_gap`（action
  refresh/partial receipt/keyboard media coverage）与 `evidence_gap`，保持 `partial`。

## TG-06 — polling, reconnect, dedup, health and stop

- **Octos 对照：** `polling_default` 初始化于
  `crates/octos-bus/src/telegram_channel.rs:261-283`；stream item 以 `last_update_id`
  在 304-313 去重，随后处理 message/callback，stream 结束后 503-518 以 5 秒起步、指数
  增长至 60 秒重连。其 stop 检查在 278-280、291-294、503-506，但 backoff sleep（518）
  本身不监听取消。authenticated health 是 `health_check`（769-773）调用 `get_me`，成功
  Healthy、失败 Down。共享 dedup 参考 `crates/octos-bus/src/dedup.rs:19-74` 是容量 1000、
  TTL 60 秒、空 ID 不 dedup，并提供 `forget` 以便 admission 失败后允许重试。
- **DIVA 对照：** `get_updates`（`agent-diva-channels/src/adapters/telegram.rs:433-436`）
  发送 `POST /bot{token}/getUpdates` JSON：当前 atomic offset、`timeout=25`、
  `allowed_updates=["message","callback_query"]`。`start`（935-976）以 context/local
  `CancellationToken` 的 `select!` 包住 HTTP 请求，失败固定等待 2 秒且可取消；成功时设置
  Healthy，失败时设置 Degraded。`process_update`（438-455）在 Fabric admission 成功后
  才写入 `seen` 与 `offset=update_id+1`，因此单条 busy/cancel 不会立即提交 dedup；但
  同一 `getUpdates` batch 中较早 update 失败、较后 update 成功时，后者会推进 offset，较早
  update 可能被永久跳过，这是 `implementation_gap`。
- **既有 fixture/test：** `agent-diva-channels/tests/fixtures/c5/telegram/api-responses.json`
  没有 `getUpdates` 或 `getMe` response。唯一相关既有测试是
  `unsupported_reaction_is_rejected_before_transport`（`src/adapters/telegram.rs:1198-1215`），
  证明 false `InteractionReaction` 在首次 HTTP 前返回 `UnsupportedCapability`；没有
  polling request/offset/timeout、cancel-in-flight、stream/retry、重复 update、Fabric busy
  或 `getMe` healthy/auth-failure fixture。既有 verification 记录中的 7 个 Telegram tests
  不包含这些生命周期断言。
- **请求/响应/receipt：** 代码预期的 polling 请求为上述 `getUpdates` JSON，但没有 local
  server capture；health 请求为 `POST /bot{token}/getMe`，成功后 receipt 是
  `Accepted(channel=telegram, chat_id=health, platform_message_id=None)`（919-923）。失败
  会透传 `AdapterError::Execution`，`probe_health` 不把本地 health snapshot 更新为 Down，
  且没有对应 typed health response transcript。
- **生命周期判定：** DIVA 的 `seen` 是无界 `HashSet<i64>`（201-202），没有 Octos shared
  dedup 的 TTL/capacity/forget 语义；reconnect/backoff 为固定 2 秒而非 Octos 5→60 秒；
  cancel-aware request/backoff 与 stop token 已在代码中，但未测试。`ReliabilityPacing` 与
  `ReliabilitySupervisedRestart` 还需要 C2 runtime/supervisor 装配证据，不能由本 adapter
  静态声明替代。故 TG-06 保持 `partial`。

## Redaction and disposition

fixture 中只使用 redacted token、数字/截断 ID 和本地 mock path；现有 test request capture
没有把真实 token、signed URL 或原始媒体写入交付文档。图片在 DIVA 中作为 typed
`ContentPart::Image` 进入 Fabric，vision provider 仍不由 channel 伪造。

最终 disposition：TG-02、TG-03、TG-04、TG-05、TG-06 全部保持 `partial`；本审计不升级
`evidence-manifest.md` 或 `capability-evidence.json`，不将缺失的实现/生命周期证据解释为
verified，也不新增 QQ D-013/D-014 结论。
