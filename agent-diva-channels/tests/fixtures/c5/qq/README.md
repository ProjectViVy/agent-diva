# QQ Bot C5 Gate 3 fixtures

These redacted Official QQ Bot API v2 shapes are the contract for the native
`QqAdapter`. The gateway uses an outbound WebSocket; REST uses the token
endpoint plus `Bearer` gateway discovery and `QQBot` message authorization.
The adapter supports C2C and group text ingress/egress, Unicode-safe 4000
character chunking, message-ID deduplication, `msg_seq`, reply `msg_id`,
heartbeat, resume and invalid-session recovery. Recovery is wire-tested but
`ReliabilityResume` remains false until the native lifecycle proof and the
shared capability disposition are accepted by Lead.

QQ media upload is deliberately not advertised. Octos is text-only and the
current repository has no reviewed official file-upload fixture; attempts to
send image/audio/video/file/card/interaction parts fail before transport and
remain tracked by C5-Q/D-014.

| Fixture | Proof target |
| --- | --- |
| `token-response.json` | access-token response and expiry parsing |
| `token-expired-response.json` / `token-refreshed-response.json` | expiry-triggered refresh |
| `token-rate-limit-response.json` | token endpoint 429 body |
| `auth-wire.json` | exact token/gateway/Identify/Resume request shapes |
| `gateway-response.json` | `/gateway` discovery |
| `gateway-frames.json` / `gateway-lifecycle.json` | Hello/Identify/Ready, heartbeat ACK, resume, reconnect and invalid-session frames |
| `c2c-message.json` | C2C identity, admission and dedup key |
| `group-message.json` | group open ID/member open ID and group policy |
| `send-responses.json` | C2C/group request fields and truthful response ID |
