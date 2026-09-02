# Feishu/Lark C5-V Gate 3 evidence

状态：`verified`（本地 deterministic wire/store evidence；真机凭据与远端平台执行仍是
外部 gap）。China/global/Lark endpoint 是 adapter 私有 region mapping；没有新增配置
键，也没有 webhook fallback。

| 目标 | 源码 symbol / endpoint | fixture 与 Rust 测试 | 精确证据 | 生命周期/剩余项 |
| --- | --- | --- | --- | --- |
| protobuf WS/ACK/dedup | `run_websocket`, `ack_frame`, `handle_event_payload`; callback WS endpoint | `protobuf-frame.json`, `protobuf-event.json`, `ack-deadline.json`, `ws-lifecycle.json`; `protocol_and_webhook_fixtures_are_parseable`, `websocket_fixture_covers_ping_event_ack_health_and_stop_lifecycle`, `duplicate_events_are_not_admitted_twice` | local socket 证明 endpoint 返回 `service_id=42`、初始 method=0 ping、event admission、ACK `biz_rt=0` 与 `{"code":200...}`；stable message ID replay 只入 Fabric 一次 | 本地 fixture 覆盖 stop/health；真 Feishu reconnect/网络行为仍需外部执行 |
| token/cache/401 | `get_access_token_with_cancel`, `get_websocket_url_with_cancel`, `invalidate_token`, `send_parts`; `/auth/v3/tenant_access_token/internal` | `send-responses.json`, `error-responses.json`, `ws-lifecycle.json`; `token_refresh_is_single_flight_under_concurrent_demand`, `token_and_ws_endpoint_requests_honor_cancellation`, `token_and_ws_endpoint_failures_preserve_typed_429_and_malformed_errors`, `send_retries_once_after_401_and_reuses_cached_token` | 并发 demand 只发一次 token；token/WS endpoint hang 在 cancel 后返回 typed `cancelled`；429 保留 `Retry-After`；malformed JSON 保留 typed error；401 后丢缓存并以新 Bearer 重试 | 真机 endpoint/凭据未执行；没有新增 region 配置键 |
| webhook security | `verify_webhook_signature`, `decrypt_webhook_payload`, `parse_webhook_event` | `webhook-url-verification.json`, `webhook-encrypted-event.json`; `webhook_signature_is_fail_closed`, `aes256_decrypt_round_trip_and_bad_padding_fail_closed` | 缺 timestamp/nonce/signature、bad signature、bad base64/padding 均 fail-closed typed error；URL challenge 只走 token 校验 | 完整 HTTP ingress handler transcript 未引入，保持无 fallback |
| typed media | `fetch_and_store_media_with_cancel`, `build_inbound_parts_with_cancel`; resource GET | `media-resource.json`, `media-types.json`, `error-responses.json`; `media_fixture_is_stored_as_typed_attachment_after_authentication`, `media_variants_are_typed_and_read_back_with_sha`, `media_content_length_overflow_is_typed_before_body_read`, `media_download_cancellation_interrupts_read_and_releases_dedup`, `corrupt_attachment_readback_is_typed_and_releases_dedup` | image/file/audio/video 真实 MIME shape 均生成 typed `ContentPart`；20 MiB header/stream bound、cancel、empty/status/path errors、`AttachmentRef` SHA validation + local `get` readback；腐化 readback 返回 typed error | Feishu wire 未提供可验证远端 digest，故只证明本地写入/读回一致性，不宣称远端真实性 |
| send/reply/edit/delete/card | `build_outbound_payload`, `send_parts`, `upload_image`, `upload_file`, `execute_edit`, `execute_delete`; `/im/v1/messages...`, `/im/v1/images`, `/im/v1/files` | `send-responses.json`, `error-responses.json`; `send_fixture_returns_real_message_id_and_reply_uses_bound_endpoint`, `outbound_media_multipart_and_edit_delete_json_are_truthful`, `execute_delete_rejects_nonzero_and_malformed_json_codes`, `send_retries_once_after_401_and_reuses_cached_token` | send/reply 使用准确 query/path 与真实 message ID；multipart 字段、filename、Octos MIME 和 bytes 有 request assertion；edit/delete 解析 JSON code；delete non-zero/malformed 是 typed error；401 retry 有 wire assertion | 本地 HTTP fixture 未替代真平台；audio/video 仍按冻结矩阵在 transport 前 typed unsupported |
| unsupported/lifecycle | `ensure_supported`, `stop`, `run_listener`, listener cancellation | `region-endpoints.json`, `ws-lifecycle.json`, `error-responses.json`; `capabilities_match_frozen_feishu_matrix`, `unsupported_typing_and_reaction_have_zero_transport_side_effects`, `region_defaults_are_explicit_and_distinct`, `websocket_fixture_covers_ping_event_ack_health_and_stop_lifecycle` | generic typing/reaction 在 transport 前 typed reject；内部 seen marker 为 best-effort；WS stop、health transition、cancel、admission deadline 与 reconnect reset 均由源码/fixture 覆盖 | 通用 `InteractionReaction` 仍明确 unsupported；真机网络、重连时序和平台配额需外部执行 |

