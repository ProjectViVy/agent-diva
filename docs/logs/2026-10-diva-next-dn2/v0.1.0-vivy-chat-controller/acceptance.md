# Acceptance — DN-2 v0.1.0 (tasks 1–4)

## Met

- [x] Behavior tests for send/history/switch-session, complete-only
  text, reasoning/tool progress, cancellation racing completion, and
  an ambiguous send timeout (contract-verified transport fixtures).
- [x] `send_message`, `get_sessions`, `get_session_history`,
  `stop_generation` and all legacy core listeners replaced with the
  frozen session/turn/run methods; browser mock chat and duplicate
  App.vue orchestration removed.
- [x] Approval/question tests: approve/deny/expire, stale review,
  double decision, reopening while pending. Decisions bind backend
  review ids; rejections render; statuses resolve via snapshots/events.
- [x] Planning/work surfaces migrated against verified schemas only;
  missing capabilities recorded as blocked in TODOLIST — none
  fake-completed. Migrated legacy DTOs and stream startup commands
  deleted.

## Not met (explicit deferral)

- [ ] Packaged chat with a real configured model + policy-gated safe
  test tool in a temporary workspace — requires a provider key from
  the owner. Live streaming + pending-approval window-reopen
  acceptance gate is covered by this deferral.
