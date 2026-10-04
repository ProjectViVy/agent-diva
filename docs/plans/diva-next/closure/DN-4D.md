# DN-4D — Connect cognitive setup and scoped companion views Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` task by task after implementation authorization. Use `superpowers:subagent-driven-development` only when delegation is explicitly authorized. Read the specification and this plan; check the parent index before starting.

**Goal:** Let the user initialize/read/edit/review scoped cognition through real actions with clear current/frozen and recovery states.
**Architecture:** Reuse the existing Markdown editor and client. Create missing views as small current-scope consumers; keep business authority in the libraries and scheduling in VIVY.
**Tech Stack:** Vue/TypeScript, existing VivyClient, Vitest
**Spec:** [DN-C2](../p0-design.md), especially the sections cited below. [Contract ledger](../backend-separation-contracts.md) is the shared schema authority.
**Epic / requirements:** DN-4 / R-3, R-4.
**State and immediate dependencies:** [authoritative index](../index.md#executable-story-package). The index owns readiness; this file does not duplicate status.
**Baseline:** DIVA `f5866a0`, VIVY `1db8b55`, Laputa `dc6066e`; exact pins are in the index. Reconcile changed source before execution.

## Global Constraints

- One VIVY Agent; one Garden owner per configured profile; no restored Rust business crates, HTTP domain daemon, duplicate authority or hidden writer fallback.
- Keep the five C exports and `vivy_call` / `vivy:event`; speech bytes stay outside Go JSON. No old-data import, local voice, generic resources or pet-host restoration.
- Native final acceptance is Windows x64 and belongs to the owner; engineering evidence and unavailable checks are reported separately.
- Read each owning repository's instructions and claim its lock before edits. Paths below are relative to that repository. New files are **proposed**; generated output is regenerated, never hand-edited.
- Use current [execution constraints and handoff](../index.md#execution-contract). No product test or implementation has been performed by generating this plan.

## Review Focus

- Unavailable capability leaves settings navigable and primary send visibly gated.
- Save timeout preserves draft; current matching text alone cannot prove which operation committed.
- Current/frozen revisions require a new conversation to apply human edits.
- Hidden scopes and stale evidence/reviews never silently refetch different authority.
- Policy toggling cannot clear an unknown-outcome recovery block.

## Task 1: Deliver the Story boundary

### Files

- **DIVA / modify existing:** `agent-diva-gui/src/App.vue`
- **DIVA / modify existing:** `agent-diva-gui/src/components/SettingsView.vue`
- **DIVA / modify existing:** `agent-diva-gui/src/components/persona-memory/PersonaMarkdownEditor.vue`
- **DIVA / modify existing:** `agent-diva-gui/src/locales/en.ts`
- **DIVA / modify existing:** `agent-diva-gui/src/locales/zh.ts`
- **DIVA / create proposed:** `agent-diva-gui/src/api/cognitive.ts`
- **DIVA / create proposed:** `agent-diva-gui/src/api/cognitive.test.ts`
- **DIVA / create proposed:** `agent-diva-gui/src/state/vivy-cognitive.ts`
- **DIVA / create proposed:** `agent-diva-gui/src/state/vivy-cognitive.test.ts`
- **DIVA / create proposed:** `agent-diva-gui/src/components/persona-memory/PersonaSetupGate.vue`
- **DIVA / create proposed:** `agent-diva-gui/src/components/persona-memory/PersonaMemoryView.vue`
- **DIVA / create proposed:** `agent-diva-gui/src/components/MemoryView.vue`
- **DIVA / create proposed:** `agent-diva-gui/src/components/EvolutionView.vue`

### Interfaces

Consumes DN-4C captured C2-3 action fixtures. Produces proposed `CognitiveClient.call<T>(actionSuffix: CognitiveAction, input: CognitiveInput): Promise<CognitiveOutcome<T>>` over the existing client only, with read/mutation classification; `VivyCognitiveController` is a view projection, not scheduler/storage.

Before Persona setup, create a setup session via the existing controller and trusted human context; gate primary send while settings/setup remain accessible. Current Persona and session FrozenCore revisions are separate. Show ACTMEM scoped Pulse/Recap/Work; owner whole-save differs from agent Work patch. Search returns cards, expansion requires observed revision, mutation exposes receipt/canonical/index state. Evolution shows policy/eligibility/window/watermark/run/result/review/pause/recovery.

### Ordered steps

- [ ] **Step 1:** Add fixture-backed client/controller tests named `humanSetupGate`, `casConflictRetainsDraft`, `unknownSaveNoReplay`, `currentVersusFrozen`, `scopeAndReceiptStates`. Assert zero automatic writes after timeout and no hidden-scope rows.

- [ ] **Step 2:** Implement typed guarded action mappings in `cognitive.ts`; validate safe integers/outcomes and preserve existing domain aliases/errors. Do not route these actions through desktop native commands.

- [ ] **Step 3:** Implement the projection controller with session-scoped stale-response fencing, retained drafts and explicit authoritative readback. Use current runtime states; no timer-triggered evolution from Vue.

- [ ] **Step 4:** Create the proposed views and reuse PersonaMarkdownEditor. Mount Persona setup/editor/review, bounded ACTMEM sections, selected memory and evolution controls. General notebook/report/resource pages remain unavailable/deferred.

- [ ] **Step 5:** Serialize App.vue/controller/client integration after DN-2B; add EN/ZH labels and component cases for denied CAS/review, degraded backend, submitted/accepted/index-pending, disabled/manual enable and recovery-required.

- [ ] **Step 6:** Run GUI checks and a developer-native setup→primary-turn→edit→new-session→review/cancel/reopen smoke using temporary state; commit scoped UI changes. Final owner acceptance remains DN-8C.

### Verification

`pnpm --dir agent-diva-gui exec vitest run src/api/cognitive.test.ts src/state/vivy-cognitive.test.ts`; add focused proposed view tests beside those views. Then `just gui-test` and `just gui-build`. Expected: real action fixtures map without legacy aliases, setup gating/drafts/scopes/recovery states are honest, and actual developer host actions change the authoritative domain.

### Acceptance and handoff

Return action-fixture version, EN/ZH mappings, component results and sanitized developer smoke. Do not call newly created pages restored legacy parity; their scope is DN-C2 only. DN-P-C receives final UI/source commits for the packaged artifact.
