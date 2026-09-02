# QQ Bot C5-V/C5-Q Gate 3 evidence

状态：`partial`；本轮只审计 `QQ-01`、`QQ-02`，没有修改 adapter、fixture、测试、manifest、JSON
或 capability matrix。C5-Q 仍未完成；D-013 仍没有官方事件投递 transcript，D-014 media 仍为
`blocked/unsupported`。

本审计以固定 Octos checkout 为准：

```text
checkout: C:\Users\Administrator\Desktop\morediva\.workspace\octos
SHA:      5ea987813de4fd2afdd1d78f2106ad2868f0d923
```

本轮仅做只读源码、fixture、测试源码和既有验证日志核对；没有执行编译、测试、clippy 或网络
访问。`verified` 的判定仍要求代码、fixture、测试源码和此前记录的通过结果同时闭合。

## Frozen matrix target

`03-capability-gap-matrix.md` 的 `T` 是冻结 C5 目标，不是当前实现的自动证明。QQ 相关目标和
当前 native adapter 的静态声明如下：

| Gate row | Frozen target | DIVA static declaration |
| --- | --- | --- |
| `QQ-01` token/gateway/Identify | `IngressText=T`, `IngressDirect=T`, `IngressGroup=T`, `ReliabilityTokenRefresh=T`；Identify intent 必须有独立官方投递证据 | `static_capabilities` (`agent-diva-channels/src/adapters/qq.rs:183-202`) 声明 text/direct/group/token-refresh；生产 mask 是 `33558528 = (1<<25)|(1<<12)` |
| `QQ-02` heartbeat/resume/invalid/cooldown | `ReliabilityHeartbeat=T`, `ReliabilityResume=T`, `ReliabilityHealth=T`, `ReliabilityPacing=T`, `ReliabilitySupervisedRestart=T` | 同一 `static_capabilities` 声明上述五项；声明本身不能替代 lifecycle wire proof |

## Assigned-row disposition

| Row | Final disposition | Classification | Upgrade decision |
| --- | --- | --- | --- |
| `QQ-01` | `partial` | `evidence_gap`；D-013 子项 `blocked`；auth-expiry safe retry 若按 QQ spec 纳入本行则为 `implementation_gap` | 保持 `partial` |
| `QQ-02` | `partial` | `evidence_gap`；invalid-session cooldown 和 heartbeat-timeout 重连状态清理为 `implementation_gap` | 保持 `partial` |

## QQ-01 — token / gateway / Identify

### Octos reference and stable anchors

| Octos relative path | Symbol / anchor | Behavior at the pinned SHA |
| --- | --- | --- |
| `crates/octos-bus/src/qq_bot_channel.rs` | constants `TOKEN_URL`, `API_BASE`, `INTENTS` (`:32-47`) | Token endpoint is `https://bots.qq.com/app/getAppAccessToken`; API base is `https://api.sgroup.qq.com`; Octos source uses `(1<<25)|(1<<30)` for `INTENTS`. This is an implementation reference, not official proof of the correct production mask. |
| same | `QQBotChannel::get_access_token` (`:117-177`) | POSTs `{"appId": ..., "clientSecret": ...}`, parses `access_token` and string/number `expires_in`, and caches until expiry. There is no single-flight guard or typed 429 handling in this Octos path. |
| same | `QQBotChannel::fetch_gateway_url` (`:180-202`) | GETs `/gateway` with `Authorization: Bearer {token}` and parses `{ "url": ... }`. |
| same | `QQBotChannel::run_connection` / `process_frames` (`:411-645`) | Connects outbound WebSocket; on Hello (op 10), chooses Resume (op 6) when session state exists, otherwise Identify (op 2). READY stores session state; C2C/group dispatch is handled after the connection is identified. |
| `crates/octos-bus/src/channel.rs` | `Channel::start` / `send` (`:15-25`) and `send_with_id` (`:78-83`) | Listener is long-running; the base `send_with_id` delegates to `send` and returns `None`, so Octos itself does not prove a platform message ID receipt. |
| `crates/octos-bus/src/dedup.rs` | `MessageDedup::is_duplicate` (`:19-63`) | Shared reference cache is LRU with default capacity 1000 and TTL 60 seconds; empty IDs are never deduplicated. This is relevant to ingress ordering but does not prove QQ intent delivery. |

### DIVA implementation and exact wire shape

