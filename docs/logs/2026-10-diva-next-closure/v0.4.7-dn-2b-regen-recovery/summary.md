# v0.4.7 — DN-2B selected-turn regeneration + recovery controls

## Scope
Fix the regenerate-selected-turn bug, add rewind/fork, reconcile
cancel-after-restart, and expose start_goal through the existing work
controller.

## Changes (agent-diva `feat/dn-closure-wave1`)

- `api/vivy/contracts.ts` — `SessionMessageAttachment` (data_url form),
  `SessionEditResult` / `SessionRewindResult` / `SessionForkResult`
  (DN-0C-verified), `SessionMessage.attachments`.
- `api/vivy/client.ts` — `sessionEdit`, `sessionRewind`, `sessionFork`
  mutation calls.
- `state/vivy-chat.ts`:
  - `regenerate(assistantMessageId)` resolves the originating user turn
    (nearest preceding user row; never the last turn). Text → atomic
    `session/edit`; image → validate original bytes → `session/rewind`
    inclusive → `turn/start` with re-encoded attachments. Runs on the
    single sendChain mutation lane.
  - `rewind(userMessageId)` + `fork(messageId)`; `retryDraft` retains
    original content+attachments when the post-rewind send fails.
  - `onConversationInvalidate(reason)` local hook fired before
    session/new|load|delete, run/cancel and all history mutations —
    DN-6C voice subscribes without rewriting transitions.
  - `stop()` reconciles -32004 not-found via `run/get` readback;
    terminal status adopted through the projection.
  - `decidePlan` accepts `{objective, max_rounds}` for start_goal.
- `App.vue` — `regenerateMessage` now calls `vivyChat.regenerate` (the
  old "resend last user turn" fallback removed); `startGoalExecution`
  wires plan/decide start_goal with objective from the plan.
- `PlanApprovalCard.vue` — goal-rounds input + "批准并启动目标循环"
  emitting `start-goal`; ChatView + NormalMode forward the event.

## Rulings
- Atomic `session/edit` is the regeneration for text turns — one call,
  one new run, suffix replaced. Image turns deliberately use
  rewind+turn/start because edit carries no attachments field.
- `retryDraft` is manual-retry only; no automatic resubmission.

## Pending owner acceptance
DN-2B awaits owner acceptance; green tests ≠ acceptance.