敏感信息：app secret/token 只使用 fixture 值；webhook 测试不记录原文或密钥。

## Fixed-SHA audit addendum

本 addendum 是对上表四个分配行的逐项审计，并由本次 Feishu repair closure
supersede 原先的 partial 结论。DIVA 基线为
c5-audit-feishu HEAD 2b3ef682c48427387b6c0e36a96d6796996bd499。Octos 基线为
C:\Users\Administrator\Desktop\morediva\.workspace\octos，SHA
5ea987813de4fd2afdd1d78f2106ad2868f0d923（v2.0.3-rc.9）。Octos 路径以下均
相对于该 checkout，行号是该 SHA 的稳定锚点。本次 focused validation 为
`cargo test -p agent-diva-channels --lib feishu`：33 passed；另执行
`cargo check -p agent-diva-channels`、`cargo fmt --all -- --check` 与
`git diff --check`。

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
- DIVA L258-L338：FeishuAdapter::get_access_token_with_cancel 以读缓存加
  token_refresh mutex 实现 single-flight，按 expire 并提前 300 秒刷新；等待
  refresh mutex、HTTP send 和 JSON read 都可被 CancellationToken 打断。请求为
  POST {api_base}/auth/v3/tenant_access_token/internal，body
  {"app_id":...,"app_secret":...}，要求 code=0 和非空 token，失败映射为
  token_malformed、token_rejected 或 HTTP typed error。
- DIVA L371-L445：get_websocket_url_with_cancel 发 POST
  {ws_base}/callback/ws/endpoint，body {"AppID":...,"AppSecret":...}，要求
  HTTP success、code=0、非空 data.URL/data.url，并读取 PingInterval；HTTP send
  和 JSON read 可被 CancellationToken 打断。
- DIVA L1050-L1205：run_listener/run_websocket 采用 250 ms 到 30 s backoff；connect、
  read、heartbeat tick、reconnect sleep 选中 context/listener cancellation；event
  admission deadline 为 2 秒，完成 admission 后才发 ACK，并在成功连接后重置
  reconnect delay。heartbeat timeout 为 300 秒，默认 ping 为 120 秒且不少于 10
  秒；ACK payload 为
  {"code":200,"headers":{},"data":[]}，并追加 biz_rt=0。
- DIVA L250-L255、L1028-L1046、L1080-L1095、L1128-L1130、L1732-L1799：health
  在 token health probe、WS connect、断开/错误、stop 时更新
  Healthy/Degraded/Down；probe_health 只取 token，不做额外 API/WS round trip。

Fixture/test/精确结果：

- agent-diva-channels/tests/fixtures/c5/feishu/region-endpoints.json、
  protobuf-frame.json、ack-deadline.json、ws-lifecycle.json；后者记录 endpoint
  request/response、service_id=42、PingInterval、初始 ping、event、ACK 和
  health/stop 观察点；ack-deadline.json 包含 admission 2000 ms、heartbeat
  timeout 300 s、backoff 250/30000 ms 和 biz_rt=0 invariant。
- protocol_and_webhook_fixtures_are_parseable（feishu.rs:L3832-L3877）解析
  frame/event fixture 并断言 ACK payload/header；
  websocket_fixture_covers_ping_event_ack_health_and_stop_lifecycle
  （L3113-L3231）运行本地 WebSocket server，断言 endpoint 后的初始 ping、event
  admission、ACK、Healthy 与 stop；没有使用远端 Feishu 网络。