| Path / symbol | Current behavior |
| --- | --- |
| `agent-diva-channels/src/adapters/qq.rs:43-46`, `INTENTS` | Keeps the deployed mask `(1<<25)|(1<<12)` (`33558528`) pending D-013; it does not silently copy Octos' `(1<<25)|(1<<30)`. |
| `access_token` (`:263-370`) | Production POST is `https://bots.qq.com/app/getAppAccessToken` with JSON `{"appId": config.app_id, "clientSecret": config.secret}`. `TokenResponse` accepts string or numeric `expires_in`; a 429 reads `Retry-After` seconds and returns `AdapterError::RateLimited`; malformed body is `Execution { code: "token_response" }`; transport failure is `token_transport`; other non-success responses are `token_refresh`. The `refreshing` flag plus 50 × 20 ms polling is a local single-flight attempt, but the wait loop has no direct cancellation branch. |
| `fetch_gateway_url` (`:372-394`) | GET is `{api_base}/gateway` with `Authorization: Bearer {token}`. It requires a successful status and non-empty `{ "url": ... }`; malformed JSON is `gateway_response`, and status/empty URL is `gateway_discovery`. No 401 refresh-and-retry path is present. |
| `run_connection` (`:673-737`) | Receives Hello op 10, takes `d.heartbeat_interval` or the 41 s fallback, refreshes/reads the token, then sends either Resume op 6 or Identify op 2. Identify payload is `{"op":2,"d":{"token":"QQBot <token>","intents":33558528,"shard":[0,1]}}`; Resume payload is `{"op":6,"d":{"token":"QQBot <token>","session_id":...,"seq":...}}`. |
| `start` (`:776-821`) | Performs token and gateway discovery before every connection attempt; outer `tokio::select!` observes both context and local cancellation. Reconnect delay is 5/10/20/40/60 s capped at 60 s. |

### Fixture, test, request, response and receipt evidence

| Evidence | Exact location and result |
| --- | --- |
| Token response | `agent-diva-channels/tests/fixtures/c5/qq/token-response.json:1-4`: redacted `{ "access_token": "fixture-access-token", "expires_in": "7200" }`. It proves the parser shape only. |
| Gateway response | `agent-diva-channels/tests/fixtures/c5/qq/gateway-response.json:1-3`: redacted `{ "url": "wss://fixture.qq.gateway.example" }`. It proves the response shape only. |
| Intent record | `agent-diva-channels/tests/fixtures/c5/qq/official-intents.json:2-10`: records bit 25, bit 12, bit 30 and status `blocked_pending_official_delivery`; it is a repository review record, not an official delivery transcript. |
| Native wire test | `agent-diva-channels/src/adapters/qq.rs:1112-1179`, `wire_qq_gateway_admits_c2c_and_group_once_and_handles_heartbeat`. The HTTP fixture at `:1329-1368` matches `/app/getAppAccessToken` and `/gateway` in the request line and returns the token/gateway fixture shape; the WS helper at `:1400-1476` sends Hello and the client emits op 2. The test checks admitted C2C/group envelopes, duplicate C2C suppression, cancellation, discovery request count, and that recorded client frames contain op 2 and op 1. The prior verification record says the QQ adapter test set passed. |
| Payload assertion limits | The native gateway test does not assert the Identify `intents`, `shard`, or `QQBot` token field; the HTTP helper does not assert the Bearer header or the complete token request JSON. The separate outbound test only checks `clientSecret` in the token request at `:1253-1256`. |
| Gateway frame fixture use | `agent-diva-channels/tests/fixtures/c5/qq/gateway-frames.json:1-7` contains Hello, READY, ACK, RESUMED, op 7, and op 9 shapes, but the native gateway helper constructs its own frames and does not load this file. Its parseability is not a native QQ-01/QQ-02 wire assertion. |

### Lifecycle, retry and disposition

- Cache/expiry and a typed token 429 path are implemented, but no existing native test drives expiry, concurrent refresh, token 429/`Retry-After`, malformed token response, gateway 401, or auth-expiry refresh-and-safe-retry. The missing wire assertions are an `evidence_gap`; the absent auth-expiry retry is an `implementation_gap` if the QQ platform spec's safe retry requirement is included in this row.
- The current code's `INTENTS` differs from the pinned Octos expression. Octos source cannot decide which QQ intent bits are officially correct. D-013 therefore remains `blocked` until an official event-delivery transcript ties the chosen mask to actual C2C/group delivery; the internal `official-intents.json` cannot close it.
- No platform message receipt is produced by the token/gateway handshake itself. Outbound receipt behavior belongs to QQ-03/QQ-04 and remains preserved below; Octos' `send_with_id` default (`channel.rs:78-83`) would otherwise return `None`.

**QQ-01 result: `partial`; do not upgrade.**

## QQ-02 — heartbeat / resume / invalid session / cooldown / stop

### Octos reference and stable anchors

