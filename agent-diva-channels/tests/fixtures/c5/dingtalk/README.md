# DingTalk C5 fixtures

These redacted fixtures describe the stable Stream/OpenAPI shapes used by the
native `DingTalkAdapter` tests. They contain no production credentials or
identifiers.

- `stream-callback.json` covers a group callback with a session webhook and
  explicit conversation/sender/message identity.
- `stream-transport.json` records the observed server Ping/client Pong
  transport transcript. DingTalk has no separate active heartbeat frame in
  the pinned Octos source, so the adapter never sends a proactive frame.
- `stream-reconnect.json` records register response/error shapes and the
  bounded reconnect/health/stop test parameters.
- `token-response.json` and `group-send-response.json` cover OAuth caching and
  the group send receipt.
- `media-upload-response.json` covers the content-addressed media upload
  response used by the retained DIVA media path.
- `media-send-responses.json` covers upload and group-send response IDs for
  image, audio, video, file, and a typed rate-limit failure.
- `inbound-media.json` covers the four typed inbound attachment shapes after
  DM/group policy has been evaluated.
- `hmac-session-webhook.json` covers the auxiliary Octos-compatible HMAC
  calculation and trusted/untrusted sessionWebhook cache inputs.

The adapter keeps the DIVA Stream path as the primary listener. Octos-derived
HMAC/session behavior is limited to the pure signature and cache helpers; no
text-only webhook downgrade is implied by these fixtures.
