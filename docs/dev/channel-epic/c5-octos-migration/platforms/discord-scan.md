# Discord：Octos → DIVA 深度扫描报告

> 扫描类型：C5-P2 只读事实扫描。Octos 以固定 SHA
> `5ea987813de4fd2afdd1d78f2106ad2868f0d923` 为准。

每个差距表的 `Decision` 使用 `Port`、`Adapt`、`Retain-DIVA`、`Reject` 或 `Blocked`。

## 证据入口

| 侧 | 文件与定位 |
| --- | --- |
| DIVA | `agent-diva-channels/src/discord.rs`：API base 22，allow/mention 299-376，附件 394-410，typing 444-481，gateway 497-674，stop 789-811，send 811-840，tests 842+ |
| DIVA policy/config | `src/base.rs`；`src/manager.rs`；`agent-diva-core/src/config/schema.rs` 中 `gateway_url`、`intents`、`mention_only`、`listen_to_bots`、`group_reply_allowed_sender_ids`、`allow_from` |
| Octos | `.workspace/octos/crates/octos-bus/src/discord_channel.rs`：Gateway/intents 153-181，attachment 85-126，send 197-224，edit/delete 228-264，reaction 266-304，embed 306-336，tests 396-456 |
| Octos contract | `.workspace/octos/crates/octos-bus/src/channel.rs`：typing/listening、bound edit/finalize、reaction/embed、health；`bus.rs`、`dedup.rs`、`docs/TESTING.md` |

## Gateway 与外部 API

| 操作 | Octos 行为 | DIVA 当前 | 目标/决策 |
| --- | --- | --- | --- |
| Gateway discovery | `GET https://discord.com/api/v10/gateway/bot`，Bearer bot token；响应 URL 拼 `?v=10&encoding=json` | `fetch_gateway_ws_url()` 497-523，支持配置 gateway fallback | `Retain-DIVA + Adapt`：优先认证 discovery，保留可测试 fallback，禁止全局 endpoint override |
| WS intents | `GUILD_MESSAGES | DIRECT_MESSAGES | MESSAGE_CONTENT` | config 可注入 intents | `Port`：实际 intent 与 `MESSAGE_CREATE` 字段缺失要产生诊断 |
| Hello/heartbeat | 接收 heartbeat_interval，发送 opcode 1，保存 sequence | `handle_gateway_message()` 606+ 已有 heartbeat | `Adapt`：接入共享 supervisor/health，保证 ACK 超时和取消可观测 |
| Identify/Resume | 处理 HELLO、IDENTIFY、RESUME、READY、RECONNECT、INVALID_SESSION、DISPATCH | DIVA 已有基础 Gateway，但必须逐 opcode fixture 核实 resume 状态 | `Port`：session ID + last acknowledged sequence，invalid session 清理状态 |
| `MESSAGE_CREATE` | 解析 author/bot、guild/channel/thread、content、mentions、message ID、attachments | 353-421 有入站解析和附件下载 | `Port`：完整 typed envelope，dedup 在 admission 后提交 |
| Typing | `POST /channels/{channel_id}/typing` | `start_typing()` 444-468 | `Port`：可取消任务，不能泄漏 detached task |
| Text send | REST create message，第一 chunk 可带 `message_reference` | 811-840 有 JSON REST 和 reply reference | `Adapt`：返回 snowflake `Accepted` receipt，首 chunk reply，其余不重复 reply |
| File send | `CreateAttachment::path` + `send_files` | 当前附件发送路径需核验 | `Port`：从 AttachmentStore 读取、multipart request capture、失败 typed |
| Edit/delete | REST edit/delete 真实消息 ID | Octos 有，DIVA legacy 未完整闭环 | `Port`：绑定 chat/message，跨频道编辑拒绝 |
| Reaction | Unicode 与 `<:name:id>`/`<a:name:id>`，添加/删除 reaction | DIVA 当前缺少完整接口 | `Port`：解析失败 unsupported，成功返回真实状态 |
| Embed/Card | `CreateEmbed` 标题/描述/颜色/字段 | DIVA current capability weak | `Adapt`：DIVA Card → Discord embed，字段/大小限制 fixture |
| Rate limit/error | 429 header/body `retry_after`，权限/transport error 保留诊断 | DIVA 有基础 REST error/retry 处理 | `Port`：映射 `RateLimited`，由共享 pacing 决定等待，不在 adapter 内盲睡 |
| Health | 鉴权 REST probe + Gateway heartbeat state | DIVA 有 Gateway 状态但需分级 | `Adapt`：Healthy/Degraded/Down/Unknown，静态 token 不宣称 refresh |

## 入站身份、群聊与媒体

- DM、guild、thread 必须用实际 channel/thread ID；不得依赖最近消息或进程全局 chat 状态。
- `author.bot` 过滤必须服从 DIVA `listen_to_bots` 配置；外部 payload 不能伪造 owner origin。
- `mention_only` 只影响群聊触发，`group_reply_allowed_sender_ids` 只放宽指定 sender；两者都不能绕过
  `allow_from`。
- 先做 sender/guild/channel policy，再下载附件。附件必须保留 Discord attachment ID、filename、MIME、
  size、digest 和原始 message ID。
- `MESSAGE_CONTENT` 缺失或附件 URL 失败时，不得把空内容视为成功入站。

## 差距矩阵与决策

| 能力 | DIVA 当前状态 | Octos 证据 | 决策 | 必要 fixture |
| --- | --- | --- | --- | --- |
| Gateway discovery/URL | 已实现，fallback 语义需固化 | `discord_channel.rs` Gateway setup | `Adapt` | `/gateway/bot` success/fail |
| Hello/heartbeat | 已有基础处理 | Gateway handler 153-181 | `Port` | interval/ACK/timeout/cancel |
| Resume/invalid session | 需补全状态证据 | handler reconnect paths | `Port` | resumable/non-resumable |
| MESSAGE_CREATE | text/attachment/mention 已有 | 85-126 | `Port` | DM/guild/thread/bot/mention |
| Dedup | 当前不具备 Octos 的明确 admission 时序 | `MessageDedup` 22+、tests 396+ | `Adapt` | duplicate + Fabric busy |
| Attachment ingress | legacy download | `download_media` 104-126 | `Port` | MIME/size/network failure |
| Send/reply/message ID | 基础 send | 197-224 | `Port` | JSON body + snowflake receipt |
| Edit/delete | 缺少完整统一合同 | 228-264 | `Port` | bound ID/404/permission |
| Reaction | 缺失或不完整 | 266-304、parse_emoji tests | `Port` | Unicode/custom add/remove |
| Embed/Card | 未统一 | 306-336 | `Adapt` | embed limits and capture |
| Typing | 已有 | 444-481 | `Retain-DIVA` | task cancellation |
| 429/pacing | 基础处理 | Octos REST behavior | `Port` | header/body retry_after |
| Health | 需分级 | `Channel::health_check` | `Adapt` | healthy/degraded/down |
| Approval/permissions | DIVA governance | Octos 无治理模型 | `Retain-DIVA` | policy + approval identity |

## 后续实施顺序

1. 先冻结 Discord address/correlation（guild/channel/thread/message/reply）与 attachment contract。
2. 统一 Gateway state machine、dedup、pacing、health；不引入 Serenity，仅保留现有 HTTP/WS transport。
3. 迁移附件、send/reply、edit/delete、reaction、embed、typing。
4. 将 Discord REST 429、权限和 malformed body 映射为稳定 typed error。
5. 参考 Octos `api_channel_property.rs` 验证并发 guild/thread 的绑定不漂移。
