# Persistent execution context boundary

Plan execution context is now persisted by plan ID, revision, and session key. Approval creates a Pending context; the dedicated continuation initializes Retain, Clear, or Compact against an immutable transcript message-count boundary. Agent turns and restart recovery reuse the stored boundary and Compact summary.

The CLI DeepSeek provider-set fixture now follows the registry default `deepseek-v4-pro` while asserting that native endpoints keep an unprefixed model ID.
