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
