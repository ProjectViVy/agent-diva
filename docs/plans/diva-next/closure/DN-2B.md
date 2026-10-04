# DN-2B — Fix selected-turn regeneration and recovery controls Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` task by task after implementation authorization. Use `superpowers:subagent-driven-development` only when delegation is explicitly authorized. Read the specification and this plan; check the parent index before starting.

**Goal:** Regenerate the selected originating turn correctly, expose Goal and reconcile cancel/reopen without duplicate sends.
**Architecture:** Keep existing chat/work controllers. Text uses atomic session/edit; images use validated original payload then inclusive rewind and normal turn/start.
**Tech Stack:** Vue/TypeScript, existing VIVY session/work/run APIs
**Spec:** [DN-C2](../p0-design.md), especially the sections cited below. [Contract ledger](../backend-separation-contracts.md) is the shared schema authority.
**Epic / requirements:** DN-2 / R-3, R-4, R-5.
**State and immediate dependencies:** [authoritative index](../index.md#executable-story-package). The index owns readiness; this file does not duplicate status.
**Baseline:** DIVA `f5866a0`, VIVY `1db8b55`, Laputa `dc6066e`; exact pins are in the index. Reconcile changed source before execution.

## Global Constraints

- One VIVY Agent; one Garden owner per configured profile; no restored Rust business crates, HTTP domain daemon, duplicate authority or hidden writer fallback.
- Keep the five C exports and `vivy_call` / `vivy:event`; speech bytes stay outside Go JSON. No old-data import, local voice, generic resources or pet-host restoration.
- Native final acceptance is Windows x64 and belongs to the owner; engineering evidence and unavailable checks are reported separately.
- Read each owning repository's instructions and claim its lock before edits. Paths below are relative to that repository. New files are **proposed**; generated output is regenerated, never hand-edited.
- Use current [execution constraints and handoff](../index.md#execution-contract). No product test or implementation has been performed by generating this plan.

## Review Focus

- Selected older assistant must use its original user input, not the last turn.
- Image regeneration never loses the image or duplicates its user message.
- Rewind success/send failure has a visible retryable draft; timeout is not rollback.
- Restarted cancel not-found requires run/session readback before claiming settled.
- Rewind keeps FrozenCore; fork starts a new snapshot and voice generation.

## Task 1: Deliver the Story boundary

### Files

- **DIVA / modify existing:** `agent-diva-gui/src/state/vivy-chat.ts`
- **DIVA / modify existing:** `agent-diva-gui/src/state/vivy-chat.test.ts`
- **DIVA / modify existing:** `agent-diva-gui/src/state/chat-message.ts`
- **DIVA / modify existing:** `agent-diva-gui/src/api/vivy/client.ts`
- **DIVA / modify existing:** `agent-diva-gui/src/api/vivy/contracts.ts`
- **DIVA / modify existing:** `agent-diva-gui/src/App.vue`
- **DIVA / modify existing:** `agent-diva-gui/src/components/ChatView.vue`
- **DIVA / modify existing:** `agent-diva-gui/src/components/planning/PlanApprovalCard.vue`

### Interfaces

Consumes DN-2A originating text/images and source-pinned edit/rewind/fork/cancel/work fixtures. Produces controller methods `regenerate(assistantMessageId: string)`, `rewind(userMessageId: string)`, `fork(messageId: string)` and existing `decidePlan('start_goal')` UI wiring. Add proposed `invalidateConversation(reason)` hook for DN-6C; it is local cancellation notification, not a new VIVY RPC.

Resolve selected assistant → originating user/run/input; never resend the last unrelated user turn. For images: validate bytes/capability/frame first → invalidate voice → rewind inclusively → turnStart original bytes. If rewind succeeds and send fails, retain retryable draft and visible rewound state. Ambiguous mutation blocks auto-resubmission and reconciles authority. Existing text edit has no images field.

### Ordered steps

- [ ] **Step 1:** Add `regenerateSelectedTextAtomicEdit`, `regenerateImageInclusiveCutoff`, `rewindSucceededSendFailedDraft`, `cancelAfterRestartReconciles`, `goalUsesWorkController` tests. Assert no last-turn append fallback and no silent image-to-text conversion.

- [ ] **Step 2:** Run the existing suites red; add exact typed edit/rewind/fork results from DN-0C and controller implementation under the session mutation lane.

- [ ] **Step 3:** Add conversation invalidation hook before regenerate/rewind/fork/load/delete/new-session and cancellation. Until voice connects it has no side effects; DN-6C subscribes rather than rewrites these transitions.

- [ ] **Step 4:** Rehydrate actual run/approval/work state on reconnect/reopen; read authoritative terminal state for cancel conflicts/not-found. Keep unknown admission outcome visible and no duplicate send.

- [ ] **Step 5:** Expose start_goal through the existing plan/work UI and controller; preserve current plan approvals and goal budgets. Do not create a second planner or INOFY mailbox feature.

- [ ] **Step 6:** Run GUI/native selected-turn/Goal/reopen smoke, commit `fix(chat): regenerate selected turns and reconcile recovery`, then release App.vue for cognition/voice edits.

### Verification

`pnpm --dir agent-diva-gui exec vitest run src/state/vivy-chat.test.ts src/api/vivy/client.test.ts src/components/ChatView.test.ts src/components/planning/PlanApprovalCard.test.ts`; `just gui-test`, `just gui-build`. Expected: each selection changes the intended authoritative history once, images survive, Goal affects real work, and restart/cancel produces truthful terminal/unknown state.

### Acceptance and handoff

Return originating-input preservation, invalidation-hook signature and selected mutation/recovery evidence. DN-4D and DN-6C consume these stable session transitions; App.vue integration is serialized even where DAG lanes are independent.
