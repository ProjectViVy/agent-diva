# Verification

Date: 2026-06-14

## Commands

- `cargo test -p agent-diva-laputa migration` — passed; 6 migration tests passed.
- `cargo test -p agent-diva-laputa` — passed; 33 crate tests passed.
- `cargo clippy -p agent-diva-laputa -- -D warnings` — passed.
- `cargo fmt --check -p agent-diva-laputa` — passed.
- `just fmt-check` — failed due to unrelated pre-existing formatting drift in `agent-diva-agent` and `agent-diva-autodream` files outside this story scope.
- `just check` — failed due to unrelated pre-existing compile/lint issues in `agent-diva-sandbox` and `agent-diva-agent` outside this story scope.

## Coverage

- New install with no legacy sources.
- Legacy upgrade with supported template backup and section mapping.
- Unsupported/future material preserved as explicit TBD payload.
- Failure injection before commit leaves previous state and sections readable.
- Idempotent rerun for the same legacy sources.
- Discovery coverage for relationship, history, and TBD task templates.
