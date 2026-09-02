# QQ Bot C5-V/C5-Q Gate 3 evidence

状态：`partial`。本轮完成 QQ-01/QQ-02 的 native auth/gateway 本地 wire 闭环，并保留
QQ-01 的 D-013 官方事件投递证据缺口；QQ-02 的 recovery 行为可测，但
`ReliabilityResume` 静态 capability 按 Lead 复核要求保持 `false`。C5-Q 仍未完成，D-014
media 仍为 `blocked/unsupported`。

本轮只修改以下 QQ-owned 文件：

- `agent-diva-channels/src/adapters/qq.rs`
- `agent-diva-channels/tests/fixtures/c5/qq/`
- 本页

本轮没有修改 shared capability JSON、manifest、matrix、TCK、配置、Manager、TODO、LOCK 或
其他频道。

本证据以固定 Octos checkout 为准：

```text
checkout: C:\Users\Administrator\Desktop\morediva\.workspace\octos
SHA:      5ea987813de4fd2afdd1d78f2106ad2868f0d923
```

## Frozen matrix target and truthful declaration

`03-capability-gap-matrix.md` 的 `T` 是冻结目标，不是当前实现的自动证明。

| Row | Frozen target | Current DIVA declaration | Disposition |
| --- | --- | --- | --- |
| `QQ-01` | `IngressText=T`, `IngressDirect=T`, `IngressGroup=T`, `ReliabilityTokenRefresh=T`; Identify intent 仍需官方投递证据 | text/direct/group/token-refresh 为 `true`；生产 `INTENTS=33558528=(1<<25)|(1<<12)` | `partial` |
| `QQ-02` | heartbeat/resume/invalid/cooldown/health/pacing/supervised restart | heartbeat/health/pacing/supervised restart 为 `true`；`ReliabilityResume` 为 `false`，但 op6/RESUMED recovery 仍被 wire 测试 | `partial` |
| `QQ-05` | reviewed media request/response wire | media/card 不声明；命令在 HTTP 前返回 typed `UnsupportedCapability` | `blocked/unsupported` |

## QQ-01 — token / gateway / Identify

### Pinned Octos anchors

| Octos path / symbol | Pinned behavior |
| --- | --- |
| `crates/octos-bus/src/qq_bot_channel.rs` `TOKEN_URL`, `API_BASE`, `INTENTS` (`:32-47`) | token endpoint is `https://bots.qq.com/app/getAppAccessToken`; gateway base is `https://api.sgroup.qq.com`; Octos uses `(1<<25)|(1<<30)` as an implementation reference only. |
| `QQBotChannel::get_access_token` (`:117-177`) | POSTs `appId`/`clientSecret`, accepts string/number `expires_in`, and caches a token. The pinned path has no single-flight or typed 429 contract. |
| `QQBotChannel::fetch_gateway_url` (`:180-202`) | GETs `/gateway` with `Authorization: Bearer {token}` and parses `{ "url": ... }`. |
| `QQBotChannel::run_connection` (`:411-645`) | On Hello op 10, chooses Resume op 6 when session state exists, otherwise Identify op 2. |

### DIVA symbols and exact local wire

`QqAdapter::access_token_with_cancel` POSTs the exact QQ token shape
`{"appId": config.app_id, "clientSecret": config.secret}` through the QQ native-TLS client. It
accepts string/number `expires_in`, checks the 60-second refresh margin, uses a Notify-backed
single-flight owner, and uses a Drop guard to clear the owner on success, failure, cancellation,
or dropped futures. A token 429 returns `AdapterError::RateLimited` with integer `Retry-After`.
Malformed bodies return typed `Execution { code: "token_response" }`.

`fetch_gateway_url_with_cancel` sends `GET {api_base}/gateway` with exactly
`Authorization: Bearer {token}`. A gateway 401 invalidates the matching cached token and
`discover_gateway_with_cancel` performs one safe refresh-and-retry. Non-success responses and
empty/malformed URLs remain typed gateway errors. Both request and body waits observe context or
adapter cancellation.

`run_connection` sends Identify with the existing DIVA mask and exact payload shape:

```json
{"op":2,"d":{"token":"QQBot <token>","intents":33558528,"shard":[0,1]}}
```

The mask is deliberately not changed to Octos' `(1<<25)|(1<<30)`. `official-intents.json` remains
an internal review record, not official delivery proof.

### Fixtures, tests and request assertions

| Evidence | Proof |
| --- | --- |
| `tests/fixtures/c5/qq/auth-wire.json` | Exact token POST body, Bearer gateway request, Identify and Resume shapes. |
| `token-response.json` | Normal string `expires_in` response. |
| `token-expired-response.json`, `token-refreshed-response.json` | Immediate expiry causes a second token POST and returns the refreshed token. |
| `token-rate-limit-response.json` | Real token 429 body shape paired with `Retry-After: 7`. |
| `gateway-response.json` | `{ "url": ... }` gateway response shape. |
| `wire_qq_gateway_admits_c2c_and_group_once_and_handles_heartbeat` | HTTP mock asserts exact token JSON and Bearer header; WS mock asserts `QQBot` token, `intents`, `shard`, op 2/op 1, C2C/group identity, admission-before-dedup and cancellation. |
| `wire_qq_token_expiry_refreshes` | Expiry and refresh request count/response behavior. |
| `wire_qq_token_refresh_is_single_flight` | Concurrent callers share one token POST and receive the same token. |
| `wire_qq_token_429_is_typed_and_next_refresh_can_proceed` | 429 + Retry-After typed error and refresh-owner release. |
| `wire_qq_gateway_refreshes_once_after_bearer_expiry` | 401 Bearer rejection, token refresh, and second Bearer request assertion. |

