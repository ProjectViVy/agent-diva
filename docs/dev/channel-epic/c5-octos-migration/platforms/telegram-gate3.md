# Telegram C5-V Gate 3 evidence — partial capability audit

状态：`partial`。本页记录 Telegram adapter 在本 worktree 中对 TG-02～TG-06 的本地可验证
修复。实现只使用允许的 native adapter、Telegram fixture、adapter 内测试和本页；没有改动
公共 channel contract、manifest、共享 TCK 或 C6 Manager。

对照源码固定为：

```text
Octos: C:\Users\Administrator\Desktop\morediva\.workspace\octos
SHA:   5ea987813de4fd2afdd1d78f2106ad2868f0d923
```

## Audit summary

| ID | 本地源码 symbol | wire fixture / passing test | 结论 |
| --- | --- | --- | --- |
| TG-02 | `should_respond_in_group`, `process_message`, `load_bot_identity` | `group-policy.json`; `group_policy_accepts_dm_reply_mention_and_command_only` | `partial` — policy 有本地证据；无 live Telegram、C6 listener/supervisor 证据 |
| TG-03 | `fetch_attachment`, `resolve_media_mime`, `audio_method` | `media-matrix.json`, `api-responses.json`; media matrix、typed failure、timeout/cancel、store corruption tests | `partial` — 本地 bounded/store wire 完整；无真实平台和生产 AttachmentStore 装配证据 |
| TG-04 | `process_callback`; `EgressCard` 仍不在静态 capabilities | `callback-query.json`, `keyboard-unsupported.json`; ACK-first/dedup 和 unsupported tests | `partial` — callback ACK 已证明，但 keyboard/reply_markup 仍明确 unsupported，不能升级 verified |
| TG-05 | `execute_send`, `execute_typing`, `refresh_chat_action`, `execute_edit`, `execute_delete` | `api-responses.json`; egress/media/partial-retry/typing/lifecycle tests | `partial` — 请求、receipt、Retry-After 和生命周期有本地 evidence；无 live delivery/idempotency runtime 证据 |
| TG-06 | `process_batch`, `get_updates`, `start`, `probe_health`, `remember_updates` | `polling-responses.json`, `api-responses.json`; polling/dedup/health lifecycle tests | `partial` — batch offset、TTL/capacity/forget、cancel/backoff/Down 有本地 evidence；C6 pacing/supervision 仍未证明 |

`verified` 需要真实平台形状的 mock transcript、请求断言、响应解析、receipt/typed error 和
相关 timeout/cancel/retry/dedup/health/lifecycle 证据；本页仍保守保留所有 TG-02～TG-06 为
`partial`。本轮不升级共享 capability manifest 或 JSON。

## TG-02 — group reply policy

Octos `crates/octos-bus/src/telegram_channel.rs` 的
`should_respond_in_group`（固定 checkout 中约 80-98）对 DM 放行，在 gated group 中仅接受
reply-to-bot、精确 `@username` 或 leading `/` command，并在媒体读取前做判断。

Telegram native adapter 现在由 `load_bot_identity` 调用 `getMe` 缓存 bot username；
`process_message` 先执行数字/username allowlist，再由 `should_respond_in_group` 检查
`private`、`group/supergroup`、reply 的 nested `from.is_bot`、mention 和 command。群 mention
在 admission 前从模型文本中移除；普通群消息不会触发 `getFile` 或 Fabric ingress。成功事件
仍保留 `chat_kind`、message/thread/reply correlation 和 sender identity。

`group-policy.json` 覆盖一个 DM、一个应忽略的普通 supergroup 消息、reply-to-bot、
`@testbot` mention 和 `/status` command。测试
`group_policy_accepts_dm_reply_mention_and_command_only` 断言只 admission 四个 envelope、
mention 被清理、普通群消息无 ingress，并断言 batch offset 在成功后推进。生产 `start`/health
probe 会先加载 bot identity；若仅直接调用私有处理函数而未加载 identity，未知 username 的
mention 会保守忽略，但 reply/command 规则仍不扩大。

仍为 gap 的部分是 live Telegram 群权限/实际 update shape、C6 listener/supervisor 装配，及
真实账号 username 变更场景。没有把这些推断为平台 verified。

## TG-03 — typed media ingress and media egress

