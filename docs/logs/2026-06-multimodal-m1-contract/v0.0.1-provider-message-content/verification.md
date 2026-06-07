# M1 Provider Message Content Verification

## Commands

- `cargo test -p agent-diva-providers message_content`
  - Result: passed.
  - Coverage: legacy string content read/write, structured parts read/write, lossy text extraction, message content sanitization helpers.
- `cargo test -p agent-diva-providers test_sanitize_messages`
  - Result: passed.
  - Coverage: LiteLLM text sanitization for legacy text and structured text parts while preserving image parts.
- `cargo test -p agent-diva-providers test_apply_cache_control_structured_system_text_parts`
  - Result: passed.
  - Coverage: LiteLLM cache-control handling for already-structured system content.
- `cargo test -p agent-diva-providers convert_messages_uses_lossy_text_for_structured_parts`
  - Result: passed.
  - Coverage: Ollama text-only conversion for structured content.
- `just fmt-check`
  - Result: passed.
- `just check`
  - Result: passed.
- `cargo test -p agent-diva-providers`
  - Result: failed in existing discovery tests where mockito received 0 `/models` requests.
  - Scope note: failing tests are provider discovery HTTP mock tests, not M1 message-content tests.
- `just test`
  - Result: failed in `agent-diva-agent` test `skills::tests::test_default_builtin_dir_loads_skills`.
  - Scope note: failure depends on the default builtin skills directory being discoverable in this test environment and is unrelated to provider message content.

## Full Gate Status

Formatting and clippy gates pass. Full workspace tests are blocked by unrelated existing/environment-sensitive tests listed above.
