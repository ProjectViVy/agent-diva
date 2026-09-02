# DingTalk：Octos → DIVA 深度扫描报告

> 扫描类型：C5-P2 只读事实扫描。DingTalk 必须同时记录 DIVA Stream 与 Octos
> webhook 两条路径；Octos webhook 不是 DIVA Stream 的替代实现。

每个差距表的 `Decision` 使用 `Port`、`Adapt`、`Retain-DIVA`、`Reject` 或 `Blocked`。

## 证据入口

| 侧 | 文件与定位 |
| --- | --- |
| DIVA | `agent-diva-channels/src/dingtalk.rs`：constants 37-38，token 225-276，upload 348-389，send_raw 389-444，send_media_ref 445-518，Stream event/ACK 669-820，send/stop 820-865，tests 965+ |
| DIVA policy/config | `src/base.rs` allowlist 73-220；`src/manager.rs` DingTalk 校验/注册；`agent-diva-core/src/config/schema.rs` DingTalkConfig、`dm_policy`、`group_policy` |
| Octos | `.workspace/octos/crates/octos-bus/src/dingtalk_channel.rs`：HMAC 34-78，webhook state/handler 83-128，session cache/target 130-204，event parse 250-328，webhook server 330-374，text send 384-425，tests 446-542 |
| Octos docs/config | `.workspace/octos/crates/octos-cli/src/commands/gateway/adapters/dingtalk.rs`；`book/src/channels.md`；`book/src/configuration.md`；`book/src/troubleshooting.md` |

## 双路径 API 与行为

### DIVA Stream/OpenAPI 主路径

| 操作 | 当前行为 | 目标/决策 |
| --- | --- | --- |
| OAuth | `POST https://api.dingtalk.com/v1.0/oauth2/accessToken`，token cache 预留刷新窗口 | `Retain-DIVA`，补 single-flight/auth-expired typed retry |
| Stream register | `POST https://api.dingtalk.com/v1.0/gateway/connections/open` | `Retain-DIVA`，必须记录 connection/response 和 ACK deadline |
| Stream WS | envelope、heartbeat、response ACK、reconnect | `Retain-DIVA + Adapt`：接入 bounded admission、supervisor、health |
| Group send | `POST https://api.dingtalk.com/v1.0/robot/groupMessages/send` | `Retain-DIVA`，保留 group conversation ID 和 Markdown/media |
| Private send | `POST https://api.dingtalk.com/v1.0/robot/oToMessages/batchSend`，body 使用 `robotCode` + `userIds` + `msgKey/msgParam` | `Retain-DIVA`，冻结 DM address；fixture 只验证 payload/response |
| Media upload | `POST https://oapi.dingtalk.com/media/upload?access_token={token}&type={image\|voice\|file}` multipart | `Retain-DIVA`，内容寻址读取、失败不得跳过 |
| Media send | image direct URL 或 upload 后 media_id，再调用 group/private send | `Adapt`：禁止发布未认证远程 URL；统一 attachment receipt |
| Policy | `dm_policy`、`group_policy`、allowlist；group 以 `cid` 识别 | `Retain-DIVA`，先授权再下载/ACK |

### Octos webhook/sessionWebhook 参考路径

| 操作 | Octos 行为 | DIVA 迁移决策 |
| --- | --- | --- |
| Signature | `timestamp + "\\n" + secret`，HMAC-SHA256 Base64；headers/query `timestamp`/`sign`/`signature` | `Port`：只借签名验证/失败语义，不替换 Stream |
| Inbound | `/dingtalk/webhook`，解析 senderStaffId/senderId/senderUnionId、conversationId、conversationType、sessionWebhook | `Adapt`：字段映射到统一 envelope；验证后才 admission |
| Session cache | 按 conversation ID 缓存短期 `sessionWebhook` | `Port`：可作为 Stream fallback/回执缓存，但必须有 TTL、脱敏和失效策略 |
| Send | 缓存 session webhook 优先，configured webhook 回退；签名 URL；文本 payload，3600 限制 | `Reject` 作为主实现；`Adapt` 仅采纳 session/sign/limit 语义 |
| Media | Octos 明确 text-only，媒体日志跳过 | `Reject`：不能让 DIVA 媒体能力退化或假成功 |

## 当前差距与实施决策

| 能力 | DIVA 当前 | Octos 证据 | 决策 | 必要 fixture |
| --- | --- | --- | --- | --- |
| Stream token/register | 已有 | webhook 实现较弱 | `Retain-DIVA` | token/cache/register |
| WS ACK/heartbeat/reconnect | 已有基础 | Octos 无 Stream 对应 | `Retain-DIVA + Adapt` | deadline/busy/reconnect |
| Group/DM policy | 已有 | conversationType/session cache | `Retain-DIVA` | policy matrix |
| Inbound signature | Stream envelope 已有；webhook 需核验 | HMAC 34-128 | `Port` 到 webhook 兼容层 | valid/missing/bad signature |
| Session webhook | DIVA 需补明确 cache | 169-204、477-542 tests | `Port` | cache preference/expiry |
| Text chunking | DIVA Markdown/text | 3600 Octos limit | `Adapt` | boundary-1/boundary+1 |
| Image/audio/video/file | DIVA upload/send 已有骨架 | Octos skips media | `Retain-DIVA` | multipart/upload/partial failure |
| Reply/edit/delete/reaction | 平台能力需按实际 API核验 | Octos false | `Reject` 未证实命令 | zero-side-effect unsupported |
| Health/supervision | 需接共享 runtime | Octos manager 不足 | `Adapt` | healthy/degraded/down/stop |
| Approval/permissions | DIVA Manager/Sandbox | Octos 无治理 | `Retain-DIVA` | identity/expiry/audit |

## 不可违反的迁移边界

1. 不能因为 Octos 只有 webhook/text，就删除 DIVA Stream、媒体、group/private policy。
2. Stream ACK 必须在 bounded admission deadline 内完成；失败要重试/重连，不 ACK-and-drop。
3. `sessionWebhook` 只能是已验证 conversation 的缓存目标，不得接受外部消息任意覆盖。
4. 媒体上传、下载和发送失败必须产生 typed partial/failure receipt。
5. 空 `allow_from` 继续遵循 DIVA 当前 allow-all 语义，但 `dm_policy/group_policy` 可进一步限制。

## 后续实施顺序

1. 先冻结 DIVA Stream envelope、ACK、conversation/session identity。
2. 补 HMAC webhook 兼容层和 session cache TCK，不改变 Stream 主路径。
3. 补 token single-flight、media upload/send receipt、policy-before-media。
4. 对 edit/delete/reaction 等无稳定 wire evidence 的命令保持 unsupported。
