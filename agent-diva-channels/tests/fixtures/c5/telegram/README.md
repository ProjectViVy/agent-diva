# Telegram C5 Gate 2 fixtures

These redacted Bot API payloads are the offline wire shapes used by the native
`TelegramAdapter`. They contain no bot token, user PII, remote URL, or binary
media. The adapter keeps `getUpdates` polling, `getFile` media resolution,
caption limits, callback acknowledgement, reply/edit/delete and typing while
mapping every accepted event to a typed `ChannelEnvelopeV1`.

| Fixture | Proof target |
| --- | --- |
| `inbound-text.json` | private message identity, text, update/message IDs |
| `inbound-group-media.json` | supergroup topic/reply and photo/document metadata before bounded download |
| `callback-query.json` | callback data and `answerCallbackQuery` correlation |
| `api-responses.json` | `getFile`, send response IDs and rate-limit response shape |

The fixture set is intentionally a protocol contract. A test that exercises a
real mock Bot API must assert method path, JSON/form fields, HTTP status,
admission result, and the returned platform message ID rather than merely
checking that a request was attempted.