| Octos relative path | Symbol / anchor | Behavior at the pinned SHA |
| --- | --- | --- |
| `crates/octos-bus/src/qq_bot_channel.rs` | Hello and heartbeat in `process_frames` (`:448-540`) | Hello op 10 adopts `heartbeat_interval`; the loop sends heartbeat op 1 with the last sequence; op 11 is logged as ACK. Octos has no heartbeat ACK timeout in this path. |
| same | READY/RESUMED and dispatch (`:541-597`) | READY stores `session_id` and sequence; RESUMED marks the session active; `GROUP_AT_MESSAGE_CREATE` and `C2C_MESSAGE_CREATE` are dispatched to the inbound sender. |
| same | op 7/op 9 (`:599-613`) | Op 7 returns an error for reconnect. Op 9 clears `session_state` only when `d=false`; both resumable and non-resumable cases return an error to the outer loop. |
| same | `run_loop` (`:647-682`) and `Channel::stop` (`:691-751`) | Reconnect uses 5 s exponential backoff capped at 60 s with a 100-attempt ceiling. Stop sets an atomic flag; the frame loop checks it between select iterations. There is no distinct invalid-session cooldown. |
| `crates/octos-bus/src/channel.rs` | `Channel::stop` (`:38-41`), `finish_stream` (`:117-129`), `health_check` (`:241-247`) | Base contract has a no-op stop, default stream finalization through edit, and default `Unknown` health. QQ-specific lifecycle proof must therefore come from the adapter, not from the shared defaults. |
| `crates/octos-bus/src/dedup.rs` | `MessageDedup::is_duplicate` / `forget` (`:40-74`) | Reference dedup records at check time and provides `forget` for failed downstream processing. DIVA intentionally records its native `seen` marker only after Fabric admission for accepted events. |

### DIVA implementation and exact typed outcomes

| Path / event | Current behavior |
| --- | --- |
| Hello op 10 | `run_connection:710-722` sets the interval from `heartbeat_interval` or 41 s fallback, then sends op 6 when `session_id` exists and op 2 otherwise. |
| Heartbeat op 1 / ACK op 11 | `run_connection:689-705` sends `{"op":1,"d":sequence}` and sets `SessionState.awaiting_heartbeat_ack=true`; if the next tick arrives without ACK it returns `Execution { code: "gateway_heartbeat_timeout", retryable: true }` (`:693-699`). op 11 clears the flag and marks health `Healthy` (`:723-725`). |
| READY / RESUMED | `process_frame:546-572` stores the READY session ID and marks READY/RESUMED health `Healthy`. Sequence values are stored from `frame.s` at `:543-545`. |
| Reconnect / invalid session | `process_frame:580-600` maps op 7 to `gateway_reconnect`; op 9 maps to `gateway_invalid_session`, clearing `session_id` and `sequence` only for `d=false`. `start:804-816` applies the same 5/10/20/40/60 s retry delay to every connection error. |
| Stop / cancellation | `start:776-821` selects on context/local cancellation around token discovery, gateway discovery and `run_connection`; `sleep_or_cancel:895-905` interrupts backoff. `stop:868-875` cancels the local token, marks `running=false`, and sets health `Down`. There are no separately spawned heartbeat/token/backoff tasks in this adapter. |
| Health | `probe_health:739-764` performs token + `GET /gateway`; success sets `Healthy` and returns `Accepted` receipt with chat id `health` and no platform ID; failure sets `Degraded` and returns `health_probe`. No native QQ health test invokes this command. |

### Fixture and test evidence

| Evidence | Exact location and audit result |
| --- | --- |
| Shipped WS fixture | `agent-diva-channels/tests/fixtures/c5/qq/gateway-frames.json:1-7` has op 10, READY, op 11, RESUMED, op 7 and op 9 (`d=false`). It does not provide a transcript for a resumable op 9, a client op 6, backoff timing, cooldown, or stop task termination. |
| Native gateway test | `wire_qq_gateway_admits_c2c_and_group_once_and_handles_heartbeat` (`src/adapters/qq.rs:1112-1179`) uses a hard-coded WS helper (`:1400-1476`) that sends Hello with a 40 ms interval, responds to client op 1 with op 11, and sends READY plus C2C/group dispatches after client op 2. Assertions are `any(op==2)`, `any(op==1)`, duplicate suppression, and context cancellation; they do not assert op 6, op 11 receipt, op 7, op 9, invalid-session mode, backoff, health, or `QqAdapter::stop()`. |
| Legacy reconnect tests | `agent-diva-channels/tests/qq_reconnect_integration.rs:287-796` contains detailed tests for server close, invalid session, reconnect op, heartbeat timeout and invalid-session backoff, but imports and instantiates legacy `QQHandler` (`:1`) rather than native `adapters::qq::QqAdapter`. These tests are not evidence for the current adapter row. |
| Live harness | `agent-diva-channels/tests/qq_live_harness.rs:55-267` is `#[ignore]`, uses the public native factory/Registry/Fabric/pacing/supervisor, and checks C2C/group inbound plus final outbound receipts. It requires external credentials, does not force a gateway disconnect or invalid session, and therefore cannot close the native resume/reconnect/stop gap. |

