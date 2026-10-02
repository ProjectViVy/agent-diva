# DN-2 tasks 1–4: VIVY chat controller + App.vue rewiring (v0.1.0)

## What landed

- `state/vivy-chat.ts` — `VivyChatController`, the single orchestration
  authority for chat/session/approval/question/plan. Owns the VIVY
  client + `VivySessionProjection`; composes the message timeline from
  session/messages history + per-run `reduceRunMessages` events;
  exposes `subscribe`/getters so the component layer stays a projection.
- `state/chat-message.ts`, `state/vivy-run-messages.ts` — UI message
  model + run-event reducer (streaming text/reasoning, tool cards,
  retry/stall markers, terminal-only answers).
- `state/vivy-session.ts` — projection gained `subscribe`/`emit`;
  pending-interaction tracking binds real domain event types
  (`user.question_*`, `tool.approval_*`).
- `api/approvals.ts` — `review/list` items map onto the drawer's
  `ApprovalView` (`reviewToApprovalView`); legacy stream guards and the
  event/list DTOs deleted.
- `api/planning.ts` — `session/work` + `session/todos` map onto
  `PlanRuntimeState` (`workToPlanRuntime`); report/stream DTOs deleted.
- `api/vivy/client.ts` — review/work/todo/plan/goal methods +
  `contextCompact`; all mutations marked `mutation: true` so timeouts
  surface `unknownOutcome`.
- `api/vivy/instance.ts` — shared client instance for call-only
  components (e.g. CompactionSettings).
- `App.vue` — 2761 → 1059 lines. send/history/switch/stop/regenerate/
  sessions go through the controller; all 18 legacy `listen` blocks,
  `start_background_stream`/`start_approval_stream`, ask-user polling,
  browser mock chat, session cache, and streaming placeholder logic
  deleted.
- `components/NormalMode.vue` — session rename forwards to the
  controller (was a dead `update_session_title` invoke); resume-plan
  emit wired through (previously swallowed).
- `components/settings/CompactionSettings.vue` — `/compact` button
  re-pointed to verified `context/compact {session_id}`.

## Verified schema claims

- `review/list`/`review/respond` cover approval approve|deny and
  question answer|cancel — one surface, backend ids.
- `session/work` carries plan review state; plan approval is NOT a
  review item (only `approval` + `question` kinds exist).
- `plan/decide` actions: `execute_once` / `revise` / `start_goal`;
  `goal/resume`, `goal/pause`, `plan/leave` share `workParams`.
- `context/compact {session_id}` exists; busy session = -32009 conflict.

## Deferred / blocked

- Task 5 (packaged live-model chat + policy-gated safe tool) needs a
  provider key — owner action.
- Attachments on send (turn/start is text-only), permission modes,
  in-place regenerate (session/rewind exists, unwired), automatic
  session title generation, plan `start_goal` UI trigger — recorded in
  TODOLIST.
- Live streaming + pending-approval window-reopen acceptance still
  requires a real model chain.
