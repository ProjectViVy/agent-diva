# DingTalk Gate3 Evidence — C5 Octos Migration

Status: `partial` for DT-01, DT-03, and DT-04.

This update closes the smallest deterministic reliability loop in the native DingTalk adapter: the source symbols, platform-shaped fixtures, request assertions, response parsing, receipts/typed errors, and lifecycle evidence are now present in the DingTalk adapter test module. It does not claim live DingTalk certification.

## Evidence boundary

- Adapter: `agent-diva-channels/src/adapters/dingtalk.rs`.
- Pinned reference: `C:\Users\Administrator\Desktop\morediva\.workspace\octos`, commit `5ea987813de4fd2afdd1d78f2106ad2868f0d923`.
- Relevant pinned Octos source: `crates/octos-bus/src/dingtalk_channel.rs` contains the session-webhook/HMAC path and no DingTalk WebSocket Stream active-heartbeat implementation. Therefore the adapter does not invent a proactive heartbeat frame. A server WebSocket Ping is answered with the transport Pong required by the WebSocket protocol.
- Fixtures are deterministic redacted wire shapes. They are not proof of a live tenant, bot permission, or current DingTalk service behavior.
- No shared contract, config key, Cargo file, Manager, shared TCK/manifest, LOCK, TODO, decision document, or other channel was changed.

## DT-01 — Stream register, callback, ACK, reconnect, health, stop

### Source and endpoint anchors

- `DingTalkAdapter::register_stream` calls `POST /v1.0/gateway/connections/open` and parses the returned WebSocket endpoint and ticket.
- `DingTalkAdapter::run_stream_once` connects to the returned endpoint with `ticket`, parses callback messages, performs Fabric admission, and sends an ACK only after successful admission.
- `DingTalkAdapter::handle_stream_message` routes callback frames and observes cancellation. `DingTalkAdapter::is_processed`/`mark_processed` provide local duplicate suppression.
- `DingTalkAdapter::start` owns registration/reconnect/backoff and health transitions. `DingTalkAdapter::wait_backoff` is cancellation-aware. `DingTalkAdapter::stop` cancels the context and marks the adapter Down.
- `DingTalkAdapter::health` and `set_health` expose the lifecycle state. Stream transport failures are typed retryable errors; reconnect timing is owned by the start loop.

### Fixture and test evidence

- `stream-callback.json` is consumed by a local WebSocket server. The test asserts the registered endpoint/ticket, callback identifiers, Fabric envelope, and exact ACK `stream-message-redacted`.
- The same callback is delivered twice. The test asserts two protocol ACKs but one admitted envelope, demonstrating ACK protocol progress plus local duplicate suppression.
- `stream-transport.json` records a server Ping payload and the expected same-payload transport Pong. `stream_server_ping_gets_transport_pong_without_active_heartbeat` asserts the wire transcript and Healthy state. No client-initiated heartbeat frame is emitted.
- `stream-reconnect.json` supplies registration tickets, a first-close/second-success transcript, a 20 ms initial backoff, and health expectations. `start_reconnects_with_backoff_and_transitions_health` asserts two registrations, observable delay, Degraded after the first close, Healthy after the second connection, and Ping/Pong on the second connection.
- `stop_interrupts_reconnect_backoff_and_marks_health_down` asserts that stop interrupts a pending 60-second backoff and returns Down promptly.
- `stream_admission_failure_is_typed_and_does_not_commit_ack_state` asserts that a closed Fabric consumer yields typed `fabric_closed`, no ACK, no processed marker, and no session-webhook cache entry.

### Conservative limitation

DT-01 remains `partial`: there is no approved live/official DingTalk Stream transcript in this checkout proving tenant-specific reconnect semantics, server heartbeat cadence, resume behavior, or production ticket expiry. The fixture proves the implemented contract and safety properties only. Lead still needs to decide whether a real DingTalk tenant run or an official current protocol artifact is required before marking this gate verified.

## DT-03 — OAuth, media upload/send, partial failure

### Source and endpoint anchors

- `DingTalkAdapter::access_token` caches an OAuth token until its expiry safety window and serializes refreshes through the token lock, providing single-flight behavior.
- `DingTalkAdapter::authenticated_json`, `authenticated_multipart`, and `authenticated_bytes` attach the bearer/access token, parse JSON/bytes responses, classify HTTP failures as `AdapterError`, and retry a 401/403 exactly once after invalidating the cached token.
- OAuth endpoint: `POST /v1.0/oauth2/accessToken`.
- Media endpoint: `POST /media/upload?access_token=...&type=image|voice|file`; uploaded `media_id` values are parsed and then used by the typed send payloads.
- Direct/group send endpoints are selected by the address shape; group payloads use the platform-shaped `openConversationId`.
- Health probe endpoint: `POST /v1.0/robot/info`; success returns an accepted receipt and Healthy, while failure returns a typed retryable error and Down.
- `DingTalkAdapter::execute_send` preserves `DeliveryReceipt` for accepted work and maps a later failure after an accepted piece to typed `partial_delivery`. It deliberately does not claim retry-safe delivery.

