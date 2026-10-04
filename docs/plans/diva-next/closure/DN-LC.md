> **Historical under DN-W3 (2026-10-04).** This file preserves prior scope,
> contracts and evidence. Its Tauri/C ABI/Rust host execution steps and
> scheduling/readiness statements are inactive. Use the [current index](../index.md)
> and its W0–W7 plans. Domain behavior and explicitly deferred scope remain
> reference material; prior package evidence is not Wails acceptance.

# DN-LC — Generate and compose the selected cognitive factory Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` task by task after implementation authorization. Use `superpowers:subagent-driven-development` only when delegation is explicitly authorized. Read the specification and this plan; check the parent index before starting.

**Goal:** Compile one DIVA cognitive module and bind its factories, providers and observers through the sealed Assembly.
**Architecture:** Follow the existing masks binding pattern with a narrow core contract. Arm the exact generated adapters before observer recovery; selected-but-unarmed initialization fails.
**Tech Stack:** Go VIVY Source Catalog/Assembly, Garden public facade
**Spec:** [DN-C2](../p0-design.md), especially the sections cited below. [Contract ledger](../backend-separation-contracts.md) is the shared schema authority.
**Epic / requirements:** DN-L / R-2, R-4, R-8.
**State and immediate dependencies:** [authoritative index](../index.md#executable-story-package). The index owns readiness; this file does not duplicate status.
**Baseline:** DIVA `f5866a0`, VIVY `1db8b55`, Laputa `dc6066e`; exact pins are in the index. Reconcile changed source before execution.

## Global Constraints

- One VIVY Agent; one Garden owner per configured profile; no restored Rust business crates, HTTP domain daemon, duplicate authority or hidden writer fallback.
- Keep the five C exports and `vivy_call` / `vivy:event`; speech bytes stay outside Go JSON. No old-data import, local voice, generic resources or pet-host restoration.
- Native final acceptance is Windows x64 and belongs to the owner; engineering evidence and unavailable checks are reported separately.
- Read each owning repository's instructions and claim its lock before edits. Paths below are relative to that repository. New files are **proposed**; generated output is regenerated, never hand-edited.
- Use current [execution constraints and handoff](../index.md#execution-contract). No product test or implementation has been performed by generating this plan.

## Review Focus

- Source hash/grants/provider inventory must exactly match the selected manifest.
- Observer recovery can occur before Service; durable capture may precede wake attachment.
- Wrong factory type or missing selected port fails initialization.
- Partial construction closes resources in reverse ownership order.
- Unselected recipes retain existing behavior and import no optional Garden runtime through core packages.

## Task 1: Deliver the Story boundary

### Files

- **VIVY / modify existing:** `sdk/internal/assembly/source.go`
- **VIVY / modify existing:** `sdk/internal/assembly/runtime_generate.go`
- **VIVY / modify existing:** `internal/modules/optional/catalog.go`
- **VIVY / modify existing:** `internal/app/app.go`
- **VIVY / modify existing:** `internal/app/assembly_observers.go`
- **VIVY / modify existing:** `recipes/diva.vivy.yml`
- **VIVY / create proposed:** `internal/cognitivecontract/ports.go`
- **VIVY / create proposed:** `internal/modules/diva-cognitive/module.go`
- **VIVY / create proposed:** `internal/modules/diva-cognitive/factory.go`
- **VIVY / create proposed:** `internal/modules/diva-cognitive/factory_test.go`
- **VIVY / create proposed:** `internal/app/assembly_cognitive.go`
- **VIVY / create proposed:** `internal/app/assembly_cognitive_test.go`

### Interfaces

Consumes DN-4A's public owner/domain APIs. Produces module ID `vivy/diva-cognitive`, ledger C2-3's 20 declared human actions, exact cognitive capture observer policies, optional recall source and narrow factory/bundle contracts (C2-6). `CognitiveFactoryValue() any` follows the generated mask-service seam; App strictly asserts the typed factory. No new dynamic registry.

Factory builds one bundle with shared single-use binding cells for the same generated action/observer objects. A selected composition cannot report Ready until DN-4B/C runtime callbacks are armed. This enabling Story tests staged construction with typed test ports; it does not pretend the complete production module is usable yet.

### Ordered steps

- [ ] **Step 1:** Add compiler/App tests for selected, omitted, type-mismatched and unarmed bindings; assert one observer/provider instance and no unsealed late subscription. Run tests red before adding binding support.

- [ ] **Step 2:** Define C2-6 factory/bundle interfaces in `cognitivecontract/ports.go`; implement module descriptor/factory and generated metadata using masks as the local pattern. Declare all future action IDs now; defer handler behavior to DN-4C, never return success placeholders.

- [ ] **Step 3:** Compose in order: validate fresh config → open one owner → arm domain capture/source → construct ObserverHost/recover → create Service → attach wake/control callbacks → validate readiness. A pre-Service receipt needs only its durable high watermark, not an unbounded notification queue.

- [ ] **Step 4:** Update the optional Source Catalog and DIVA recipe with exact action grants/event field policies. Regenerate Assembly through the existing generator and refresh source digests under VIVY rules; do not hand-edit generated Go.

- [ ] **Step 5:** Test cleanup/error ordering and omitted recipes; keep primary admission closed for incomplete selected runtime ports until DN-4B/C supplies them.

- [ ] **Step 6:** Commit scoped compiler/module/composition work and hand off the generated contract, source hashes and inventory proof.

### Verification

From VIVY: `go test ./sdk/internal/assembly ./internal/app ./internal/observerhost ./internal/modules/diva-cognitive ./internal/cognitivecontract -count=1` after proposed packages exist. Use the repository's existing generation/source-hash command. Expected: selected inventories match exactly, omitted composition is unchanged, invalid/unarmed binding fails, and recovery owns one adapter set.

### Acceptance and handoff

Return sealed input/output inventories, generated binding test results and typed bundle signatures. DN-4B consumes this assembly seam; no product-ready cognitive module or new accepted DLL is claimed at this enabling boundary.

