# Feishu/Lark C5-V Gate 3 evidence

状态：`partial`。China/global/Lark endpoint 是 adapter 私有 region mapping；没有新增配置
键，也没有 webhook fallback。

| 目标 | 源码 symbol / endpoint | fixture 与 Rust 测试 | 精确证据 | 生命周期/剩余项 |
| --- | --- | --- | --- | --- |
| protobuf WS/ACK/dedup | `run_websocket`, `ack_frame`, `handle_event_payload`; callback WS endpoint | `protobuf-frame.json`, `protobuf-event.json`, `ack-deadline.json`; `protocol_and_webhook_fixtures_are_parseable`, `duplicate_events_are_not_admitted_twice` | protobuf frame 解码；ACK `biz_rt=0` 与 `{"code":200...}`；event/message dedup 只入 Fabric 一次 | local WS connect/reconnect/heartbeat deadline 仍需完整 server transcript |
| token/cache/401 | `get_access_token`, `invalidate_token`, `send_parts`; `/auth/v3/tenant_access_token/internal` | `send-responses.json`; `send_fixture_returns_real_message_id_and_reply_uses_bound_endpoint`, `send_retries_once_after_401_and_reuses_cached_token` | 缓存命中只发一次 token；401 后丢缓存、刷新一次并以新 Bearer 重试；成功 ID 为 `om-retried` | 并发 single-flight 压力与 429 Retry-After 仍 partial |
| webhook security | `verify_webhook_signature`, `decrypt_webhook_payload`, `parse_webhook_event` | `webhook-url-verification.json`, `webhook-encrypted-event.json`; `webhook_signature_is_fail_closed`, `aes256_decrypt_round_trip_and_bad_padding_fail_closed` | 缺 timestamp/nonce/signature、bad signature、bad base64/padding 均 fail-closed typed error；URL challenge 只走 token 校验 | 完整 HTTP ingress handler transcript 未引入，保持无 fallback |
| typed media | `fetch_and_store_media`, `build_inbound_parts`; resource GET | `media-resource.json`; `media_fixture_is_stored_as_typed_attachment_after_authentication` | token 后 GET message resource；content-type/size bounded；AttachmentStore 返回 typed Image ref | file/audio/video、损坏 bytes 与下载 timeout 仍需逐类 fixture |
| send/reply/edit/delete/card | `build_outbound_payload`, `send_parts`, `execute_edit`, `execute_delete`; `/im/v1/messages...` | `send-responses.json`; `send_fixture_returns_real_message_id_and_reply_uses_bound_endpoint` | send 用 `receive_id_type=chat_id`；reply 用 `/messages/{parent}/reply`；真实 message ID | edit/delete exact response、multipart upload、429/malformed response 需补 wire 记录 |
| unsupported/lifecycle | `ensure_supported`, `stop`, listener cancellation | `region-endpoints.json`; `capabilities_match_frozen_feishu_matrix`, `unsupported_typing_and_reaction_have_zero_transport_side_effects`, `region_defaults_are_explicit_and_distinct` | false typing/reaction 在 transport 前 typed reject；region mapping 无全局 endpoint 覆盖 | WS stop/heartbeat 端到端仍 partial；seen reaction 是内部 best-effort marker，不是通用 reaction capability |

敏感信息：app secret/token 只使用 fixture 值；webhook 测试不记录原文或密钥。

## Fixed-SHA audit addendum

本 addendum 是对上表四个分配行的逐项审计。DIVA 基线为
c5-audit-feishu HEAD 2b3ef682c48427387b6c0e36a96d6796996bd499。Octos 基线为
C:\Users\Administrator\Desktop\morediva\.workspace\octos，SHA
5ea987813de4fd2afdd1d78f2106ad2868f0d923（v2.0.3-rc.9）。Octos 路径以下均
相对于该 checkout，行号是该 SHA 的稳定锚点。既有日志记录 Feishu adapter tests
10 passed；本 addendum 没有重新运行测试。

### FS-01 — region/token/WS

Frozen matrix target：ReliabilityHealth、ReliabilityHeartbeat、
ReliabilityTokenRefresh、ReliabilityPacing、ReliabilitySupervisedRestart。
region mapping 是 adapter 私有实现，不是新的 capability 或配置键。

Octos/DIVA symbol 对照：

- Octos crates/octos-bus/src/feishu_channel.rs:L314-L327：
  base_url_for_region、domain_for_region 将 global/lark 映射到
  https://open.larksuite.com[/open-apis]，其它值回落到
  https://open.feishu.cn[/open-apis]。
