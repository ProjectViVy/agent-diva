# HQ-05 Acceptance

## Accepted automatically

1. Same-session FIFO and maximum executor concurrency of one are deterministic.
2. Independent sessions overlap while one provider is blocked.
3. Queue-full and wait-timeout reject before provider/tool/history/BML side effects.
4. Timeout/release races, dropped futures, task abort, and worker panic leave no ghost lease.
5. Stop preserves queued work; Reset/Delete cancel accepted session work and clean up after
   quiescence.
6. Manager, CLI, Bus, GUI, and provider observations retain exact request/trace/session ownership.
7. Existing configuration loads without migration and invalid zero durations fail validation.
8. Full Rust/GUI gates plus CLI, live GUI page, and embedded-Gateway smoke pass.

## Optional operator spot-check

1. With default limits, start one long-running turn and enqueue two more in the same chat. Confirm the
   desktop shows queued state and a fourth request receives a localized queue-full explanation.
2. Stop the running request and confirm the first queued request proceeds without reordering.
3. Reset the chat and confirm remaining queued placeholders become a localized reset explanation.
4. Open another chat while the first provider is blocked and confirm it can begin independently.
5. Run `agent-diva --config <path> config validate` after adding the documented configuration object.

These spot checks are useful for a deployment environment but are not blockers for repository Epic
closure because the same contracts are covered by deterministic automated and live-path tests.
