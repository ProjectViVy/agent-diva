# Verification

## Automated
- `just fmt-check`
- `just check`
- `just test`
- `cargo test -p agent-diva-agent context_budget --lib`
- `cargo test -p agent-diva-agent process_inbound_retries_once_after_context_overflow --lib`
- `cargo test -p agent-diva-agent process_inbound_returns_explicit_message_after_repeated_context_overflow --lib`
- `cargo test -p agent-diva-agent subagent --lib`
- `cargo test -p agent-diva-providers provider_error_detects_context_overflow_messages --lib`
- `cargo test -p agent-diva-core validate --lib`

## Behavior checks
- Verified overflow-like provider failures trigger one retry and then recover successfully.
- Verified repeated overflow-like failures return an explicit context-too-large user message after the single retry budget is exhausted.
- Verified subagent security tests still pass after reusing the new budgeted request path.
