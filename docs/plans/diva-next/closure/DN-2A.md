# DN-2A — Wire image send and admitted permission controls Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` task by task after implementation authorization. Use `superpowers:subagent-driven-development` only when delegation is explicitly authorized. Read the specification and this plan; check the parent index before starting.

**Goal:** Send actual supported images and confirmed session permission choices without discarding drafts or files.
**Architecture:** Extend the existing client/controller rather than add a parallel sender. Keep one mutation lane per session; validate the complete serialized ABI frame.
**Tech Stack:** Vue/TypeScript, VIVY existing chat RPCs, Vitest
**Spec:** [DN-C2](../p0-design.md), especially the sections cited below. [Contract ledger](../backend-separation-contracts.md) is the shared schema authority.
**Epic / requirements:** DN-2 / R-2, R-3.
**State and immediate dependencies:** [authoritative index](../index.md#executable-story-package). The index owns readiness; this file does not duplicate status.
**Baseline:** DIVA `f5866a0`, VIVY `1db8b55`, Laputa `dc6066e`; exact pins are in the index. Reconcile changed source before execution.

## Global Constraints

- One VIVY Agent; one Garden owner per configured profile; no restored Rust business crates, HTTP domain daemon, duplicate authority or hidden writer fallback.
- Keep the five C exports and `vivy_call` / `vivy:event`; speech bytes stay outside Go JSON. No old-data import, local voice, generic resources or pet-host restoration.
- Native final acceptance is Windows x64 and belongs to the owner; engineering evidence and unavailable checks are reported separately.
- Read each owning repository's instructions and claim its lock before edits. Paths below are relative to that repository. New files are **proposed**; generated output is regenerated, never hand-edited.
- Use current [execution constraints and handoff](../index.md#execution-contract). No product test or implementation has been performed by generating this plan.

## Review Focus

- Unsupported file/model preserves the draft and performs no mutation.
- Permission timeout/readback mismatch blocks sending instead of silently accepting an old policy.
- Base64 expansion and JSON overhead can cross the ABI limit despite a small source file.
- Duplicate clicks/session switches cannot create simultaneous mutation sequences.
- Admission timeout is reconciled from authoritative run/session state, never auto-replayed.

## Task 1: Deliver the Story boundary

### Files

- **DIVA / modify existing:** `agent-diva-gui/src/api/vivy/contracts.ts`
- **DIVA / modify existing:** `agent-diva-gui/src/api/vivy/client.ts`
- **DIVA / modify existing:** `agent-diva-gui/src/api/vivy/client.test.ts`
- **DIVA / modify existing:** `agent-diva-gui/src/state/vivy-chat.ts`
- **DIVA / modify existing:** `agent-diva-gui/src/state/vivy-chat.test.ts`
- **DIVA / modify existing:** `agent-diva-gui/src/components/ChatView.vue`
- **DIVA / modify existing:** `agent-diva-gui/src/components/ChatView.test.ts`
- **DIVA / modify existing:** `agent-diva-gui/src/App.vue`

### Interfaces

Consumes DN-0C image/permission fixtures. Produces proposed `TurnAttachment = {name?:string; mime_type:string; data:string}` and extension `VivyClient.turnStart(sessionId: string,text: string,attachments?: TurnAttachment[]): Promise<TurnStartResult>`. `setSessionPermission(sessionId,preset: 'cautious'|'smart'|'trusted')` calls `session/set_permission` and returns the owning session DTO.

Controller `send(text,attachments?,preset?)` confirms the chosen permission before `turn/start`; outcome unknown blocks send until authoritative session readback. Map existing UI labels explicitly; do not assume a label equals a wire enum. Validate actual image bytes/MIME/model support and `TextEncoder` bytes of the complete call request ≤4 MiB including base64/text/JSON framing.

### Ordered steps

- [ ] **Step 1:** Add `sendImageBytes`, `rejectOversizeFramedRequest`, `unsupportedFileKeepsDraft`, `permissionReadbackBeforeSend`, `duplicateSendSerialized` tests to existing client/controller suites. Assert exact fixture payload and zero turn calls on validation/permission failure.

- [ ] **Step 2:** Run client/controller tests red, then add the typed RPC methods and safe frame measurement. Remove the App.vue "attachments dropped" branch and text-only producer claim.

- [ ] **Step 3:** Read selected files into supported MIME/base64 bytes once; preserve draft/original data through validation and mutation ambiguity. Reuse existing model capability/settings evidence; unavailable capability is explicit.

- [ ] **Step 4:** Serialize permission→confirmed readback→turn start in the controller. Publish the admitted policy snapshot and restore run/approval state through the existing one projection owner.

- [ ] **Step 5:** Wire ChatView/App.vue events to the extended controller and cover native/preview unsupported/error states with EN/ZH copy in existing locale files.

- [ ] **Step 6:** Run focused/full GUI checks, developer-host image + policy smoke and commit `feat(chat): wire images and permission presets`.

### Verification

`pnpm --dir agent-diva-gui exec vitest run src/api/vivy/client.test.ts src/state/vivy-chat.test.ts src/components/ChatView.test.ts`; `just gui-test`, `just gui-build`. Expected: exact image/permission contract, full-frame boundary tests, preserved failed draft and one admitted send. Developer-native smoke uses an image-capable configured model; absent credential/capability is pending rather than a text-only success.

### Acceptance and handoff

Return source fixtures, frame-boundary and mutation-readback evidence. DN-2B consumes retained originating inputs and serialized mutation behavior; DN-P-C refreshes installed proof.
