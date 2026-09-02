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
| `group-policy.json` | DM, reply-to-bot, @mention, command, and ignored group updates |
| `media-matrix.json` | voice, audio, video, and document Bot API message shapes |
| `polling-responses.json` | `getMe`, `getUpdates`, empty batch, and auth failure shapes |
| `keyboard-unsupported.json` | Telegram inline keyboard shape rejected before transport |
| `api-responses.json` | `getFile`, send/edit/delete/action IDs and rate-limit response shapes |

The fixture set is intentionally a protocol contract. A test that exercises a
real mock Bot API must assert method path, JSON/form fields, HTTP status,
admission result, and the returned platform message ID rather than merely
checking that a request was attempted. Inline keyboards remain outside the
native v1 command contract: the fixture is parsed only to prove that a card
request still returns `UnsupportedCapability` without a private `reply_markup`
side channel.
