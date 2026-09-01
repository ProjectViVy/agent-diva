# QQ Bot C5-V/C5-Q Gate 3 evidence

状态：`partial`，且 C5-Q 仍未完成。当前生产 intent mask 是 `33558528 = (1<<25)|(1<<12)`；
D-013 仍未用官方投递证据解决，D-014 media 仍明确 `blocked/unsupported`。

| 目标 | 源码 symbol / endpoint | fixture 与 Rust 测试 | 精确证据 | 生命周期/剩余项 |
| --- | --- | --- | --- | --- |
| token/gateway/identify | `access_token`, `fetch_gateway_url`, `run_connection`; `/app/getAppAccessToken`, `GET /gateway` | `token-response.json`, `gateway-response.json`; `wire_qq_gateway_admits_c2c_and_group_once_and_handles_heartbeat` | token POST 与 Bearer discovery path 被 fixture 精确匹配；Hello 后客户端发送 Identify | official intent delivery 与 token 429/refresh edge 仍 partial |
| C2C/group ingress | `event_identity`, `process_message_event`; `C2C_MESSAGE_CREATE`, `GROUP_AT_MESSAGE_CREATE` | `c2c-message.json`, `group-message.json`; same gateway wire test | C2C chat/sender 使用 user openid；group 使用 group/member openid；Fabric envelope 保留 event/message ID 与 chat kind | 实际 QQ permission/allowlist 及真实 event delivery 仍待 live |
| admission/dedup | `process_message_event`; Fabric admission | same fixtures; `admission_precedes_dedup_commit_for_c2c_event`, gateway wire test | duplicate C2C frame 不产生第二个 Fabric envelope；seen 只在 admission 成功后提交 | busy/cancel admission fault injection 尚需补 |
| C2C/group outbound | `send_envelope`, `post_message`; `/v2/users/{openid}/messages`, `/v2/groups/{openid}/messages` | `send-responses.json`; `wire_qq_outbound_routes_c2c_and_group_with_seq_reply_and_real_ids` | C2C/group 需显式 `qq.chat_kind`；body 有 `msg_type=0`、递增 `msg_seq`、首块 reply `msg_id`；`id`/`message_id` 真实回执进入 Accepted | 真实 permission/429 response 与 live response ID 尚待平台 smoke |
| heartbeat/stop | `run_connection`, `start`, `stop`; Gateway op 1/11 | `gateway-frames.json`; gateway wire test | server 对 heartbeat 回 op 11；测试等待后 cancel，listener/token/discovery task cleanly 结束 | RESUME/INVALID_SESSION cooldown/reconnect 需 live/额外 scripted fault fixture |
| media | `ensure_supported`, `static_capabilities` | `official-intents.json`, `send-responses.json`; `unsupported_media_fails_before_transport` | image/audio/video/file/card 首次 HTTP 前返回 typed `UnsupportedCapability`，无调用；D-014 保持 blocked | 未有审查过的官方 media endpoint+wire proof，不得升级 |

## D-013/D-014

`official-intents.json` 明确列出 GROUP_AND_C2C bit 25、DIRECT_MESSAGE bit 12 和未启用的
PUBLIC_GUILD_MESSAGES bit 30，状态为 `blocked_pending_official_delivery`。未将第三方资料当作
官方依据，也没有修改生产 mask。真实 harness 是 `tests/qq_live_harness.rs` 的 `#[ignore]`
测试；缺少仓库外凭据时只打印变量名并退出，不联网、不写入 secret。