Octos 对照包括 `get_file` 后按 file path 下载、bounded stream，以及 `.ogg/.oga/.opus` 使用
voice、其它已证实音频使用 audio 的发送分流。DIVA native adapter 的
`fetch_attachment` 在 allowlist/group policy 和 admission 前执行：

- Telegram declared/file response size 与 response `Content-Length` 都受 20 MiB bound；
  流读取再做 cap，超限返回 `Execution` code `telegram_media_too_large`，不再静默丢弃。
- response HTTP status、`Content-Type` 与 declared MIME 经过
  `resolve_media_mime`；缺失/冲突/不匹配返回 `telegram_media_mime`，缺 path 返回
  `telegram_media_missing_path`，非成功下载返回 `telegram_media_http`。
- 单媒体操作由 30 秒 timeout 包住，并同时监听 request/local cancellation；超时为可重试的
  `telegram_media_timeout`，取消为 `AdapterError::Stopped`。
- `IngressAttachment` 带 Telegram message ID、sender ID、file name 和有效 MIME，交给
  `AdapterServices.attachments.put` 后立即 `get` 并 `StoredAttachment::validate`。读回损坏是
  typed `attachment_store_corrupt`；adapter 没有自己伪造第二套存储契约。
- 出站 `ContentPart::Image/Audio/Video/File` 读取并校验 store bytes/MIME，分别使用
  `sendPhoto`、`sendVoice`/`sendAudio`、`sendVideo`、`sendDocument`，并从 Telegram result
  message ID 生成 receipt。

`media-matrix.json` 的 voice/audio/video/document 四种 Telegram message shape 通过本地
HTTP fixture，各自断言 `getFile`、文件 GET 顺序、Content-Type、文件名、size 和 typed
`ContentPart`。media failure test 覆盖 declared oversize、missing path、503 status 和 MIME
mismatch；另有 hold-open fixture 覆盖 timeout/cancel，腐坏 Store 覆盖 read-back validation。
出站测试断言四个 multipart endpoint/field/file name，并解析
`api-responses.json` 中的 `send_voice`、`send_audio`、`send_video`、`send_document` response。

本地 fixture 使用 `fixture-photo` 等脱敏 bytes；没有声称 Telegram CDN、真实 MIME、生产
AttachmentStore 或视频能力已获得 live 证据。生产 AttachmentStore/C6 注入也不在本频道允许
范围内，因此 TG-03 保持 `partial`。

## TG-04 — callback ACK and inline keyboard boundary

Octos callback 路径要求先 `answerCallbackQuery(callback.id)`，再进行 allowlist/source
处理；inline keyboard 是它自己的 metadata parser 和 `reply_markup` 发送路径。

`process_callback` 现在先发送：

```json
{"callback_query_id":"callback-9"}
```

成功 ACK 后才 admission callback envelope，并保留 callback ID/data、source message ID 和
thread ID。`callback_ack_precedes_admission_and_duplicate_update_is_deduped` 使用
`callback-query.json` 和 `api-responses.json.callback_ack`，断言 ACK request body、成功
ingress、相同 update 的第二次处理无第二个 ACK/ingress。

`keyboard-unsupported.json` 是真实形状的 `ContentPart::Card`，只用于证明
`ChannelCapability::EgressCard` 在 transport 前返回 `UnsupportedCapability`。adapter 没有
读取 card body 的私有 `reply_markup` side channel，也没有把 callback ACK 当成 keyboard
发送能力。因此 TG-04 绝不能因 callback ACK 成功而标为 `verified`；keyboard/reply_markup
仍是明确 gap。

尚未在本地证明真实 Telegram callback 超时/过期行为、allowlist 拒绝后的平台响应和 C6
admission busy/cancel 集成；这些也不被本页宣称为完成。

## TG-05 — send/reply/edit/delete/typing/finalize

Octos 的 HTML fallback、首个 send unit 的 reply parameters、typing/record action、edit/delete
和 stream finalize 是本轮 wire 对照面。DIVA native adapter 现有证据为：

- `send_text_message` 在 Telegram parse error 后移除 HTML parse mode 并以 plain text 重试；
  `execute_send` 以 Unicode-safe 4096 字符分片，首块/首个媒体使用 numeric
  `reply_parameters.message_id`，receipt 取真实 Telegram result ID。
