> **Historical under DN-W3 (2026-10-04).** This file preserves prior scope,
> contracts and evidence. Its Tauri/C ABI/Rust host execution steps and
> scheduling/readiness statements are inactive. Use the [current index](../index.md)
> and its W0–W7 plans. Domain behavior and explicitly deferred scope remain
> reference material; prior package evidence is not Wails acceptance.

# DN-4C — Implement governed human cognitive actions Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` task by task after implementation authorization. Use `superpowers:subagent-driven-development` only when delegation is explicitly authorized. Read the specification and this plan; check the parent index before starting.

**Goal:** Expose the complete closed cognitive human control inventory through authenticated embedded actions.
**Architecture:** Use ActionHost and trusted peer origin, then typed same-owner capabilities. Keep Persona review, runtime policy and Agent execution approvals distinct.
**Tech Stack:** Go ActionHost/RPC/module actions, strict JSON schemas
**Spec:** [DN-C2](../p0-design.md), especially the sections cited below. [Contract ledger](../backend-separation-contracts.md) is the shared schema authority.
**Epic / requirements:** DN-4 / R-2, R-4.
**State and immediate dependencies:** [authoritative index](../index.md#executable-story-package). The index owns readiness; this file does not duplicate status.
**Baseline:** DIVA `f5866a0`, VIVY `1db8b55`, Laputa `dc6066e`; exact pins are in the index. Reconcile changed source before execution.

## Global Constraints

- One VIVY Agent; one Garden owner per configured profile; no restored Rust business crates, HTTP domain daemon, duplicate authority or hidden writer fallback.
- Keep the five C exports and `vivy_call` / `vivy:event`; speech bytes stay outside Go JSON. No old-data import, local voice, generic resources or pet-host restoration.
- Native final acceptance is Windows x64 and belongs to the owner; engineering evidence and unavailable checks are reported separately.
- Read each owning repository's instructions and claim its lock before edits. Paths below are relative to that repository. New files are **proposed**; generated output is regenerated, never hand-edited.
- Use current [execution constraints and handoff](../index.md#execution-contract). No product test or implementation has been performed by generating this plan.

## Review Focus

- Face text/session/actor fields in untrusted JSON do not create human authority.
- Malformed/duplicate/unknown/trailing fields and unsafe numeric revisions fail before mutation.
- Mission writes/reviews/ACTMEM owner save retain separate domain restrictions.
- Timed-out writes remain unknown and are not resubmitted automatically.
- Disabled/paused/recovery-required and submitted versus applied remain distinct results.

## Task 1: Deliver the Story boundary

### Files

- **VIVY / modify existing:** `internal/app/facehost.go`
- **VIVY / modify existing:** `internal/embedded/host.go`
- **VIVY / modify existing:** `internal/actionhost/host.go`
- **VIVY / modify existing:** `internal/rpc/control.go`
- **VIVY / create proposed:** `internal/modules/diva-cognitive/actions.go`
- **VIVY / create proposed:** `internal/modules/diva-cognitive/actions_test.go`
- **VIVY / create proposed:** `internal/app/cognitive_origin_test.go`
- **DIVA / create proposed:** `docs/plans/diva-next/fixtures/closure-cognitive-actions.json`
- **DIVA / modify existing:** `docs/plans/diva-next/backend-separation-contracts.md`

### Interfaces

Consumes DN-4B lifecycle/control/authority gate and DN-4A typed capabilities. Produces all 20 C2-3 actions in `module.action.invoke {module_id,action_id,input}` with `CognitiveOutcome<T>` and CapabilityStatus. No extra RPC envelope/origin payload field.

Embedded host stamps a server-owned human invocation origin at DialControl; missing origin/other transport/model tools cannot invoke these actions. Verify input session against trusted peer binding and session registry, then derive capability. C2-6 runtime policy CAS/status extensions preserve numeric UI revision and string RunBinding policy pin. Page limits 20/100, input 64 KiB/output 256 KiB, safe integers and existing domain error codes are enforced.

### Ordered steps

- [ ] **Step 1:** Add origin/schema tests covering forged face/origin/actor/session, model action dispatch, duplicate keys/trailing JSON, revision conflicts and output secret sentinels. Assert denied actions produce no domain effects.

- [ ] **Step 2:** Run ActionHost/App/module tests red; extend trusted identity/origin plumbing and exact authorization predicates. Retain ordinary action governance; never set blanket full-auto or bypass an unsupported approval continuation.

- [ ] **Step 3:** Implement each C2-3 handler in `actions.go` with typed C2-6 facades/runtime controls; bind generated cells only after required callbacks exist. Missing selected handler/port fails initialization, not a successful empty response.

- [ ] **Step 4:** Implement adapter policy revision CAS and status/read/result paging. Reconcile ambiguous writes via scoped receipts/history; no generic force-clear endpoint. Never expose a human capability to the compiled agent tools.

- [ ] **Step 5:** Capture action success/conflict/forbidden/unavailable/unknown fixtures via trusted embedded peer. Assert all 20 IDs match the generated manifest, owning DTO aliases and schemas.

- [ ] **Step 6:** Commit scoped ActionHost/RPC/module updates, update ledger mappings from proposed to captured only where proven, and return action/source/fixture hashes.

### Verification

From VIVY: `go test ./internal/actionhost ./internal/rpc ./internal/app ./internal/modules/diva-cognitive -count=1`. Validate the new JSON fixture from DIVA. Expected: exact selected action inventory, server-bound origin/session enforcement, no unauthorized writes, domain-preserving outcomes and truthful unknown states.

### Acceptance and handoff

DN-4D consumes captured action DTOs and outcomes. Hand off trusted-origin negative evidence, all 20 action mappings and control CAS tests. Stop if the actual transport cannot issue/verify human origin; changing JSON payload claims is not a permitted substitute.

