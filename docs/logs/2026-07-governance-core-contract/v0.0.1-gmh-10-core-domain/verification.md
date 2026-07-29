# GMH-10 Verification

## Required validation

- `cargo test -p agent-diva-core governance`
- `cargo test -p agent-diva-core evolution`
- Existing Plan approval serialization tests
- Existing Sandbox approval serialization tests
- `just fmt-check`
- `just check`
- `just test`

## Result

- `cargo test -p agent-diva-core governance` — 7 passed.
- `cargo test -p agent-diva-core evolution` — 6 passed.
- `cargo test -p agent-diva-core legacy_plan_approval_request_json_contract_is_unchanged` — 1 passed.
- `cargo test -p agent-diva-sandbox legacy_command_approval_request_json_contract_is_unchanged` — 1 passed.
- `cargo test -p agent-diva-sandbox approval_coordinator` — 5 passed before the compatibility fixture was added; the final full workspace run includes all 6 approval coordinator tests.
- `just fmt-check` — passed.
- `just check` — passed with the existing `imap-proto` future-incompatibility notice.
- `just test` — passed after the final compatibility fixtures were added.

The test build reports existing unused-variable warnings in unrelated test
modules. Clippy with warnings denied remains green because those warnings are
limited to test-only code outside this story.

No CLI, GUI, or Channel smoke is required because GMH-10 adds Rust domain
contracts only and does not change executable or user-visible behavior.