- Octos crates/octos-bus/src/feishu_channel.rs:L684-L736：
  FeishuChannel::get_token 发 POST /open-apis/auth/v3/tenant_access_token/internal，
  body 为 {"app_id":...,"app_secret":...}，缓存 TOKEN_TTL_SECS=7000；非 2xx、
  非 JSON、缺 tenant_access_token 只返回 eyre 错误，不做 typed 429/401 retry。
- Octos crates/octos-bus/src/feishu_channel.rs:L738-L789：
  FeishuChannel::get_ws_url 发 domain-root POST /callback/ws/endpoint，body 为
  {"AppID":...,"AppSecret":...}，读取 data.URL 或 data.url。
- Octos crates/octos-bus/src/feishu_channel.rs:L64-L210、L298-L312：
  Frame 编解码包含 protobuf2 fields 1/2/3/4/5/8/9；new_ping_frame 是
  method=0、header type=ping，service 来自 URL 的 service_id。
- Octos crates/octos-bus/src/feishu_channel.rs:L1214-L1408：
  start_ws 每次连接前取 URL，固定 120 秒 ping，断线后固定 2 秒重连；阻塞 WS
  read 未选入 shutdown，也没有 heartbeat ACK deadline。
- DIVA agent-diva-channels/src/adapters/feishu.rs:L50-L75、
  L131-L183：FeishuRegion::{api_base,ws_base} 和默认 China/显式 Global/Lark；
  当前 shared config 没有 region 字段。
- DIVA L257-L342：FeishuAdapter::get_access_token 以读缓存加 token_refresh mutex
  实现 single-flight，按 expire 并提前 300 秒刷新。请求为
  POST {api_base}/auth/v3/tenant_access_token/internal，body
  {"app_id":...,"app_secret":...}，要求 code=0 和非空 token，失败映射为
  token_malformed、token_rejected 或 HTTP typed error。
- DIVA L348-L407：get_websocket_url 发 POST {ws_base}/callback/ws/endpoint，
  body {"AppID":...,"AppSecret":...}，要求 HTTP success、code=0、非空 data.URL，
  并读取 PingInterval。
- DIVA L964-L1104：run_listener/run_websocket 采用 250 ms 到 30 s backoff；connect、
  read、heartbeat tick、reconnect sleep 选中 context/listener cancellation；event
  admission deadline 为 2 秒，完成 admission 后才发 ACK。heartbeat timeout 为
  300 秒，默认 ping 为 120 秒且不少于 10 秒；ACK payload 为
  {"code":200,"headers":{},"data":[]}，并追加 biz_rt=0。
- DIVA L249-L255、L950-L962、L1527-L1532、L1584-L1593：health 在连接/health
  probe、断开/错误、stop 时更新 Healthy/Degraded/Down；probe_health 只取 token，
  不做 API/WS round trip。

Fixture/test/精确结果：

- agent-diva-channels/tests/fixtures/c5/feishu/region-endpoints.json、
  protobuf-frame.json、ack-deadline.json；后者只是策略数据，包含 admission
  2000 ms、heartbeat timeout 300 s、backoff 250/30000 ms 和 biz_rt=0 invariant。
- protocol_and_webhook_fixtures_are_parseable（feishu.rs:L2664-L2711）只解析
  frame/event/fixture shape 并断言 ack payload/header；没有 WS socket transcript。
- capabilities_match_frozen_feishu_matrix（L2393-L2417）和
  region_defaults_are_explicit_and_distinct（L2875-L2893）断言 capability/
  region；send_retries_once_after_401_and_reuses_cached_token（L2611-L2662）
  断言 token-first → 401 → token-second → successful message response，最终
  receipt ID 为 om-retried。

生命周期结论：

- 已有实现路径覆盖 endpoint mapping、token cache/expiry/mutex、protobuf
  event/ping/ACK、2 s admission、300 s receive timeout、连接/读取/重连 sleep
  cancellation 与 health state transitions。
- evidence_gap：没有 WS local server 逐帧证明 endpoint response、service_id、
  初始 ping、heartbeat/pong、event ACK、断线 backoff、stop；没有并发
  single-flight 压力、token/WS endpoint 429/malformed、health probe wire 证据。
- implementation_gap：get_access_token/get_websocket_url 不接收 cancellation
  token，初始 token/endpoint 请求或等待 refresh mutex 的取消只依赖 HTTP client
  timeout；run_listener 也没有在成功连接后显式 reset backoff。两项预期都还没有
  被生命周期测试固定。
- blocked/unsupported：无。

