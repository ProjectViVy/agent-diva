# Octos 外部接口与函数迁移台账

固定来源：Octos SHA `5ea987813de4fd2afdd1d78f2106ad2868f0d923`；本表是 C5-P2 的端点级索引，
完整差距和 fixture 见各频道 `*-scan.md`。路径中的 `{token}`、ID 和 secret 一律是变量，不能
写入日志或样例。

## Telegram Bot API

基础 URL：`https://api.telegram.org/bot{token}/{method}`；文件下载：
`https://api.telegram.org/file/bot{token}/{file_path}`。

| Octos symbol | 方法/路径 | 请求/响应要点 | DIVA 现状/目标 |
| --- | --- | --- | --- |
| `telegram_channel.rs::polling_default` | `getUpdates` long polling | update_id、offset、timeout；stream end 重连 | legacy polling；补 dedup/admission/backoff |
| `download_telegram_file` | `getFile(file_id)` + file URL | file_path → bounded bytes；30s timeout | 补 AttachmentStore、MIME/size/digest |
| `send_html_with_fallback` | `sendMessage` | chat_id、parse_mode=HTML、reply_parameters；parse error → plain | 保留 fallback，解析真实 message_id |
| egress media branch | `sendVoice`/`sendAudio`/`sendDocument` | multipart file；首项 caption ≤1024 | 逐项 receipt，禁止失败后假成功 |
| `send_typing`/`send_listening` | `sendChatAction` | `typing` / `record_voice` | legacy typing；统一可取消 action |
| callback handler | `answerCallbackQuery` | callback query ID ACK | 独立 inbound callback envelope |
| keyboard parser | `sendMessage` + `reply_markup.inline_keyboard` | text/callback_data rows | 冻结 schema、校验长度与权限 |
| `edit_message` | `editMessageText` | chat_id + message_id + HTML/keyboard | bound edit + receipt |
| `delete_message` | `deleteMessage` | chat_id + message_id | 跨 chat 拒绝 |
| `health_check` | `getMe` | authenticated bot probe | Healthy/Down typed mapping |

## Discord Gateway/REST

基础 REST：`https://discord.com/api/v10`；Gateway discovery 使用 bot Bearer token。

| Octos/DIVA symbol | 方法/路径 | 请求/响应要点 | DIVA 目标 |
| --- | --- | --- | --- |
| `fetch_gateway_ws_url` | `GET /gateway/bot` | Authorization `Bot {token}`；返回 URL | 保留配置 fallback，补证据 |
| Gateway handler | WS `?v=10&encoding=json` | HELLO、IDENTIFY、RESUME、READY、RECONNECT、INVALID_SESSION、DISPATCH | 完整状态机 + health |
| `start_typing` | `POST /channels/{channel_id}/typing` | bot auth；204 expected | 可取消 task |
| `send_with_id` | `POST /channels/{channel_id}/messages` | content、message_reference、attachments；响应 snowflake | 真实 `Accepted` ID |
| attachment send | 同上 multipart | filename/MIME/bytes | AttachmentStore readback |
| `edit_message` | `PATCH /channels/{channel_id}/messages/{message_id}` | content/embed fields | bound correlation |
| `delete_message` | `DELETE /channels/{channel_id}/messages/{message_id}` | 204/permission/404 | typed error |
| `react_to_message` | `PUT /channels/{channel_id}/messages/{message_id}/reactions/{emoji}/@me` | Unicode/custom emoji encoded | parse/escape fixture |
| `remove_reaction` | `DELETE /channels/{channel_id}/messages/{message_id}/reactions/{emoji}/@me` | same emoji encoding | zero-side-effect invalid emoji |
| `send_embed` | create message with embed | title/description/color/fields | Card→embed adapter |
| rate limit | REST 429 | header/body `retry_after` | map `RateLimited`, shared pacing waits |

## Feishu/Lark

China base：`https://open.feishu.cn/open-apis`；Global/Lark：
`https://open.larksuite.com/open-apis`。

| Octos symbol | 方法/路径 | 请求/响应要点 | DIVA 目标 |
| --- | --- | --- | --- |
| `get_token` | `POST {base}/auth/v3/tenant_access_token/internal` | app_id/app_secret → tenant_access_token | region-aware cache/single-flight |
| `get_ws_url` | `POST {domain}/callback/ws/endpoint` | client config → WS URL | 保留 DIVA protobuf frame |
| `handle_webhook` | `POST /webhook/event` | signature headers；plaintext URL verification | fail-closed signed/encrypted events |
| `verify_signature` | SHA-256(timestamp+nonce+encrypt_key+body) | compare `X-Lark-Signature` | typed auth error |
| `decrypt_lark_event` | AES-256-CBC | key=SHA256(encrypt_key) | encrypted event fixture |
| `download_feishu_media` | `GET /im/v1/messages/{message_id}/resources/{file_key}?type=image|file` | Bearer token → bytes | typed attachment |
| `upload_image` | `POST /im/v1/images` multipart | `image_type=message`, image → image_key | receipt + digest |
| `upload_file` | `POST /im/v1/files` multipart | `file_type=stream`, file_name, file → file_key | receipt + digest |
| `send_message_returning_id` | `POST /im/v1/messages?receive_id_type=open_id|chat_id` | msg_type/content/card → message_id | parse real ID |
| `reply_message_returning_id` | `POST /im/v1/messages/{parent_id}/reply` | reply body → child message_id | frozen reply_to |
| `edit_message` | `PATCH /im/v1/messages/{id}` | interactive card/content | bound edit |
| `delete_message` | `DELETE /im/v1/messages/{id}` | auth + message ID | typed error |
| DIVA `add_reaction` | `POST /im/v1/messages/{id}/reactions` | reaction_type | retain as best-effort seen, not generic command |

