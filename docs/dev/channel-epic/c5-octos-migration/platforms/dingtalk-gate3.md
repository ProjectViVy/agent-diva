# DingTalk C5-V Gate 3 evidence

状态：`partial`。主 listener 是 DingTalk Stream；Octos HMAC/sessionWebhook 只作辅助，不是
降级 webhook。

| 目标 | 源码 symbol / endpoint | fixture 与 Rust 测试 | 精确证据 | 生命周期/剩余项 |
| --- | --- | --- | --- | --- |
| OAuth/cache/401 | `access_token`, `authenticated_json`; `/v1.0/oauth2/accessToken` | `token-response.json`; `token_is_cached_and_group_payload_uses_open_conversation_id`, `authenticated_send_refreshes_once_after_401` | token response 缓存；401 清缓存、第二 token 后重试；真实 send ID 保留 | expiry/single-flight 并发与 token 429 需补 |
| Stream register/ACK/stop/dedup | `register_stream`, `run_stream_once`, `handle_stream_message`; `/v1.0/gateway/connections/open` | `stream-callback.json`; `stream_register_callback_ack_cancel_and_dedup_are_wire_bound` | register path/body、WS callback 两次均返回 code 200 ACK；Fabric 只收一次；取消结束 reader | start reconnect/backoff 还需多连接 transcript |
| policy before media | `parse_event`, `policy_allows`, `admit_event` | `stream-callback.json`; `parser_applies_allowlist_before_media_url_resolution`, `media_ingress_downloads_after_policy_and_admits_typed_attachment` | denied sender 在 parse attachments 前返回；allowed media 才取 token/download/store；envelope 是 typed part | group policy 的全矩阵与 download failure retry 仍 partial |
| media upload/send | `authenticated_multipart`, `send_attachment`; `/media/upload`, group/private send | `media-upload-response.json`, `group-send-response.json`; `media_upload_and_send_preserve_typed_part_and_partial_failure` | multipart filename/MIME/bytes exact；`sampleImageMsg` 带 media ID；send 429 返回 `RateLimited`，不伪造 receipt | audio/video/file 分支需逐类 wire |
| text/group/private receipt | `send_text`, `execute_send`; `/v1.0/robot/groupMessages/send`, `/v1.0/robot/oToMessages/batchSend` | `group-send-response.json`; `token_is_cached_and_group_payload_uses_open_conversation_id` | group 使用 `openConversationId`，private 使用 `userIds`；success message ID 进入 Accepted receipt | reply/chunk/card/edit/delete/typing 保持 false/unsupported |
| HMAC/session safety | `dingtalk_signature`, `verify_dingtalk_signature`, `cache_session_webhook` | `stream-callback.json`; `signature_matches_octos_shape_and_fails_closed`, `shipped_stream_and_media_fixtures_are_parseable` | bad/empty signature fail-closed；session URL 仅信任 HTTPS DingTalk host | TTL expiry 与真实 callback signature transcript 仍 partial |

敏感信息：测试中的 token、session URL、sender ID 都是 redacted/fake，错误信息不回显 secret。
