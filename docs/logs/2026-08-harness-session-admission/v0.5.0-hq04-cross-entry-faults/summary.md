# HQ-04 Completion Summary

HQ-04 completes cross-entry fault injection and makes session-admission backpressure truthful at
the desktop boundary.

- Provider retry and final-wire observation is request scoped through Tokio task-local observers.
  Legacy provider listeners remain as a compatibility fallback, while concurrent Bus turns no
  longer overwrite each other's request/trace correlation.
- Persistent session workers are generation identified and supervised. A worker panic removes only
  its own registry generation, resolves its running and queued requests as
  `session_worker_unavailable`, and allows a later request to create a healthy replacement worker.
- Actual Bus tests inject provider stalls, retries, queue-full, wait-timeout, Stop, Reset, and worker
  panic. They prove rejection before provider side effects, no ghost lease, no cross-session event
  leakage, deterministic draining, and recovery after failure.
- Manager SSE and CLI end-to-end tests preserve queued/running admission events with exact
  session/request/trace identity before the final response.
- The desktop uses its existing active-stream request ID as the sole ownership authority. It shows
  queued state, clears it on running, ignores foreign/stale observations, translates all five
  terminal codes, and preserves the active request when Stop reports `queued_preserved`.

Implementation commits:

- `2e3553fb` (`fix: isolate provider request observers`)
- `5176ac18` (`fix: supervise session worker failures`)
- `0226571e` (`feat: surface session admission backpressure`)
