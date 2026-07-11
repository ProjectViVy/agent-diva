# Verification

## Commands

```text
cargo test -p agent-diva-core default_and_new_share -- --nocapture
cargo test -p agent-diva-core drafts_and_execution -- --nocapture
cargo check -p agent-diva-core -p agent-diva-manager -p agent-diva-agent
```

## Results

| Check | Result |
| --- | --- |
| `default_and_new_share_process_registry_for_approve` | pass |
| `drafts_and_execution_are_isolated_by_session_and_replaced` | pass |
| `cargo check` core/manager/agent | pass (existing manager dead_code warning only) |

## Deferred

- Full desktop GUI smoke (generate plan → click execute) requires a running gateway + LLM; not run in this iteration.
- `just fmt-check` / full workspace `just ci` not re-run; scoped Rust checks only.