### Gap classification

- **Evidence gap:** no native scripted transcript proves client Resume op 6 with the saved `session_id` and sequence, server `RESUMED`, op 7 reconnect, op 9 both `d=true` and `d=false`, bounded timing, health probe, direct `stop()`, or cancellation while token wait/backoff/heartbeat is pending. The shipped frame file is not consumed by the native wire test.
- **Implementation gap — invalid-session cooldown:** there is no invalid-session-specific cooldown state or delay. `start` uses the generic reconnect delay and has no attempt ceiling; `RECONNECT_MAX` only caps delay. The C5 target explicitly calls for bounded invalid-session cooldown/backoff.
- **Implementation gap — heartbeat state across timeout reconnect:** `awaiting_heartbeat_ack` is set before op 1 and cleared only on op 11. `run_connection` does not reset it on a new socket, on Hello, or when handling op 9. After a heartbeat-timeout error, the next connection can inherit `true` and fail at its next heartbeat tick before a new ACK. This needs a native fault transcript and a code decision before QQ-02 can be verified.
- **Cancellation is implemented but not fully evidenced:** the outer start select and `sleep_or_cancel` cover cancellation of connection, token/gateway discovery and backoff, while the native test only exercises context cancellation after message delivery. No test proves simultaneous termination of listener, pending heartbeat, token wait and backoff.
- **Dedup cross-check:** unlike Octos' check-time LRU/TTL (`dedup.rs:40-74`), DIVA's `process_message_event:621-670` checks `seen`, admits to Fabric, and inserts the ID only after successful admission. This is the correct admission ordering for accepted events; the existing direct unit test only covers C2C and the native wire test only replays C2C. It does not upgrade QQ-02 or change the separately owned QQ-03 disposition.

**QQ-02 result: `partial`; do not upgrade.**

## Existing evidence retained outside this audit

The following previously recorded QQ evidence is preserved and was not re-adjudicated by this worker:

| 目标 | 源码 symbol / endpoint | fixture 与 Rust 测试 | 精确证据 | 生命周期/剩余项 |
| --- | --- | --- | --- | --- |
| C2C/group ingress | `event_identity`, `process_message_event`; `C2C_MESSAGE_CREATE`, `GROUP_AT_MESSAGE_CREATE` | `c2c-message.json`, `group-message.json`; `wire_qq_gateway_admits_c2c_and_group_once_and_handles_heartbeat` | C2C chat/sender 使用 user openid；group 使用 group/member openid；Fabric envelope 保留 event/message ID 与 chat kind | 实际 QQ permission/allowlist 及真实 event delivery 仍待 live |
| admission/dedup | `process_message_event`; Fabric admission | `admission_precedes_dedup_commit_for_c2c_event`, gateway wire test | duplicate C2C frame 不产生第二个 Fabric envelope；seen 只在 admission 成功后提交 | busy/cancel admission fault injection 尚需补 |
| C2C/group outbound | `send_envelope`, `post_message`; `/v2/users/{openid}/messages`, `/v2/groups/{openid}/messages` | `send-responses.json`; `wire_qq_outbound_routes_c2c_and_group_with_seq_reply_and_real_ids` | C2C/group 需显式 `qq.chat_kind`；body 有 `msg_type=0`、递增 `msg_seq`、首块 reply `msg_id`；`id`/`message_id` 真实回执进入 Accepted | 真实 permission/429 response 与 live response ID 尚待平台 smoke |
| media | `ensure_supported`, `static_capabilities` | `official-intents.json`, `send-responses.json`; `unsupported_media_fails_before_transport` | image/audio/video/file/card 首次 HTTP 前返回 typed `UnsupportedCapability`，无调用；D-014 保持 blocked | 未有审查过的官方 media endpoint+wire proof，不得升级 |

## D-013 / D-014 and redaction

`official-intents.json` lists `GROUP_AND_C2C_EVENT` bit 25, `DIRECT_MESSAGE` bit 12 and disabled
`PUBLIC_GUILD_MESSAGES` bit 30, with `adapter_intents=33558528` and status
`blocked_pending_official_delivery`. The pinned Octos `INTENTS` expression is not treated as official
QQ documentation. D-013 therefore remains blocked. D-014 remains `blocked/unsupported` because no
reviewed official QQ media request/response wire proof exists; media commands fail before transport.

Fixture identifiers are redacted. The native adapter does not log the token or raw media; any future
live evidence must record only hashes/truncated identifiers and state transitions.
