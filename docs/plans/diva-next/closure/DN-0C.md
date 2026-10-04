> **Historical under DN-W3 (2026-10-04).** This file preserves prior scope,
> contracts and evidence. Its Tauri/C ABI/Rust host execution steps and
> scheduling/readiness statements are inactive. Use the [current index](../index.md)
> and its W0–W7 plans. Domain behavior and explicitly deferred scope remain
> reference material; prior package evidence is not Wails acceptance.

# DN-0C — Capture current chat and console contracts Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` task by task after implementation authorization. Use `superpowers:subagent-driven-development` only when delegation is explicitly authorized. Read the specification and this plan; check the parent index before starting.

**Goal:** Produce redacted, source-pinned request/response fixtures for existing chat, work and OBS producers.
**Architecture:** Extend the existing in-process capture harness. Capture current producers separately from the historical core transcript; final packaged capture is repeated on DN-P-C.
**Tech Stack:** Go, existing DialControl harness, JSON fixtures
**Spec:** [DN-C2](../p0-design.md), especially the sections cited below. [Contract ledger](../backend-separation-contracts.md) is the shared schema authority.
**Epic / requirements:** DN-0 / R-2, R-3, R-9.
**State and immediate dependencies:** [authoritative index](../index.md#executable-story-package). The index owns readiness; this file does not duplicate status.
**Baseline:** DIVA `f5866a0`, VIVY `1db8b55`, Laputa `dc6066e`; exact pins are in the index. Reconcile changed source before execution.

## Global Constraints

- One VIVY Agent; one Garden owner per configured profile; no restored Rust business crates, HTTP domain daemon, duplicate authority or hidden writer fallback.
- Keep the five C exports and `vivy_call` / `vivy:event`; speech bytes stay outside Go JSON. No old-data import, local voice, generic resources or pet-host restoration.
- Native final acceptance is Windows x64 and belongs to the owner; engineering evidence and unavailable checks are reported separately.
- Read each owning repository's instructions and claim its lock before edits. Paths below are relative to that repository. New files are **proposed**; generated output is regenerated, never hand-edited.
- Use current [execution constraints and handoff](../index.md#execution-contract). No product test or implementation has been performed by generating this plan.

## Review Focus

- Image regeneration must preserve original bytes; text edit has no attachment field.
- A timed-out mutation has an unknown outcome; capture readback instead of a duplicate write.
- Run waiting and active model calls are separate; null usage differs from measured zero.
- Diagnostic partial append can lack a structured accepted-prefix receipt.
- A source fixture is not proof the selected DLL implements the same contract.

## Task 1: Deliver the Story boundary

### Files

- **VIVY / modify existing:** `internal/app/dn0_capture_test.go`
- **VIVY / verify existing:** `internal/rpc/control.go`
- **VIVY / verify existing:** `internal/rpc/diagnostics.go`
- **VIVY / verify existing:** `internal/rpc/tokenstats.go`
- **DIVA / create proposed:** `docs/plans/diva-next/fixtures/closure-chat-obs.json`
- **DIVA / modify existing:** `docs/plans/diva-next/backend-separation-contracts.md`

### Interfaces

Consumes C2-1 and the existing `App.DialControl(ctx, notifications)`/scripted provider. Produces `fixtures/closure-chat-obs.json` with source SHA, recipe/Generation when available, capture kind, method, request, result/error and ordered notifications. A source-host capture is labelled `source_host`; it never claims to be bundled-runtime proof.

Freeze `turn/start` image bytes/MIME, `session/set_permission`, `session/edit` (text only), inclusive `session/rewind`, `session/fork`, cancel/reopen, `plan/decide` with `start_goal`, `trajectory/session`, `stats/tokens`, `diagnostics/logs` and GUI append. Read full owning structs; do not substitute PR prose (`trajectory/get` or shorthand coverage names) for source fields. Child references use actual existing child read DTOs; no mailbox or new child-control work.

### Ordered steps

- [ ] **Step 1:** Add capture assertions: `rewind` removes the target user message inclusively; edited text starts one run; image bytes survive the replay path. Include unsupported MIME, busy session and malformed params errors.

- [ ] **Step 2:** Extend `TestDN0CaptureCoreTranscript` into a separately named closure capture case; run it with `DN0_CAPTURE_OUT=/tmp/diva-closure-chat-obs.json go test ./internal/app -run TestDN0CaptureClosureTranscript -count=1 -v`. Preserve the existing historical capture case and explicitly redirect the new output away from core-rpc.json. Record any unavailable source dependency instead of fabricating frames.

- [ ] **Step 3:** Capture stats v2, trajectory v2 and diagnostic success/empty/partial/gap/error cases against disposable stores. Add `projection_version == 2`, safe sequence numbers and response-shape assertions.

- [ ] **Step 4:** Write the redacted fixture and exact symbol/field mappings into C2-1; retain historical `fixtures/core-rpc.json` unchanged. Check schema round-trip and absence of secret/audio/raw-thinking sentinels.

- [ ] **Step 5:** Commit scoped harness/fixture updates in their owning repositories; return fixture digest, test result and producer pins. Keep selected-artifact capture pending until DN-P-C.

### Verification

From VIVY: `go test ./internal/app ./internal/rpc -count=1`. From DIVA: `python3 -m json.tool docs/plans/diva-next/fixtures/closure-chat-obs.json` after creation. Expected: real source-host frames match inspected structs, inclusive cutoff is demonstrated, failures remain failures, and every fixture states its capture provenance. No cloud keys are required for scripted contracts.

### Acceptance and handoff

Accept only captured, redacted producer contracts and their digests. DN-2A and OBS-06/07/08 may then consume those source contracts; their final installed acceptance still requires the new artifact. Stop on contradictory producer semantics or missing captured data; do not guess an adapter.

