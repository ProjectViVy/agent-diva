# DN-4B — Bind primary authority and safe cognitive runtime lifecycle Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` task by task after implementation authorization. Use `superpowers:subagent-driven-development` only when delegation is explicitly authorized. Read the specification and this plan; check the parent index before starting.

**Goal:** Put session FrozenCore in actual primary model input and run cognition with per-run authority, durable capture and recovery fences.
**Architecture:** Extend existing Service/CognitiveBinding and embedded start/stop. Keep one scheduler, persist admitted pins, and share the host authority gate with human Mission writes.
**Tech Stack:** Go VIVY runtime, Journal/observers, INOFY/Laputa strategy
**Spec:** [DN-C2](../p0-design.md), especially the sections cited below. [Contract ledger](../backend-separation-contracts.md) is the shared schema authority.
**Epic / requirements:** DN-4 / R-2, R-3, R-4, R-6.
**State and immediate dependencies:** [authoritative index](../index.md#executable-story-package). The index owns readiness; this file does not duplicate status.
**Baseline:** DIVA `f5866a0`, VIVY `1db8b55`, Laputa `dc6066e`; exact pins are in the index. Reconcile changed source before execution.

## Global Constraints

- One VIVY Agent; one Garden owner per configured profile; no restored Rust business crates, HTTP domain daemon, duplicate authority or hidden writer fallback.
- Keep the five C exports and `vivy_call` / `vivy:event`; speech bytes stay outside Go JSON. No old-data import, local voice, generic resources or pet-host restoration.
- Native final acceptance is Windows x64 and belongs to the owner; engineering evidence and unavailable checks are reported separately.
- Read each owning repository's instructions and claim its lock before edits. Paths below are relative to that repository. New files are **proposed**; generated output is regenerated, never hand-edited.
- Use current [execution constraints and handoff](../index.md#execution-contract). No product test or implementation has been performed by generating this plan.

## Review Focus

- Current Persona edits affect new sessions, not reopened or rewound FrozenCore.
- Required authority overflow/corruption gates inference; optional recall failure is visible degradation.
- Supervisor is primary kind; stored origin/session must still exclude it from capture.
- Crash after workflow creation but before state save reconciles persisted operation identity.
- Hide keeps one host/loop; Quit stops admission/observers before Garden/storage close.

## Task 1: Deliver the Story boundary

### Files

- **VIVY / modify existing:** `internal/runtime/service.go`
- **VIVY / modify existing:** `internal/runtime/context.go`
- **VIVY / modify existing:** `internal/runtime/contextadapter.go`
- **VIVY / modify existing:** `internal/runtime/cognitive_binding.go`
- **VIVY / modify existing:** `internal/runtime/cognitive_service.go`
- **VIVY / modify existing:** `internal/runtime/inofy_executor.go`
- **VIVY / modify existing:** `internal/app/app.go`
- **VIVY / modify existing:** `internal/embedded/host.go`
- **VIVY / modify existing:** `internal/runtime/cognitive_recovery_test.go`
- **VIVY / create proposed:** `internal/runtime/primary_cognitive_context_test.go`
- **VIVY / create proposed:** `internal/app/embedded_cognitive_lifecycle_test.go`

### Interfaces

Consumes DN-LC's bound owner, primary-context port, source/sink and authority gate. Produces actual model-input projection, persisted `evolution.RunBinding`, scoped trigger state and lifecycle evidence for DN-4C. DN-C2 §§4.3–4.4 and C2-6 fix the port signatures; existing domain DTOs are retained.

Prepare seven FrozenCore v2 slots in order Mission/Identity/Relationship/Redline/User/Dream/Dark before optional evidence/history consumes budget; WORLD/ACTMEM remain tool-only. Per-run workflow binding pins current Mission/policy; primary session snapshot remains immutable. Unknown effects/authority changes/missing bindings durably block retries; safe transient failures have at most three attempts per window. Human cancellation pauses, never auto-retries.

### Ordered steps

- [ ] **Step 1:** Add tests `TestPrimaryFrozenCoreActualModelInput`, `TestMissionAdmissionFence`, `TestSupervisorCaptureExcluded`, `TestUnknownOutcomeNoNewAttempt`, `TestEmbeddedCognitionStartsOnce`. Assert actual scripted model request slots/order, same reopened digest, no WORLD/ACTMEM authority, and zero new operation keys after unknown outcomes.

- [ ] **Step 2:** Run targeted runtime/App tests red; fix primary preparation/reserved budget before inference in the existing admission/context path. Reject v1/corrupt/oversize required snapshots explicitly; fork creates a new snapshot, rewind does not.

- [ ] **Step 3:** Extend per-admission binding resolution and run-bound domain guards. Persist original pins in workflow input and verify before effects/recovery; serialize Mission human edits against effect check/write using the shared authority gate.

- [ ] **Step 4:** Capture committed user/final assistant content by Journal identity `run_id:journal_seq`; preserve source namespace/profile/scope/destination. Exclude supervisor/workflow/child provenance. Advance observer cursor only after durable acceptance, returning original receipt on replay.

- [ ] **Step 5:** Serialize manual/timer admission and state-save reconciliation. Add durable pause/recovery reason and safe retry bound; disabling/enabling never clears unknown state. Reconciliation requires scoped receipts plus run facts, not matching document text or an absent receipt.

- [ ] **Step 6:** Unify embedded and ordinary start helpers; start selected cognition once, stop admission before drains, close observers before sink/Garden and storage. Prove partial construction, crash/reopen and shutdown with pending effects.

- [ ] **Step 7:** Run focused tests plus the owning integration checks, commit runtime changes and return source/model-input/receipt/lifecycle evidence.

### Verification

From VIVY: `go test ./internal/runtime ./internal/app ./internal/embedded ./internal/observerhost -count=1`; run focused concurrency cases with `-race` on a supported native toolchain. Expected: real scripted model input carries required authority, stable durable receipts and per-run pins survive restart, and unknown/cancelled work creates no automatic new attempt.

### Acceptance and handoff

Return actual model-input fixture digest, authority/recovery test results, scoped state-key format and one-loop teardown evidence. DN-4C attaches human controls to these runtime ports. A failed persistence/reconciliation fence blocks affected execution; no force-clear recovery endpoint is introduced.
