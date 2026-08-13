# Summary

## Changes
- Fixed cron expression compatibility in `agent-diva-core/src/cron/service.rs` by normalizing 5-field expressions (e.g. `0 9 * * *`) to 6-field format expected by the parser (`0 0 9 * * *` style with seconds field prepended).
- Added execution observability logs for cron jobs:
  - scheduled due-trigger start
  - manual/scheduled execution start
  - execution success/failure completion
- Added regression test `test_compute_next_run_cron_five_fields_supported`.

## Impact
- Cron jobs created from GUI/CLI using common 5-field syntax now compute `nextRunAtMs` correctly and can be triggered by scheduler.
- Gateway logs now clearly show cron trigger and completion behavior for diagnosis.

- Fixed gateway cron delivery routing in CLI: cron jobs targeting channel gui now emit via API event stream (channel=api, chat_id=cron:*) so GUI background listener can receive callbacks.
- Changed cron gateway callback to publish `InboundMessage` and trigger one real agent turn per due job instead of directly pushing static outbound/event text.
- Added GUI compatibility bridge for cron-triggered dialogue: when payload channel is `gui`, message is routed as `api` channel with `cron:*` chat id for frontend background SSE consumption.
- Added cron safety guards in agent loop:
  - Detect cron-triggered turns by metadata/sender and disable tool loop to prevent recursive scheduling loops.
  - Mark cron-triggered turn origin explicitly via system instruction in prompt.
  - Persist cron-triggered input as `system` role in session history (instead of `user`) to avoid user-context pollution.
- Adjusted cron-triggered tool policy: only `cron` tool is filtered/blocked; other tools (e.g., web search/fetch) remain available during scheduled turns.
- Added execution-level hard guard: if model still emits `cron` tool call in cron-triggered turn, return explicit error instead of executing.
- Fixed cron stop-by-message reliability: `cron remove` now enforces session context isolation when context is available, preventing accidental removal of unrelated job IDs from other chats.
- Added regression test `test_cron_tool_remove_respects_context`.