- capabilities_match_frozen_feishu_matrix（L2657-L2680）和
  region_defaults_are_explicit_and_distinct（L4281-L4298）断言 capability/
  region；token_refresh_is_single_flight_under_concurrent_demand（L2944-L2971）
  断言并发 demand 只有一个 token request；
  token_and_ws_endpoint_requests_honor_cancellation（L2973-L3030）断言 token/
  endpoint hang 在 cancel 后及时返回 `cancelled`；
  token_and_ws_endpoint_failures_preserve_typed_429_and_malformed_errors
  （L3032-L3085）断言 429 Retry-After 7/9 与 malformed typed errors；
  send_retries_once_after_401_and_reuses_cached_token（L2891-L2942）断言
  token-first → 401 → token-second → successful message response，最终 receipt
  ID 为 om-retried。

生命周期结论：

- 已闭合：endpoint mapping、token cache/expiry/mutex single-flight、token/WS
  request cancellation、protobuf event/ping/ACK、2 s admission、300 s receive
  timeout、连接/读取/重连 sleep cancellation、successful connection backoff
  reset、429/malformed typed errors 与 Healthy/Degraded/Down transitions。
- evidence boundary：WS lifecycle、429/malformed、single-flight 和 health wire
  证据来自 deterministic local fixtures；没有真 Feishu 凭据、真实网络、平台
  quota 或长时间 heartbeat/reconnect soak。
- blocked/unsupported：无 FS-01 blocked 项；generic InteractionReaction 仍按
  冻结 capability matrix unsupported，内部 seen marker 不改变该结论。

最终 disposition：verified（local deterministic evidence）；真机/长时网络验证
由 Lead 或发布前环境完成。

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
- DIVA agent-diva-channels/src/adapters/feishu.rs:L1349-L1398：
  build_inbound_parts_with_cancel 映射 text→Text、post→Markdown、image→Image、
  file|sticker→File、audio→Audio、media|video→Video。
- DIVA L1400-L1603：fetch_and_store_media_with_cancel 在 sender/bot/allowlist 和
  admission permit 之后取 token，再发
  GET {api_base}/im/v1/messages/{message_id}/resources/{key}，query
  type=image|file|audio|video。MEDIA_REQUEST_TIMEOUT=20 s；HTTP status、20 MiB
-  Content-Length/body 上限、empty body、typed MIME family 和 path segment 均有
  typed error；stream read、store put/get 都可被 CancellationToken 打断。成功调用
  AdapterServices.attachments.put，传递 source channel、message ID、sender、
  filename、declared MIME、bytes，并通过 validate_attachment_reference、
  StoredAttachment::validate 和 byte-for-byte readback 后才发布 ContentPart。
- DIVA agent-diva-channels/src/adapter.rs:L44-L120、L140-L188：
  IngressAttachment、StoredAttachment 和 validate_attachment_reference 提供
  sha256:<hex>、size、MIME、leaf filename 校验；这是共享 store 合同，不是
  Feishu media wire 证据。

Fixture/test/精确结果：

- agent-diva-channels/tests/fixtures/c5/feishu/media-resource.json、
  media-types.json、error-responses.json 覆盖 image/file/audio/video 的 Feishu
  resource path/query、MIME 和脱敏 deterministic bytes；media-types 明确不含
  远端 digest。
- media_fixture_is_stored_as_typed_attachment_after_authentication
  （feishu.rs:L2750-L2821）按 token → resource GET → local store put/get →
  seen-reaction 顺序，断言 Fabric `ContentPart::Image`、reference SHA、size、
  MIME 与 readback bytes。
- media_variants_are_typed_and_read_back_with_sha（L3233-L3355）逐类断言
  image/file/audio/video ContentPart、Feishu MIME、`sha256:<hex>` ref 和本地
  `ChannelAttachmentStore::get` readback；
  corrupt_attachment_readback_is_typed_and_releases_dedup（L3357-L3401）断言
  corrupt readback 返回 `attachment_corrupt` 且不发布；
  media_content_length_overflow_is_typed_before_body_read（L3403-L3445）断言
  Content-Length 超 20 MiB；media_download_cancellation_interrupts_read_and_releases_dedup
  （L3447-L3497）断言挂起 body 在 cancel 后返回 `cancelled` 且 pending dedup
  可重用；failed_media_admission_releases_message_dedup_for_retry
  （L3749-L3817）断言 media HTTP failure 后同一 message 可重试。

生命周期结论：