QQ-01 remains `partial`: the local wire proves the chosen request/response and event shapes, but
D-013 still needs an official event-delivery transcript tying the deployed intent mask to C2C/group
delivery. The pinned Octos intent expression cannot close that external evidence gap.

## QQ-02 — heartbeat / resume / invalid session / cooldown / stop

### DIVA lifecycle behavior

- On every new socket and every Hello op 10, `awaiting_heartbeat_ack` is reset before the new
  heartbeat schedule. Op 1 records the pending ACK; op 11 clears it and marks health `Healthy`.
  A missed next tick returns typed `gateway_heartbeat_timeout`, so a timeout cannot poison the next
  socket.
- READY stores the session ID and sequence. With saved state, Hello emits Resume op 6 with
  `QQBot` token, `session_id`, and last sequence; RESUMED marks the connection healthy. Recovery is
  implemented and measurable, but static `ReliabilityResume` remains false until the capability
  disposition is accepted by Lead.
- Op 7 returns typed `gateway_reconnect` and preserves resumable state. Op 9 returns typed
  `gateway_invalid_session`; `d=true` preserves session/sequence for a Resume attempt, while
  `d=false` clears both and forces Identify. Both paths clear pending heartbeat ACK state.
- `RetryPolicy` keeps generic reconnect backoff bounded at 5/10/20/40/60 seconds, while invalid
  sessions use a separate 5/15/30/60-second progression. Five consecutive invalid sessions enter a
  bounded 300-second cooldown before the streak resets. The policy also caps generic reconnects at
  100 attempts.
- `start` and `sleep_or_cancel` observe context and adapter cancellation during token refresh,
  gateway discovery, WebSocket connect/read, heartbeat, reconnect backoff, and invalid-session
  cooldown. `stop` cancels the adapter, marks health `Down`, and releases the listener path.
- `probe_health` uses the same token/Bearer gateway check and returns an `Accepted` receipt with
  chat id `health`; failures set `Degraded` and remain typed. No media path is added.

### Lifecycle fixtures and native wire tests

`gateway-frames.json` covers Hello, READY, heartbeat ACK, RESUMED, op 7, and both op 9 modes.
`gateway-lifecycle.json` is consumed by the scripted WS mock and supplies the exact lifecycle frame
shapes. The mock records every client frame and asserts the expected first client opcode.

| Test | Native evidence |
| --- | --- |
| `wire_qq_gateway_resumes_after_reconnect_and_receives_resumed` | READY → op 7 → reconnect; asserts second connection op 6 with token/session/sequence and RESUMED. |
| `wire_qq_resumable_invalid_session_preserves_resume_state` | op 9 `d=true` preserves state and next connection emits op 6. |
| `wire_qq_non_resumable_invalid_session_identifies_again` | op 9 `d=false` clears state and next connection emits op 2, not op 6. |
| `wire_qq_heartbeat_timeout_reconnect_resets_pending_ack` | Suppressed ACK causes timeout; next resumed connection still sends heartbeat op 1 and accepts ACK. |
| `wire_qq_invalid_session_backoff_enters_bounded_cooldown` | Three invalid sessions use increasing invalid-only gaps, then no fourth connection occurs during the test cooldown. |
| `wire_qq_stop_interrupts_pending_reconnect_backoff` | Direct `stop()` terminates the pending backoff and prevents another connection. |
| `wire_qq_token_wait_cancels_without_leaking_refresh_state` | Cancellation interrupts a stalled token HTTP request while the refresh owner is active. |
| `wire_qq_health_probe_returns_receipt_after_bearer_gateway_check` | Bearer gateway health probe, `Accepted` receipt, and `Healthy` projection. |

The focused native adapter run for this page passed:

```text
cargo test -p agent-diva-channels adapters::qq --lib
22 passed; 0 failed
```

## Preserved QQ-03/QQ-04 behavior and explicit boundary

The patch retains C2C/group identity (`user_openid`, `group_openid`, `member_openid`), empty/non-empty
allowlist semantics, policy-before-admission and admission-before-dedup, explicit
`qq.chat_kind` outbound routing, `msg_seq`, first-chunk reply `msg_id`, real response `id`/
`message_id` receipts, Unicode-safe 4000-character chunking, and final-only stream send behavior.
No media upload, media parsing, card transport, or interaction capability is implemented or
claimed. `QQ-05`/D-014 remains `blocked/unsupported`.

## Remaining external and Lead-owned gaps

- QQ-01 remains `partial` until D-013 official event-delivery evidence is supplied; the local
  `official-intents.json` must not be treated as that evidence.
- `ReliabilityResume` is intentionally false in QQ `static_capabilities` even though recovery wire
  behavior is implemented and tested. Lead must update the shared
  `agent-diva-channels/tests/fixtures/c5/capability-evidence.json` QQ-02 disposition and any
  shared capability manifest/matrix entries (`evidence-manifest.md` and
  `03-capability-gap-matrix.md`) to match this truthful declaration. This QQ commit does not touch
  those files.
- C5-Q/live QQ credentials, permissions, official event delivery, and real response IDs remain
  external validation work. D-014 media remains unsupported.

Fixture identifiers are redacted or local-only. The adapter does not log tokens, secrets, raw media,
or full open IDs; future live evidence must use hashes/truncated identifiers and state transitions.
