# DN-4A — Expose one owned Garden domain and selected backend Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` task by task after implementation authorization. Use `superpowers:subagent-driven-development` only when delegation is explicitly authorized. Read the specification and this plan; check the parent index before starting.

**Goal:** Provide public typed human/agent/evolution capabilities from one Garden owner, with one explicit memory backend.
**Architecture:** Extend agentapi.Open and runtimecore composition. Reuse Persona/ACTMEM/effect ledgers and the existing Mentle adapter; all reads, ingest and effects use the selected scoped backend.
**Tech Stack:** Go Garden/Laputa/Mentle libraries
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

- Human-only Mission/save/review APIs are absent from agent capabilities.
- Scope/profile/destination mismatch fails before any authority read/write.
- Unavailable selected writer never activates another backend for recall or ingest.
- Close invalidates all derived handles exactly once.
- Stale reviews/CAS/cursors and canonical/index states retain owning domain semantics.

## Task 1: Deliver the Story boundary

### Files

- **Laputa / modify existing:** `garden/agentapi/open.go`
- **Laputa / modify existing:** `garden/agentapi/contract.go`
- **Laputa / modify existing:** `garden/agentapi/bound_reads.go`
- **Laputa / modify existing:** `garden/agentapi/actmem.go`
- **Laputa / modify existing:** `garden/agentapi/session_ingest.go`
- **Laputa / modify existing:** `garden/agentapi/index_health.go`
- **Laputa / modify existing:** `garden/internal/runtimecore/runtime.go`
- **Laputa / modify existing:** `garden/internal/recall/fast.go`
- **Laputa / modify existing:** `garden/internal/ingest/service.go`
- **Laputa / create proposed:** `garden/agentapi/embedded_domain.go`
- **Laputa / create proposed:** `garden/agentapi/embedded_domain_test.go`
- **Laputa / verify existing:** `garden/memory/contract.go`

- **Laputa / modify existing:** `garden/evolution/domain.go`
- **Laputa / verify existing:** `garden/backends/mentle/adapter.go`

### Interfaces

Consumes C2-2/3 and existing `memory.Backend`, `evolution.Domain`, `agentapi.BoundClient`. Produces the typed public extensions in ledger C2-6. `Config.BackendID`, `DestinationID` and scope-bound `Backends` explicitly select the existing Mentle writer; no BML fallback. `BindHumanSession` is restricted to a trusted user-owned Client, `BindAgentSession` reduces capability, and `BindEvolution` reuses the same owner; VIVY imports no Garden internal packages.

Human APIs stamp actor/source from trusted composition, not parameters. Scope/destination must match configured ownership; capability Close cannot close the shared owner. Page defaults 20/max 100, closed scope-bound cursors, input 64 KiB/output 256 KiB with tighter domain limits retained.

### Ordered steps

- [ ] **Step 1:** Add `TestEmbeddedDomainSameOwner`, `TestHumanCapabilityCannotBeMintedByAgent`, `TestSelectedBackendNoFallback` and `TestDerivedHandlesAfterClose` in the proposed test file. Assert one owner, agent Mission save denied, exact scope rejection, and original dedupe receipt after replay.

- [ ] **Step 2:** Run `go test ./agentapi -run "EmbeddedDomain|HumanCapability|SelectedBackend|DerivedHandles" -count=1` from Garden. Expected red: public capability/selection extensions are missing.

- [ ] **Step 3:** Implement C2-6 methods in `embedded_domain.go`, keeping domain DTO aliases and one owner. Update every bound operation in open/bound_reads/actmem/session_ingest/index_health to use its private per-handle principal and admitted scope rather than the parent Client principal. Wrap `garden/evolution.NewDomain` inside the existing owner; add bounded results reads to its existing effects/notes JSONL ledger. Reuse domain CAS/review/ACTMEM parser and effect receipts; never export runtimecore handles.

- [ ] **Step 4:** Route FastRecall/Search/Expand/Capture/effects through one selected scope-bound backend. Retain FrozenCore-only/spooled/index-degraded behavior with explicit health; do not call those modes full memory success.

- [ ] **Step 5:** Add strict page/budget tests: hidden-scope totals absent, stale cursor rejected, expected revision preserved, canonical accepted versus completed versus index pending visible.

- [ ] **Step 6:** Run focused owning library/conformance tests, commit scoped public facade changes, and refresh exact public signatures/fixtures in C2-2/6 before releasing DN-LC.

### Verification

From Garden: `go test ./agentapi ./internal/runtimecore ./internal/recall ./internal/ingest ./memory/... ./evolution -count=1`. From Laputa module: `go test ./persona ./actmem ./evolution/... -count=1`. Expected: same-owner, capability, scoped backend and domain regression tests pass without VIVY importing `garden/internal`. Compile/native-model environment limits remain explicit; mocks do not prove a functioning installed writer.

### Acceptance and handoff

Hand off public source commit, typed facade signatures, conformance fixture digest and known degraded/native requirements. Stop on a change to domain ownership, ACTMEM storage, Mission authority or selected backend policy; those require an architecture revision, not a local workaround.
