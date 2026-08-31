# Feishu/Lark：Octos → DIVA 深度扫描报告

> 扫描类型：C5-P2 只读事实扫描。Octos 基线固定为
> `5ea987813de4fd2afdd1d78f2106ad2868f0d923` (`v2.0.3-rc.9`)。

每个差距表的 `Decision` 使用 `Port`、`Adapt`、`Retain-DIVA`、`Reject` 或 `Blocked`。

## 证据入口

| 侧 | 文件与定位 |
| --- | --- |
| DIVA transport/security | `agent-diva-channels/src/feishu.rs`：frame 60-95，region/base 39-41，dedup 221-259，token 277-334，WS 336-591，event/media 593-771，reaction 774-806，send/reply 954-1035，tests 1074-1188 |
| DIVA webhook/config | `feishu.rs` webhook handler 442-592；`src/manager.rs` 78-84；`agent-diva-core/src/config/schema.rs` 814-832；`src/base.rs` 73-220 |
| Octos transport | `.workspace/octos/crates/octos-bus/src/feishu_channel.rs`：frame/region 33-55/314-326，crypto 329-431，webhook 442-577/1410-1473，token/ws 614-789/1214-1410，media 791-894，send/reply 903-1011，parse 1023-1179，send_with_id/edit/delete 1560-1647 |
| Octos tests/contracts | `feishu_channel.rs` 1740-2142；`channel.rs` 15-247；`tests/api_channel_property.rs`；`tests/jsonl_replay_thread_binding.rs`；`book/src/troubleshooting.md` |

## 外部 API 与协议行为

| 操作 | Octos 行为 | DIVA 当前 | 目标/决策 |
| --- | --- | --- | --- |
| Tenant token | `POST {region_base}/auth/v3/tenant_access_token/internal`，app_id/app_secret，缓存 TTL | `get_access_token()` 277-334，固定 China base | `Port` region/domain；`Adapt` single-flight/cache/error |
| WS endpoint | `POST {domain}/callback/ws/endpoint`，返回 URL/client config | `GET`/简化 endpoint 336-371，固定 China | `Port`：region-aware endpoint，保留 DIVA protobuf framing |
| WS frames | Protobuf-like header、Ping、heartbeat、ACK、重连 | `PbFrame/PbHeader` 60-95，run 373-591 | `Adapt`：协议等价但接入 bounded admission/health |
| Webhook | `/webhook/event`，plaintext URL verification；加密事件先验签再解密 | 当前配置有 key/token，但模式与安全闭环需核验 | `Port`：显式 mode，默认不开放 webhook |
| Signature | `X-Lark-Signature`、timestamp、nonce；SHA-256 计算 | DIVA 现状需补 fail-closed | `Port`：缺任一 header 或不匹配均拒绝 |
| AES event | encrypt_key SHA-256 派生 AES-256-CBC，解密后解析 JSON | DIVA 字段存在但未完整接入 | `Port`：加密事件无 key 或解密失败为 typed error |
| Inbound text/post | sender open_id、chat_id/chat_type、message_id、reply/thread | 593-712 有消息解析，但 typed envelope 不完整 | `Adapt`：填充统一 address/correlation |
| Image/file/audio/media/sticker | `/im/v1/messages/{message_id}/resources/{file_key}?type=image|file` | image marker 较完整，其他类型偏占位 | `Port`：typed attachment + AttachmentStore；marker 只能兼容 |
| Image upload | `POST {base}/im/v1/images` multipart `image_type=message` | 出站骨架需证据化 | `Port`：上传失败不得静默跳过 |
| File upload | `POST {base}/im/v1/files` multipart `file_type=stream`, `file_name`, file | 出站骨架需证据化 | `Port`：返回 file_key 和 receipt |
| Send | `POST {base}/im/v1/messages?receive_id_type=open_id|chat_id`，interactive card/plain | `send_message_returning_id()` 954-1035 | `Port`：解析真实 message_id |
| Reply | `POST {base}/im/v1/messages/{parent_message_id}/reply` | 954-1011 已有 | `Retain-DIVA + Adapt`：冻结 reply_to，不从最近消息推断 |
| Edit/delete | `PATCH {base}/im/v1/messages/{id}` / `DELETE .../{id}` | legacy 需统一 | `Port`：bound message receipt |
| Reaction | `POST /im/v1/messages/{id}/reactions` | DIVA 774-806 作为 seen marker | `Retain-DIVA`：保持 best-effort seen；另行决定 command capability |
| Health/reconnect | token probe、WS ping/heartbeat、连接状态 | 有 heartbeat/reconnect | `Adapt`：映射 Healthy/Degraded/Down，取消时停止所有任务 |

## 入站、群聊和权限

- `sender.open_id` 是外部身份；`chat_id`、`chat_type`、`message_id` 和 reply root 必须进入统一 envelope。
- 群聊事件先检查 allowlist 和 policy，再下载资源和 admission。
- 空 allowlist 保持 DIVA 当前 allow-all；非空严格限制 sender。
- webhook plaintext URL verification 是唯一可在缺签名时放行的路径；普通事件必须 fail-closed。
- region 必须是显式配置（China/Global/Lark），不能把用户提供的任意 URL 当作产品 endpoint。

## 当前差距与实施决策

| 能力 | DIVA 当前 | Octos 证据 | 决策 | 必要 fixture |
| --- | --- | --- | --- | --- |
| Region/domain | 固定 China | 314-326 | `Port` | cn/global base assertions |
| Token/cache | 已有缓存 | token helper/TOKEN_TTL | `Adapt` | cache hit/expiry/single-flight |
| WS frame/ACK | 已有 protobuf WS | 33-55、1214-1410 | `Retain-DIVA + Adapt` | ping/ACK/deadline/reconnect |
| Webhook mode | 配置字段但未完成闭环 | 442-577、1410-1473 | `Port` | URL verification/signature/encryption |
| Inbound typed media | image marker，file/audio/media 不完整 | 1023-1179 | `Port` | all message types + resource download |
| Upload/send/reply | 部分骨架 | 816-1011 | `Port` | multipart + response IDs |
| Edit/delete | 需统一 | 1581-1647 | `Port` | PATCH/DELETE and bound correlation |
| Reaction seen | 已有 | DIVA 774-806 | `Retain-DIVA` | best-effort failure not inbound failure |
| Dedup | 已有 message key | Octos `MessageDedup` | `Adapt` | admission-before-commit |
| Card/Markdown | 已有 card/table rendering | interactive send | `Retain-DIVA` | card payload limits |
| Approval/permissions | DIVA governance | Octos 无治理 | `Retain-DIVA` | approval identity/expiry and allowlist |

## 安全与生命周期不变量

1. Feishu ACK deadline 必须大于 bounded admission deadline；忙时重试/重连，不能 ACK-and-drop。
2. token 只在过期或 auth error 后安全刷新一次；非幂等发送不得盲重试。
3. 签名、解密、媒体下载和卡片错误必须带稳定 code，日志脱敏。
4. dedup 标记在 Fabric admission 成功后提交；重复事件不重复触发工具或审批。
5. 取消必须终止 WS read、heartbeat、token wait、media upload/download 和 reconnect。

## 后续实施顺序

1. 冻结 region/mode/config 与 address/correlation 字段。
2. 完成 webhook 签名/AES fail-closed 和 WS ACK TCK。
3. 完成 typed media download/upload、send/reply/edit/delete、receipt。
4. 保留 DIVA reaction seen 和 card/table 渲染，不把其误报为通用 reaction capability。
5. 完成 mock server、加密事件、region、dedup、ACK busy 和 cancellation fixtures 后再合入。