- 已闭合：allowlist/bot filter 与 admission permit 在 media 前、Bearer 在 resource
  GET 前、四类 typed ContentPart、20 MiB Content-Length/stream bound、MIME/
  empty/status/path errors、read cancellation、store put/get readback，以及
  media/build/Fabric failure release pending dedup；不会把远端 URL/path 发布为
  envelope attachment。
- integrity boundary：Feishu resource response 没有平台 digest/signature 可比对。
  本实现和 fixture 只以本地 AttachmentStore SHA/reference/readback 证明“写入的
  bytes 与读回 bytes 一致”，没有凭空添加或宣称远端媒体真实性 digest。
- blocked/unsupported：无 FS-03 blocked 项；sticker 沿用 file 形状，未宣称额外
  平台能力。

最终 disposition：verified（local store integrity boundary）；远端媒体真实性与
  真机平台行为仍需 Lead/发布前环境完成。

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
- DIVA agent-diva-channels/src/adapters/feishu.rs:L521-L604：
  send_parts 的 Text 为 msg_type=text、{"text":text}，Markdown/Card 为 interactive；
  普通 send 是 POST {api_base}/im/v1/messages?receive_id_type=chat_id|open_id，
  reply 是 POST {api_base}/im/v1/messages/{reply_to}/reply。code rejection、
  malformed JSON、HTTP failure 为 typed send_rejected/send_malformed/HTTP error；
  成功解析 data.message_id 并返回 accepted_receipt。带 idempotency key 时，401
  清 token、刷新并最多重试一次。
- DIVA L606-L878：build_outbound_payload/upload_image/upload_file 从
  content-addressed store 读 bytes，multipart 目标分别为 /im/v1/images 和
  /im/v1/files；image_type=message，file_type=stream、file_name；成功解析
  image_key/file_key，HTTP/malformed/rejected 是 typed error。audio/video 在
  transport 前返回 UnsupportedCapability；store readback 先做
  StoredAttachment::validate，避免上传损坏数据。
- DIVA L889-L1025：execute_edit 发 PATCH 并解析 MessageResponse，返回带目标 ID
  的 accepted_receipt；execute_delete 发 DELETE 后同样解析 MessageResponse，要求
  JSON code=0 才返回 delivered_receipt，malformed/non-zero code 映射为 typed
  delete_malformed/delete_rejected。
- DIVA L2199-L2220：map_http_error 将 429 和 Retry-After 秒数映射到
  AdapterError::RateLimited（无 header 时 1 s），没有自动 429 retry。send 的
  idempotency-safe 401 retry 保持最多一次。

Fixture/test/精确结果：

- agent-diva-channels/tests/fixtures/c5/feishu/send-responses.json 记录 send、
  reply、upload、edit、delete 的 Feishu-shaped request/response/receipt；
  outbound_media_multipart_and_edit_delete_json_are_truthful
  （feishu.rs:L3499-L3651）执行 image/file send、edit、delete，断言 multipart
  field names、filename、Octos `application/octet-stream`、bytes、message IDs、
  edit request 与 delete path/receipt。
- send_fixture_returns_real_message_id_and_reply_uses_bound_endpoint
  （L2823-L2889）证明普通 send receipt `om-fixture-sent`、reply receipt
  `om-fixture-reply` 与 exact POST paths；
  send_retries_once_after_401_and_reuses_cached_token（L2891-L2942）证明一次
  idempotency-safe 401 token refresh；
  execute_delete_rejects_nonzero_and_malformed_json_codes（L3653-L3690）证明
  HTTP 200 但 code 非 0 或 JSON malformed 都返回 typed error；
  token_and_ws_endpoint_failures_preserve_typed_429_and_malformed_errors
  （L3032-L3085）证明 HTTP 429/Retry-After 保留为 typed RateLimited。

生命周期结论：

- 已闭合：receive ID type/reply endpoint、真实 message ID、image/file multipart
  构造与响应解析、store corruption rejection、edit response parsing、delete
  JSON code parsing、invalid media combination、unsupported audio/video
  pre-transport reject、带 key 的一次 401 retry、429 typed mapping。
- evidence boundary：本地 HTTP fixture 证明 request/response parsing 和 receipt/error
  语义；没有真 Feishu endpoint、平台 partial failure 或长时 REST soak。冻结矩阵
  外 audio/video 仍明确 typed unsupported，未添加 webhook fallback。
- blocked/unsupported：无 FS-04 blocked 项；`InteractionStreamFinalize` 沿用
  adapter 既有 edit/finalize 路径，未凭 fixture 宣称额外平台 API。