- multipart 发送前读取并校验 attachment；voice/audio/video/document 分流有独立 endpoint。
- 中间 unit 已成功而后续 unit 返回错误时，`partial_delivery_receipt` 返回 `Failed`、
  `partial_delivery`、已发送的最后 platform ID、delivered/total diagnosis 和 retry delay。
  HTTP 429 优先使用 `Retry-After` header，缺 header 才回退到 Telegram JSON parameters。
- `execute_typing` 立即发送 `typing`/`record_audio`，再由可取消 refresh task 定期发送 action；
  `Stopped`/adapter stop 会取消 task。`execute_edit`、`execute_delete` 和带 message ID 的
  `FinalizeStream` 发送 numeric target ID，并返回 accepted receipt。

`wire_telegram_egress_uses_html_reply_real_id_and_plain_fallback` 断言 HTML→plain 两次
`sendMessage`、numeric reply `91` 和 receipt `93`。`egress_media_uses_telegram_typed_methods_and_readback_bytes`
断言 `sendVoice`/`sendAudio`/`sendVideo`/`sendDocument` multipart fields 和 response IDs。
`partial_chunk_failure_returns_receipt_and_prefers_retry_after_header` 让第一块成功、第二块
返回 HTTP 429 + `Retry-After: 7`，断言 `Failed/partial_delivery`、ID `401` 和 `retry_after_ms=7000`。
`typing_refreshes_until_stopped`、`edit_delete_and_finalize_use_numeric_message_ids` 提供
action refresh/cancel、edit/delete/finalize transcript。

这些测试证明的是请求构造、解析和本地 receipt；没有声称 outbound idempotency key 已接入
Telegram API 去重，也没有真实送达、网络重试后端或跨账号/thread 的 live 证据。故 TG-05
保持 `partial`。

## TG-06 — polling, reconnect, dedup, health and lifecycle

Octos 对照包括 `getUpdates` long polling、update ID 去重、5 秒起步至 60 秒的 reconnect
backoff、`getMe` health 和 stop 生命周期。DIVA native adapter 现在：

- `get_updates` 发送 `POST /bot{token}/getUpdates`，body 含 atomic offset、`timeout:25` 和
  `allowed_updates: ["message","callback_query"]`。
- `process_batch` 只有在整个 batch 的每一个新 update 成功处理后才写入 seen 并把 offset
  推进到 `max(update_id)+1`；中间失败会忘记本批早先处理的 IDs，允许平台重试。
- `seen` 是 TTL 60 秒、capacity 1000 的 bounded map，另有 `forget_update`；listener/backoff
  使用 5、10、20、40、60 秒并在 backoff 与 HTTP poll 同时监听 cancellation。
- `probe_health`/listener identity failure 设置 health `Down`；成功 batch 设置 `Healthy`，
  stop/cancel 结束时设置 `Down`。这不改变静态 `ReliabilityPacing`/
  `ReliabilitySupervisedRestart` 声明仍需 C6 runtime 证据的事实。

`polling-responses.json` 提供 `getMe`、empty batch、message batch 和 unauthorized shape。
`polling_batch_advances_offset_only_after_a_successful_batch` 让同一 batch 的首个 text 成功、
第二个 media 缺 path，断言 offset 仍为 0；随后 captured `getUpdates` request 明确包含
`"offset":0`, `"timeout":25`，成功重试后 offset 才变为 73002。dedup test 断言重复、
TTL expiry、forget、capacity bound 和 5→60 backoff。health/lifecycle test 断言 unauthorized
`getMe` 返回 typed error 并将 snapshot 设为 `Down`，另以 hold-open `getUpdates` 证明 start
可取消且最终 `Down`。

仍没有 live Telegram long-poll transcript、平台连接断开后的真实 retry、空 update ID 语义、
跨进程 dedup、C6 pacing 或 supervisor restart 证据；不能将静态 capability 当成这些行为的
完成证明。

## Redaction and disposition

所有 fixture token、ID、媒体 bytes 和 HTTP endpoint 都是脱敏/本地 mock；不会写入真实 token、
signed URL 或平台数据。没有添加公共/私有 keyboard side channel，没有新增 config key，也
没有修改共享契约。

本次 focused command：

```text
cargo test -p agent-diva-channels adapters::telegram::tests --lib -- --test-threads=1
```

结果：`21 passed; 0 failed`。TG-02、TG-03、TG-04、TG-05、TG-06 全部继续为 `partial`；
共享 manifest/JSON、Manager/C6 装配和 live-platform acceptance 留给 Lead/真机完成。
