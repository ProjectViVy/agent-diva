# Acceptance — DN-2 task 5

- [x] Packaged chat with a real configured model in a temporary
  workspace (sensenova-6.8-flash-lite, streamed, Journal-persisted).
- [x] Policy-gated safe test tool: `write_file` under
  `workspace_write`+`ask` — proven not executed before approval and
  not executed after denial.
- [x] Cancellation reaches an authoritative terminal (`run.cancelled`,
  confirmed via `run/get`).
- [x] Journal/session readback agrees with streamed text.
- [x] Review center reconstructs settled decisions via snapshots —
  no resend (reopen semantic at the RPC layer).
- [x] No production credentials in evidence.
