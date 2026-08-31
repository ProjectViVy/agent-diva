# QQ Bot：Octos → DIVA 深度扫描报告

> 扫描类型：C5-P2 只读事实扫描。Octos 基线固定为
> `5ea987813de4fd2afdd1d78f2106ad2868f0d923` (`v2.0.3-rc.9`)。

每个差距表的 `Decision` 使用 `Port`、`Adapt`、`Retain-DIVA`、`Reject` 或 `Blocked`。

## 证据入口

| 侧 | 文件与定位 |
| --- | --- |
| DIVA | `agent-diva-channels/src/qq.rs`：token 79-131，gateway 301-336，identify/resume 338-366，WS 431-845，event 855-962，send 1023-1074，tests 1177-1325 |
| DIVA tests/config | `agent-diva-channels/tests/qq_reconnect_integration.rs`：mock gateway 46-264、reconnect/resume 293-392、ping/pong 395-458、invalid session 461-552、reconnect opcode 555-635、heartbeat timeout 638-711、storm/cooldown 714-807；`agent-diva-core/src/config/schema.rs` QQConfig 1042-1053 |
| Octos | `.workspace/octos/crates/octos-bus/src/qq_bot_channel.rs`：constants 30-46，token 117-177，gateway 180-200，group send 205-248，C2C send 250-291，parse 296-391，WS 411-686，Channel impl 686-748，tests 780-876 |
| Octos contract/docs | `.workspace/octos/crates/octos-bus/src/channel.rs`、`bus.rs`、CLI gateway adapter/config、`book/src/channels.md`、`book/src/troubleshooting.md` |

## 外部 API 与 WS 行为

| 操作 | Octos 行为 | DIVA 当前 | 目标/决策 |
| --- | --- | --- | --- |
| Access token | `POST https://bots.qq.com/app/getAppAccessToken`，app_id/client_secret，expires_in 提前 60s 刷新 | 79-131 已有，支持测试 override | `Port`：保留 native TLS 与脱敏错误，统一 single-flight |
| Gateway discovery | `GET https://api.sgroup.qq.com/gateway`，Bearer token | 当前 301-336 只走 `/gateway/bot`，带 `Authorization`、`X-Union-Appid` | `Adapt`：先 `/gateway`，必要时兼容 `/gateway/bot`；endpoint 必须私有可测 |
| Identify | opcode 2，`token`、`intents`、properties | 338-353，当前 intents 与 Octos 需核对 | `Adapt`：先核实官方 intent 位；不能盲搬 `(1<<25)|(1<<30)` |
| Resume | opcode 6，token/session_id/seq | 355-366、529-547 | `Port`：last acknowledged seq/session 冻结 |
| Hello/heartbeat | opcode 10，interval；opcode 1 heartbeat；ACK opcode 11 | 449-749，最多 3 次未 ACK 后重连 | `Retain-DIVA + Adapt`：保留成熟 heartbeat/backoff，接入统一 health |
| Reconnect/invalid session | opcode 7/9；invalid session 清 session，指数退避和 cooldown | 675-845，已覆盖 storm/cooldown | `Port`：状态机、取消、测试 evidence；必要时 auth/gateway refresh |
| Group inbound | `GROUP_AT_MESSAGE_CREATE`：group_openid/member_openid/content/id | DIVA `is_group_event()` 后 864-910 明确拒绝 | `Port`：group @ 进入 typed envelope，policy 先于 admission |
| C2C inbound | `C2C_MESSAGE_CREATE`：user_openid/content/id | 915-962 已有 | `Retain-DIVA + Adapt`：统一 message/thread/origin、admission 后 dedup |
| Group send | `POST {API_BASE}/v2/groups/{group_openid}/messages`，`msg_type=0`、`msg_seq`、可选 `msg_id` | 当前无 | `Port`：群消息是 C5 必做，返回真实 receipt |
| C2C send | `POST {API_BASE}/v2/users/{user_openid}/messages`，同样带 seq/reply msg_id | 1035-1053 只发 text/msg_id | `Adapt`：补 msg_seq、response parsing、error body |
| Auth headers | group/C2C `Authorization: QQBot {token}`、`X-Union-Appid` | DIVA 当前 headers 需逐调用核实 | `Retain-DIVA`：不在日志暴露 token/open_id |
| Media | Octos 当前明确 text-only，媒体跳过 | DIVA 也无完整 media | `Reject` 假成功；先补官方 media wire path，证据不足时 typed unsupported |
| Health/receipt | Octos Channel 合同；发送错误返回失败 | DIVA 无独立 receipt/message ID parsing | `Adapt`：成功必须解析真实 response message ID，health/cancel 可观测 |

## 当前差距与实施决策

| 能力 | DIVA 当前 | Octos 证据 | 决策 | 必要 fixture |
| --- | --- | --- | --- | --- |
| Token/cache | 已有 | 117-177 | `Port` | cache hit/expiry/auth error |
| Gateway path | `/gateway/bot` | `/gateway` | `Adapt` | primary/fallback |
| Identify intents | `(1<<25)|(1<<12)` 现状 | `(1<<25)|(1<<30)` | `Investigate → Adapt` | official event delivery proof |
| Heartbeat/resume | 已有且测试较强 | 411-686 | `Retain-DIVA + Port` | ping/ACK/resume/invalid |
| C2C inbound | 可用 | 346-391 | `Retain-DIVA` | dedup/allowlist/message ID |
| Group inbound | 明确拒绝 | 296-344 | `Port` | group @ / unauthorized / dedup |
| C2C send | text/msg_id，无 seq/response ID | 250-291 | `Adapt` | request fields + response ID |
| Group send | 缺失 | 205-248 | `Port` | group address + msg_seq |
| Media ingress/egress | 缺失 | Octos text-only | `Reject` fake success | official API research + typed unsupported |
| Allowlist | 空列表 allow-all | 113-115 | `Retain-DIVA` | empty/non-empty |
| Approval/permissions | DIVA governance | Octos 无治理 | `Retain-DIVA` | group policy/approval identity |

## 身份、群聊和安全不变量

- C2C 用 `user_openid`，群聊用 `group_openid`；出站地址必须来自冻结 command，不能从最近入站推断。
- group 事件必须经过 sender/group policy 和 `allow_from`，再进入 Fabric；拒绝时不能下载媒体或触发审批。
- `origin` 永远是 `ExternalUser`；事件 payload 不得携带 owner/session authority。
- `msg_seq` 必须由 adapter 内原子序列分配，reply 的 `msg_id` 只来自当前关联消息。
- 认证、gateway、WS、媒体 URL、open_id、secret 全部脱敏；失败 body 截断后再记录。

## 后续实施顺序

1. 先核实官方 intents、group event 字段及官方媒体操作，形成 decision-log，禁止凭猜测改位图。
2. 补 group inbound/outbound、msg_seq、真实 response message_id 和 receipt。
3. 统一 dedup-after-admission、group/C2C address、权限-before-media。
4. 保留现有 heartbeat/resume/backoff/cooldown 测试，扩展 group、reply、health、cancel。
5. 媒体在有真实 wire path 和 fixture 前保持 unsupported，不允许静默跳过。