## DingTalk

### DIVA Stream/OpenAPI

| DIVA symbol | 方法/路径 | 请求/响应要点 | 目标 |
| --- | --- | --- | --- |
| `get_access_token` | `POST https://api.dingtalk.com/v1.0/oauth2/accessToken` | app key/secret → access_token | retain cache/single-flight |
| Stream register | `POST https://api.dingtalk.com/v1.0/gateway/connections/open` | connection info → WS endpoint | retain Stream |
| `send_raw` group | `POST https://api.dingtalk.com/v1.0/robot/groupMessages/send` | conversation ID/content | retain real ID/error |
| `send_raw` private | `POST https://api.dingtalk.com/v1.0/robot/oToMessages/batchSend` | `robotCode` + `userIds` + `msgKey/msgParam` | fixture validates payload and response mapping |
| `upload_media` | `POST https://oapi.dingtalk.com/media/upload?access_token={token}&type={kind}` | multipart → media_id | retain typed media |

### Octos webhook/sessionWebhook

| Octos symbol | 方法/路径 | 请求/响应要点 | DIVA 目标 |
| --- | --- | --- | --- |
| `handle_webhook` | `POST /dingtalk/webhook` | JSON event；timestamp/signature | signature-compatible side path |
| `verify_dingtalk_signature` | HMAC-SHA256(`timestamp\\nsecret`) Base64 | header/query sign | fail-closed |
| `target_webhook` | cached sessionWebhook or configured webhook | append `timestamp`/`sign` query | session cache with TTL |
| text send | `POST https://oapi.dingtalk.com/robot/send?access_token=...` | text ≤3600 | do not replace Stream; text-limit fixture |

## Email IMAP/SMTP

| Octos symbol | 协议调用 | 请求/响应要点 | DIVA 目标 |
| --- | --- | --- | --- |
| `imap_poll` | TLS connect → login → select mailbox → `SEARCH UNSEEN` | sequence set / RFC822 bytes | retain configurable mailbox + spawn_blocking |
| `imap_poll` | `FETCH RFC822` | parse From/Subject/Message-ID/In-Reply-To/References | typed envelope |
| `imap_poll` | `STORE +FLAGS (\\Seen)` | mark only after admission | DIVA mark_seen ordering |
| `email_thread_topic` | pure function | References root → In-Reply-To → ID → normalized subject | adapt into thread_id |
| `should_skip_self_reply` | pure function | self sender + `Re:` | retain/strengthen DIVA loop guard |
| `smtp_send` | SMTP 465 implicit TLS / other STARTTLS | recipient/subject/body → acceptance | retain consent/auto-reply |
| DIVA `smtp_send_blocking` | multipart MIME | attachments + In-Reply-To/References | retain stronger DIVA path |

## QQ Bot

| Octos symbol | 方法/路径 | 请求/响应要点 | DIVA 目标 |
| --- | --- | --- | --- |
| `get_access_token` | `POST https://bots.qq.com/app/getAppAccessToken` | app_id/client_secret → token/expires_in | retain native TLS/cache |
| `fetch_gateway_url` | `GET https://api.sgroup.qq.com/gateway` | Bearer token → URL | add `/gateway/bot` compatibility only if fixture proves |
| identify | WS opcode 2 | token/intents/properties | verify official intents |
| resume | WS opcode 6 | token/session_id/seq | retain DIVA resume |
| heartbeat | WS opcode 1 / ACK 11 | sequence and interval | retain timeout/backoff |
| group send | `POST /v2/groups/{group_openid}/messages` | `msg_type=0`, `msg_seq`, optional `msg_id` | add group path + receipt |
| C2C send | `POST /v2/users/{user_openid}/messages` | same fields | add msg_seq/response parse |
| group event | `GROUP_AT_MESSAGE_CREATE` | group/member/content/id | add typed group ingress |
| C2C event | `C2C_MESSAGE_CREATE` | user/content/id | retain and unify envelope |
| media | no complete Octos wire path | no silent skip allowed | investigate official operation; otherwise unsupported |