最终 disposition：partial。实现路径存在，但 WS lifecycle、取消期间 token wait、
single-flight 和 health wire 证据不完整，不能升级 verified。

### FS-03 — typed image/file/audio/media ingress

Frozen matrix target：IngressTypedAttachments；adapter snapshot 同时声明
max_attachment_bytes=20 MiB，以及 image/*、audio/*、video/*、
application/octet-stream MIME 集合。

Octos/DIVA symbol 对照：

- Octos crates/octos-bus/src/feishu_channel.rs:L1023-L1212：
  FeishuChannel::parse_event 处理 im.message.receive_v1；image/file/audio/media/
  sticker 都调用 download_feishu_media，结果写入 InboundMessage.media 路径字符串，
  下载失败只 warn。
- Octos crates/octos-bus/src/feishu_channel.rs:L791-L813：
  download_feishu_media 先 get_token，再请求
  GET /open-apis/im/v1/messages/{message_id}/resources/{file_key}?type={type}，
  带 Authorization Bearer，落地时间命名文件。
- Octos crates/octos-bus/src/media.rs:L15-L92：
  max_media_bytes/download_media_with_cap 默认 50 MiB；Content-Length 或流式
  body 超限拒绝，成功写本地路径；没有 Feishu MIME family、digest 或
  AttachmentStore 校验。
- DIVA agent-diva-channels/src/adapters/feishu.rs:L1230-L1267：
  build_inbound_parts 映射 text→Text、post→Markdown、image→Image、
  file|sticker→File、audio→Audio、media|video→Video。
- DIVA L1270-L1405：fetch_and_store_media 在 sender/bot/allowlist 和 admission
  permit 之后取 token，再发
  GET {api_base}/im/v1/messages/{message_id}/resources/{key}，query
  type=image|file|audio|video。MEDIA_REQUEST_TIMEOUT=20 s；HTTP status、20 MiB
  Content-Length/body 上限、empty body、typed MIME family 和 path segment 均有
  typed error；成功调用 AdapterServices.attachments.put，传递 source channel、
  message ID、sender、filename、declared MIME、bytes。
- DIVA agent-diva-channels/src/adapter.rs:L44-L120、L140-L188：
  IngressAttachment、StoredAttachment 和 validate_attachment_reference 提供
  sha256:<hex>、size、MIME、leaf filename 校验；这是共享 store 合同，不是
  Feishu media wire 证据。

Fixture/test/精确结果：

- agent-diva-channels/tests/fixtures/c5/feishu/media-resource.json 只覆盖
  GET /im/v1/messages/om-fixture-message/resources/file-key-fixture?type=image，
  200 image/png，body 为脱敏 deterministic placeholder，Authorization 为
  Bearer <redacted>。
- media_fixture_is_stored_as_typed_attachment_after_authentication
  （feishu.rs:L2487-L2541）按 token → resource GET → seen-reaction 三请求顺序，
  断言 Fabric envelope 的 ContentPart::Image 和一次 store put。
- attachment_store_is_content_addressed_and_round_trips
  （channel_adapter_shared_tck.rs:L96-L117）只证明通用 store 的 digest/round-trip；
  没有 Feishu file/audio/video 逐类执行。

生命周期结论：

- 已有实现路径覆盖 allowlist/bot filter 在 media 前、admission permit 在 media
  前、Bearer 在 resource GET 前、四类 typed ContentPart、20 MiB/MIME/empty/read/
  path error，以及 media/build/Fabric failure release pending dedup；不会把远端
  URL/path 发布为 envelope attachment。
- evidence_gap：没有 file、audio、video/media、sticker fixture 和 typed envelope；
  没有 Content-Length/stream 超限、MIME mismatch、empty/HTTP failure、read timeout/
  cancel、filename、SHA-256 读回或损坏 fixture。Feishu test store 只计数和生成摘要，
  不执行 get round-trip。
- implementation_gap：Feishu resource response 没有平台 digest/signature 可比对；
  fetch_and_store_media 只能检查 size/MIME/non-empty。共享 store digest 只能证明
  存储引用与已收到 bytes 一致，不能证明远端内容真实性。
- blocked/unsupported：无；不能把证据不足的四类 ingress media 改标为 unsupported。

最终 disposition：partial。image 主路径有实现和局部证据，其他媒体类型、下载
边界、取消和内容完整性尚未闭合。

### FS-04 — upload/send/reply/edit/delete

Frozen matrix target：EgressText、EgressMarkdown、EgressReply、EgressImage、
EgressFile、EgressCard、InteractionEdit、InteractionDelete、
InteractionStreamFinalize。

Octos/DIVA symbol 对照：

- Octos crates/octos-bus/src/feishu_channel.rs:L815-L894：
  upload_image/upload_file 分别 POST /im/v1/images multipart
  image_type=message,image，以及 POST /im/v1/files multipart
  file_type=stream,file_name,file；读取本地 bytes，MIME 为
  application/octet-stream，只取 data.image_key/file_key。
- Octos crates/octos-bus/src/feishu_channel.rs:L896-L1012：
  send_message_returning_id 普通请求为
  POST /im/v1/messages?receive_id_type=chat_id|open_id，body 含
  receive_id,msg_type,content；reply_message_returning_id 为
  POST /im/v1/messages/{parent_id}/reply，body 不含 receive_id，成功读取
  data.message_id。send_with_id 只发送 interactive Markdown card。
- Octos crates/octos-bus/src/feishu_channel.rs:L1498-L1549：
  send 把 media 作为独立 top-level message；upload/send 失败只 warn 并继续，
  没有 per-part receipt。
- Octos crates/octos-bus/src/feishu_channel.rs:L1582-L1647：
  edit_message/delete_message 使用 PATCH/DELETE /im/v1/messages/{message_id}；
  JSON code != 0 只 warn 仍 Ok，属于 legacy silent-success。
- DIVA agent-diva-channels/src/adapters/feishu.rs:L486-L604：
  send_parts 的 Text 为 msg_type=text、{"text":text}，Markdown/Card 为 interactive；
  普通 send 是 POST {api_base}/im/v1/messages?receive_id_type=chat_id|open_id，
  reply 是 POST {api_base}/im/v1/messages/{reply_to}/reply。code rejection、
  malformed JSON、HTTP failure 为 typed send_rejected/send_malformed/HTTP error；
  成功解析 data.message_id 并返回 accepted_receipt。带 idempotency key 时，401
  清 token、刷新并最多重试一次。
- DIVA L606-L825：build_outbound_payload/upload_image/upload_file 从
  content-addressed store 读 bytes，multipart 目标分别为 /im/v1/images 和
  /im/v1/files；image_type=message，file_type=stream、file_name；成功解析
  image_key/file_key，HTTP/malformed/rejected 是 typed error。audio/video 在
  transport 前返回 UnsupportedCapability。
- DIVA L827-L948：execute_edit 发 PATCH 并解析 MessageResponse，返回带目标 ID
  的 accepted_receipt；execute_delete 发 DELETE，当前只检查 HTTP status 后返回
  delivered_receipt，不解析 Feishu JSON code。
- DIVA L1966-L1987：map_http_error 将 429 和 Retry-After 秒数映射到
  AdapterError::RateLimited（无 header 时 1 s），没有自动 429 retry。

Fixture/test/精确结果：

- agent-diva-channels/tests/fixtures/c5/feishu/send-responses.json 和
  send_fixture_returns_real_message_id_and_reply_uses_bound_endpoint
  （feishu.rs:L2543-L2609）只证明普通 send receipt om-fixture-sent、reply receipt
  om-fixture-reply，以及 exact POST paths。fixture 的 edit/delete/upload 只有请求
  字符串，没有 response body。
- send_retries_once_after_401_and_reuses_cached_token（L2611-L2662）只证明一次
  401 token refresh；没有 upload multipart body、edit/delete 或 429 response
  断言。

生命周期结论：

- 已有实现路径覆盖 receive ID type/reply endpoint、真实 message ID、image/file
  multipart 构造、edit response parsing、invalid media combination、unsupported
  audio/video pre-transport reject、带 key 的一次 401 retry、429 typed mapping。
- evidence_gap：没有 multipart body/MIME/name、upload response、edit response、
  delete response、429 header/body、malformed JSON、Feishu code != 0、partial
  failure、stream-finalize edit/new-message、REST timeout/cancel 或幂等重试 wire
  transcript。
- implementation_gap：execute_delete 对 HTTP 200 但 JSON code != 0 会直接返回
  Delivered，没有 typed rejection，和 truthful receipt/error 合同不一致。
- blocked/unsupported：FS-04 不 blocked；audio/video 是冻结矩阵之外的明确
  unsupported，且已在首次 transport 前返回 typed error。

最终 disposition：partial。send/reply/401 主路径已证明，但 upload/edit/delete、
429/malformed/partial failure 和 finalize 生命周期没有完整证据。

### FS-05 — reaction seen/dedup

Frozen matrix target：IngressDedupId。Feishu frozen capability set 不包含
InteractionReaction；seen reaction 只是内部 best-effort marker。

Octos/DIVA symbol 对照：

- Octos crates/octos-bus/src/feishu_channel.rs:L1040-L1047：
  FeishuChannel::parse_event 在 sender/allowlist/media 前以 message_id 调用
  MessageDedup::is_duplicate；空 ID 不 dedup 并丢弃。
- Octos crates/octos-bus/src/dedup.rs:L19-L81：
  MessageDedup 容量 1000、默认 TTL 60 s；is_duplicate 检查时立即记录，forget
  可为失败重试移除，但 Feishu parser 不调用 forget。
- 在固定 SHA 的 Octos feishu_channel.rs 中没有 add_reaction symbol，也没有
  Feishu reaction 实现。最近的共享锚点是
  crates/octos-bus/src/channel.rs:L202-L210 的 Channel::react_to_message 默认
  no-op；不能从 Octos 推导 Feishu reaction wire capability。
- DIVA agent-diva-channels/src/adapters/feishu.rs:L107-L112、L1808-L1819：
  DedupState 的 seen/pending/last_cleanup，dedup_key 优先 event ID、缺失时回退
  message ID；TTL 600 s、cleanup 300 s。
- DIVA L1110-L1227：handle_event_payload 先做 bot/allowlist，再 reserve pending；
  admission busy 在 2 s 内返回 fabric_admission_busy 并 release；media/build/Fabric
  failure release；只有 Fabric admission 成功后 commit_dedup，随后才调用 marker。
- DIVA L1407-L1436：add_seen_reaction 在成功入站后发
  POST {api_base}/im/v1/messages/{message_id}/reactions，Bearer token，body
  {"reaction_type":{"emoji_type":"THUMBSUP"}}。非 2xx body 被消费后仍 Ok，调用方
  对 transport error 使用 100 ms timeout 并忽略，所以 marker 不会让已 admission
  的 ingress 失败。
- DIVA L1535-L1558 与 L185-L218：ChannelCommand::React 在 capability check 后返回
  UnsupportedCapability(InteractionReaction)，无 HTTP side effect；这是 generic
  React 路径，不等于内部 marker。

Fixture/test/精确结果：

- agent-diva-channels/tests/fixtures/c5/feishu/protobuf-event.json；
  duplicate_events_are_not_admitted_twice（feishu.rs:L2452-L2484）断言同一
  event/message 第二次没有 Fabric envelope。
- unsupported_typing_and_reaction_have_zero_transport_side_effects
  （L2420-L2449）断言 generic React 的 pre-transport typed reject。
- media_fixture_is_stored_as_typed_attachment_after_authentication
  （L2487-L2541）只把 reaction endpoint 放入 fixture 顺序，没有 reaction failure
  或 response status 断言。

生命周期结论：

- 已有实现路径覆盖 pending 与 committed dedup 分离、busy/media/Fabric failure
  release、Fabric success 后 commit、commit 后 100 ms best-effort marker，以及
  generic React 的 false-capability typed error。marker 不生成 DeliveryReceipt。
- evidence_gap：没有 reaction transport spy 来证明 non-2xx、transport error、token
  error、timeout 不影响 ingress；没有并发 pending、Fabric busy、admission cancel、
  TTL expiry、missing IDs、event/message collision 或 reconnect replay 测试。现有
  duplicate test 使用默认构造器，没有隔离 reaction endpoint 的 wire transcript。
- implementation_gap：generic InteractionReaction 不在冻结目标且明确 unsupported，
  不是缺陷；marker 非 2xx 即吞掉是有意的 best-effort 语义，不能作为 reaction
  receipt。dedup_key 在 event ID 存在时不同时使用 message ID，跨 event-ID replay
  语义尚未由测试固定。
- blocked/unsupported：InteractionReaction 明确 unsupported，且首次 transport
  前返回 typed error；FS-05 行本身不转为 blocked，因为 ingress dedup 有实现路径。

最终 disposition：partial。admission/commit/release 主路径存在，但 seen-reaction
failure isolation 和 dedup 边界没有独立 wire/lifecycle 证据。

## Audit handoff

- FS-01、FS-03、FS-04、FS-05 均保持 partial，未升级 verified。
- 本 worker 只编辑本页；没有修改 adapter、fixture、test、public contract、
  Cargo 文件、manifest、JSON、TODO 或 LOCK。
- 本 worker 未执行 compilation、build、cargo test、clippy、workspace gate 或
  live network；文中测试结果均来自现有源码和既有验证日志。
