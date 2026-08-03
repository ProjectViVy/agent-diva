# Verification

## Focused automated evidence

- `cargo test -p agent-diva-core state_pages_use_stable_request_id_cursor_and_replay_time` — passed.
- `cargo test -p agent-diva-core event_pages_replay_append_order_after_durable_cursor` — passed.
- `cargo test -p agent-diva-sandbox governed_timeout_materializes_expired_event` — passed.
- `cargo test -p agent-diva-manager handlers::approvals::tests` — 9 passed.
- `pnpm --dir agent-diva-gui test -- src/api/approvals.test.ts` — 2 passed.
- `pnpm --dir agent-diva-gui build` — passed; Vite reported only the existing large-chunk advisory.
- `cargo check -p agent-diva-gui` — passed.

The Manager tests exercise Command CAS/idempotent replay, cancellation/waiter wake-up, Plan consume-before-execution, Memory denial/domain transition, typed extractor errors, invalid cursor/limit rejection, all event-name mappings, and an actual SSE body frame carrying a durable cursor and request ID.

## Crate gates

- `cargo test -p agent-diva-core` — 669 passed.
- `cargo test -p agent-diva-sandbox` — passed.
- `cargo test -p agent-diva-laputa` — passed.
- `cargo test -p agent-diva-manager` — 98 passed, 1 ignored.
- `cargo clippy -p agent-diva-core --all-targets -- -D warnings` — passed.
- `cargo clippy -p agent-diva-manager --all-targets -- -D warnings` — passed.

## Delivery gates

- `cargo test -p agent-diva-gui` — passed.
- `pnpm --dir agent-diva-gui test` — passed.
- `just fmt-check` — passed.
- `just check` — passed.
- `just test` — passed. Two test-only unused-variable warnings in unrelated Channel/Tools fixtures remain non-failing and are recorded in `TODOLIST.md`.

The first combined gate process exceeded its outer 120-second budget after GUI tests, formatting, and Clippy had passed. A standalone `cargo test --all` passed in 105.8 seconds, followed by an exact `just test` rerun that passed in 101.6 seconds; no failure or residual process was hidden by the timeout.

Manual GUI smoke is intentionally deferred to the single final Epic/M3 acceptance pass, per the frozen Goal cadence; automated evidence does not substitute for that final observation.

`cargo clippy -p agent-diva-gui --all-targets -- -D warnings` exposes one pre-existing `field_reassign_with_default` finding in a Tauri test constructor. It is recorded in `TODOLIST.md`; the GMH-31 Tauri compile check passes.