### Fixture and test evidence

- `media-send-responses.json` contains redacted upload `media_id` values, send `messageId` values, and a 429 partial-failure response with `retry_after_seconds: 7`.
- `inbound-media.json` contains image, audio, video, and file events with DingTalk-shaped fields, trusted HTTPS media URLs, filenames, and MIME types. `fixture_media_events_download_and_admit_all_four_typed_parts` asserts URL downloads, Fabric admission, and all four typed `ContentPart` shapes.
- `media_image_audio_video_file_use_typed_group_wire_shapes` asserts multipart kind/MIME/filename, parsed upload IDs, group JSON shape, parsed send receipts, and absence of an idempotency field/header.
- `oauth_cache_expiry_and_single_flight_are_observable` runs concurrent token callers against one token fixture, asserts one token request, waits past the fixture expiry, and asserts a new token request.
- `media_upload_refreshes_oauth_once_after_401` and `media_download_refreshes_oauth_once_after_401` assert the exact token sequence across one 401 refresh and one retry.
- `media_multi_part_failure_is_typed_and_never_claims_retry_safe` asserts an accepted first part followed by typed `partial_delivery`, `retry_after = 7s`, retryable classification, and no fabricated idempotency claim.
- `token_is_cached_and_group_payload_uses_open_conversation_id` asserts the redacted platform-shaped group request and token reuse. `health_probe_returns_receipt_and_typed_down_failure` covers the success and failure health paths.

### Retry and idempotency boundary

The adapter retries only the authentication refresh case (401/403, once). It does not add an undocumented idempotency key/header, and the inbound `idempotency_key` remains local metadata. Consequently DingTalk media/send delivery is not advertised as retry-safe. Rate limits and other failures remain typed; a 429 can carry the parsed retry-after duration without triggering an unsafe automatic replay.

### Conservative limitation

DT-03 remains `partial`: no live DingTalk upload/send receipt, tenant permission check, or official current statement about an idempotency field was available in the pinned checkout. Lead still needs a real-platform smoke run and must retain the disabled retry-safe claim unless DingTalk supplies authoritative idempotency semantics.

## DT-04 — media admission policy, HMAC/sessionWebhook auxiliary validation

### Source and endpoint anchors

- `DingTalkAdapter::parse_event` parses sender, conversation type, conversation ID, message ID, and attachment metadata without downloading media.
- `DingTalkAdapter::policy_allows` applies DM/group policy before URL validation, OAuth media download, and Fabric admission. Explicit deny/closed/disabled modes fail closed.
- `DingTalkAdapter::admit_event` downloads media only after policy admission, maps the four typed media forms, and admits the resulting envelope to Fabric before caching a session webhook.
- `dingtalk_signature` and `verify_dingtalk_signature` implement the Octos-shaped auxiliary HMAC check. `cache_session_webhook_with_ttl` validates trusted HTTPS DingTalk hosts and expiry; it is not a webhook delivery fallback.

### Fixture and test evidence

- `hmac-session-webhook.json` records the fixed timestamp/secret/signature, trusted and untrusted `sessionWebhook` values, and header names. `hmac_fixture_and_session_webhook_cache_are_bounded_and_fail_closed` asserts valid/invalid signatures, trusted-host filtering, no overwrite by an untrusted URL, and TTL expiry.
- `parser_applies_allowlist_before_media_url_resolution` asserts a policy-denied event performs zero media download, HTTP, or store operations.
- `deny_group_policy_blocks_media_before_url_validation` asserts a denied group is rejected before an invalid media URL can be inspected.
- `stream_admission_failure_is_typed_and_does_not_commit_ack_state` proves Fabric admission precedes the ACK/cache side effects.
- There is no signed HTTP callback server or `sessionWebhook` fallback path in the adapter. HMAC/sessionWebhook are auxiliary verification/cache data only.

### Conservative limitation

DT-04 remains `partial`: the pinned Octos source is evidence for the auxiliary HMAC/session-webhook shape, not proof of a DingTalk Stream callback signature or a production webhook contract. Lead needs official/live evidence before expanding the trust boundary or adding any fallback.

## Validation

Focused validation for this update:

```text
cargo test -p agent-diva-channels dingtalk --lib  # 29 passed, 0 failed
cargo fmt --all -- --check                         # passed
cargo check -p agent-diva-channels --lib           # passed
cargo clippy -p agent-diva-channels --lib -- -D warnings # passed
git diff --check                                   # passed
```

The final command results and commit are reported with the handoff. Full workspace validation is intentionally not used as the DingTalk acceptance criterion while other channel lanes are active; any unrelated workspace failure must remain outside this focused change.
