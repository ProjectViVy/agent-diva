# HQ-03 Completion Summary

HQ-03 enables the production per-session actor path after completing request correlation and the
runtime-control contract.

- `AgentLoop::run` now submits Bus turns concurrently to persistent session workers. The HQ-01
  kernel remains the sole admission authority, so one session is FIFO/serial while independent
  sessions can overlap.
- Stop targets an optional request ID. A running target is cancelled; a matching queued target is
  reported as `queued_preserved`. Reset and Delete cancel admission first and perform destructive
  session cleanup only after the worker reaches quiescence. Direct/TUI execution defers the same
  cleanup until its active turn returns.
- `agents.defaults.session_admission` is additive and defaults to two waiters, a 30-second wait
  timeout, and a 600-second idle TTL. Existing configuration files require no migration.
- Admission events expose stable codes, phase, queue depth, wait latency, and session/request/trace
  identity. Manager chat streams, the general event SSE endpoint, CLI remote chat, and the desktop
  bridge preserve exact request ownership.
- The desktop change reuses the existing single active-stream authority. It does not introduce a
  second busy state or duplicate request lifecycle state machine.

Implementation commit: `011db1f3` (`feat: complete session admission runtime contract`).
