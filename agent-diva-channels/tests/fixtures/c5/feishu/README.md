# Feishu/Lark C5 Gate 2 fixtures

These fixtures are deterministic, redacted protocol evidence for the native
`FeishuAdapter`. They are derived from the DIVA Feishu Protobuf WebSocket path
and the endpoint-level Octos scan at SHA
`5ea987813de4fd2afdd1d78f2106ad2868f0d923` (`v2.0.3-rc.9`). They contain no
production tokens, message bodies, signed URLs, or private identifiers.

| Fixture | Proof target | Adapter behavior |
| --- | --- | --- |
| `protobuf-event.json` | Protobuf DATA payload, direct/group identity, thread and dedup IDs | `handle_event_payload` creates one `ExternalUser` envelope and commits dedup only after Fabric admission |
| `protobuf-frame.json` | DIVA `PbFrame` header shape and protocol ACK metadata | `PbFrame` decode + `ack_frame` preserve `service`, `message_id`, `sum`, `seq`, and `biz_rt=0` |
| `webhook-url-verification.json` | Plaintext URL verification | `parse_webhook_event` permits only the challenge path and checks configured verification token |
| `webhook-encrypted-event.json` | Signed/encrypted webhook shape | Missing/bad signature and invalid AES-CBC payloads fail closed |
| `media-resource.json`, `media-types.json` | Typed image/file/audio/video resource response metadata | Resource bytes are bounded, MIME-labelled, content-addressed, read back and rejected on corruption through `AdapterServices` |
| `send-responses.json` | Send/reply/upload/edit/delete response IDs | HTTP operations return real `message_id`/`image_key`/`file_key`, parse edit/delete JSON `code`, or return typed failure |
| `error-responses.json` | Feishu-shaped 429, malformed, rejected and best-effort reaction responses | `Retry-After`, malformed JSON, non-zero platform codes and reaction failure isolation are asserted |
| `region-endpoints.json` | China/global/Lark endpoint mapping | Region is explicit; current shared config defaults production construction to China |
| `ack-deadline.json` | Admission deadline and heartbeat policy | Admission is bounded at 2s, heartbeat timeout at 300s, and cancellation interrupts reads/reconnect |
| `ws-lifecycle.json` | Callback endpoint, protobuf ping/event frame and ACK shape | Local WebSocket wire fixture asserts service ID, initial ping, event admission, `biz_rt=0` ACK, health and stop lifecycle |

The `webhook-encrypted-event.json` value is intentionally a shape fixture, not
an encrypted secret. The unit test generates deterministic AES-256-CBC bytes
from a test key and checks both successful decryption and bad-padding rejection.
The media fixtures intentionally contain no remote digest: only the injected
local `ChannelAttachmentStore` reference validation and readback can prove the
bytes written by this adapter.