最终 disposition：verified（local wire evidence）；真机 endpoint 与平台配额/网络
行为仍由 Lead/发布前环境完成。

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
- DIVA agent-diva-channels/src/adapters/feishu.rs:L107-L112、L2017-L2031：
  DedupState 的 seen/pending/last_cleanup，dedup_key 优先稳定 message ID、缺失
  时回退 event ID；TTL 600 s、cleanup 300 s。
- DIVA L1208-L1347：handle_event_payload 先做 bot/allowlist，再 reserve pending；
  admission busy 在 2 s 内返回 fabric_admission_busy 并 release；media/build/Fabric
  failure/cancel release；只有 Fabric admission 成功后 commit_dedup，随后才调用
  marker。
- DIVA L1606-L1645：add_seen_reaction_with_cancel 在成功入站后发
  POST {api_base}/im/v1/messages/{message_id}/reactions，Bearer token，body
  {"reaction_type":{"emoji_type":"THUMBSUP"}}。非 2xx body 被消费后仍 Ok，调用方
  对 transport/token/cancel error 使用 100 ms timeout 并忽略，所以 marker 不会让
  已 admission 的 ingress 失败。
- DIVA L1535-L1558 与 L185-L218：ChannelCommand::React 在 capability check 后返回
  UnsupportedCapability(InteractionReaction)，无 HTTP side effect；这是 generic
  React 路径，不等于内部 marker。

Fixture/test/精确结果：

- agent-diva-channels/tests/fixtures/c5/feishu/protobuf-event.json、
  error-responses.json；duplicate_events_are_not_admitted_twice
  （feishu.rs:L2716-L2748）断言同一 event/message 第二次没有 Fabric envelope；
  dedup_uses_stable_message_id_before_event_id_fallback（L3819-L3830）断言
  message ID 优先且缺失时回退 event ID。
- unsupported_typing_and_reaction_have_zero_transport_side_effects
  （L2420-L2449）断言 generic React 的 pre-transport typed reject。
- media_fixture_is_stored_as_typed_attachment_after_authentication
  （L2750-L2821）将 reaction endpoint 纳入真实请求顺序；
  seen_reaction_failure_is_isolated_from_ingress_and_message_dedup
  （L3692-L3747）用 503 reaction wire fixture 断言 ingress 仍成功、marker failure
  不产生 typed ingress failure，且新 event ID 的同 message replay 不再触发
  Fabric admission 或第二个 reaction。

生命周期结论：

- 已闭合：pending 与 committed dedup 分离、stable message ID 优先、busy/media/
  Fabric/cancel failure release、Fabric success 后 commit、commit 后 100 ms
  best-effort marker，以及 generic React 的 false-capability typed error。marker
  不生成 DeliveryReceipt，non-2xx/transport/token/cancel failure 不会撤销已入站
  admission。
- evidence boundary：fixture 覆盖 reaction 503 与 event-ID replay；没有宣称 TTL
  expiry、跨进程 dedup、真实 Feishu reaction quota 或 reconnect soak。
- implementation boundary：generic InteractionReaction 不在冻结目标且明确
  unsupported，不是缺陷；marker 非 2xx 即吞掉是有意的 best-effort 语义，不能
  作为 reaction receipt。跨 event-ID replay 现以 stable message ID dedup，并由
  独立 fixture 固定。
- blocked/unsupported：InteractionReaction 明确 unsupported，且首次 transport
  前返回 typed error；FS-05 行本身不转为 blocked，因为 ingress dedup 有实现路径。

最终 disposition：verified（local dedup/reaction-isolation evidence）；真机平台
配额与长时 replay 行为仍由 Lead/发布前环境完成。

## Audit handoff

- FS-01、FS-03、FS-04、FS-05 均升级为 `verified`，边界为本地 deterministic
  wire/store evidence。
- 本 worker 修改 Feishu adapter、Feishu C5 fixture 与本页 Gate3；没有修改公共
  contract、Cargo 文件、manifest、shared TCK、TODO、LOCK、Manager 或其他频道。
- 本 worker 执行 `cargo test -p agent-diva-channels --lib feishu`（33 passed）、
  `cargo check -p agent-diva-channels`、`cargo fmt --all -- --check` 与
  `git diff --check`；没有执行 live network。真机凭据、平台 quota、长时
  heartbeat/reconnect soak 留给 Lead/发布前环境。
