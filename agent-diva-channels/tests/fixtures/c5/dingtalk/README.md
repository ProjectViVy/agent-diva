# DingTalk C5 fixtures

These redacted fixtures describe the stable Stream/OpenAPI shapes used by the
native `DingTalkAdapter` tests. They contain no production credentials or
identifiers.

- `stream-callback.json` covers a group callback with a session webhook and
  explicit conversation/sender/message identity.
- `token-response.json` and `group-send-response.json` cover OAuth caching and
  the group send receipt.
- `media-upload-response.json` covers the content-addressed media upload
  response used by the retained DIVA media path.

The adapter keeps the DIVA Stream path as the primary listener. Octos-derived
HMAC/session behavior is limited to the pure signature and cache helpers; no
text-only webhook downgrade is implied by these fixtures.
