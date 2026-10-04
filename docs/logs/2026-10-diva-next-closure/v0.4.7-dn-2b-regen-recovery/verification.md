# v0.4.7 verification — DN-2B

Repo: agent-diva `feat/dn-closure-wave1`. Node v24.19.0, vitest 4.1.9.

## Named plan tests (Step 1)
- `regenerateSelectedTextAtomicEdit` — PASS. `session/edit` called with
  the ORIGINATING user message (u1/first q) for selected assistant a1;
  no turn/start, no rewind; admitted run subscribed, isTyping true.
- `regenerateImageInclusiveCutoff` — PASS. `session/rewind` at u2 before
  `turn/start`; the resent turn carries the original PNG bytes as wire
  `attachments` — no silent image→text conversion, no duplicated user
  message, no session/edit.
- `rewindSucceededSendFailedDraft` — PASS. Rewind landed, turn/start
  failed: `retryDraft` holds original text+bytes; the message view stays
  rewound (no re-appended copy).
- `cancelAfterRestartReconciles` — PASS. run/cancel -32004 → run/get
  readback → phase 'completed' adopted via snapshot; no throw, isTyping
  false.
- `goalUsesWorkController` — PASS. decidePlan('start_goal') sends
  plan/decide with objective+max_rounds+expected_version; exactly one
  plan/decide call, no second planner.

## Extra coverage
- `fork(messageId)` — inclusive fork opens the copied child session.
- `invalidateConversation(reason)` — fires session/load + session/delete.

## Gates
- `vitest run` named files: 69/69 pass.
- `just gui-test`: 518/518, 61 files.
- `just gui-build`: clean (existing >500kB chunk warnings only).
- `npx vue-tsc --noEmit`: clean.

## Honest limits
- No real-backend smoke here (fixture-derived handlers only); the
  GUI/native selected-turn smoke belongs to the DN-8C/owner pass.
- Fork/rewind message-chrome buttons are not yet surfaced — controller
  methods exist and are tested; UI entry points ride the DN-6C chrome
  pass. Recorded in TODOLIST.
