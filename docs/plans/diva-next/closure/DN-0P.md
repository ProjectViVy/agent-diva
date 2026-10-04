# DN-0P — Inventory pinned build and native dependency closure Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` task by task after implementation authorization. Use `superpowers:subagent-driven-development` only when delegation is explicitly authorized. Read the specification and this plan; check the parent index before starting.

**Goal:** Give the final pack Story reproducible source staging and a truthful native-dependency inventory.
**Architecture:** Reuse SDK shared-target pack/inspect and existing embed smoke. Stage exact source pins with an explicit build-only workspace; do not edit developer checkouts or assume sibling layouts.
**Tech Stack:** Go/cgo toolchains, SDK pack/inspect, Rust native loading
**Spec:** [DN-C2](../p0-design.md), especially the sections cited below. [Contract ledger](../backend-separation-contracts.md) is the shared schema authority.
**Epic / requirements:** DN-0 / R-2, R-6, R-8.
**State and immediate dependencies:** [authoritative index](../index.md#executable-story-package). The index owns readiness; this file does not duplicate status.
**Baseline:** DIVA `f5866a0`, VIVY `1db8b55`, Laputa `dc6066e`; exact pins are in the index. Reconcile changed source before execution.

## Global Constraints

- One VIVY Agent; one Garden owner per configured profile; no restored Rust business crates, HTTP domain daemon, duplicate authority or hidden writer fallback.
- Keep the five C exports and `vivy_call` / `vivy:event`; speech bytes stay outside Go JSON. No old-data import, local voice, generic resources or pet-host restoration.
- Native final acceptance is Windows x64 and belongs to the owner; engineering evidence and unavailable checks are reported separately.
- Read each owning repository's instructions and claim its lock before edits. Paths below are relative to that repository. New files are **proposed**; generated output is regenerated, never hand-edited.
- Use current [execution constraints and handoff](../index.md#execution-contract). No product test or implementation has been performed by generating this plan.

## Review Focus

- Transitive sibling replace directives must resolve from the staged directory.
- No startup model download or old-home discovery is admitted.
- Native dependency inventory differs from missing-model degraded behavior.
- A Linux artifact does not prove a Windows DLL will link/load.
- Generated library/header/Generation mismatch must stop packaging.

## Task 1: Deliver the Story boundary

### Files

- **VIVY / verify existing:** `go.mod`
- **VIVY / verify existing:** `sdk/internal/frontend_v1.go`
- **VIVY / verify existing:** `recipes/diva.vivy.yml`
- **Laputa / verify existing:** `garden/go.mod`
- **Laputa / verify existing:** `laputa/go.mod`
- **Laputa / verify existing:** `mentle/go.mod`
- **Laputa / verify existing:** `examples/vivy-embed-smoke/go.mod`
- **DIVA / create proposed:** `docs/plans/diva-next/fixtures/closure-build-inputs.json`

### Interfaces

Produces `closure-build-inputs.json` with exact Git SHAs of VIVY/Garden/Laputa/Mentle/INOFY, toolchain/target, replacement map, module graph, CGO/ONNX/tokenizer/TLS/system libraries and staged build command. Garden/Laputa/Mentle are directories of the same inspected repository; hash directory inputs as well as its commit.

No new compiler or container framework. Existing `go run ./sdk pack --recipe recipes/diva.vivy.yml --target shared --output <task-artifact-dir>` and `go run ./sdk inspect-artifact <task-artifact-dir>` stay the product path. An unavailable target compiler/library is a concrete gate, not evidence that a missing runtime model removes link requirements.

### Ordered steps

- [ ] **Step 1:** Resolve every transitive `replace` in fresh pinned source staging; record hashes and exact build-only go.work/replacements. Do not modify reference checkout go.mod files.

- [ ] **Step 2:** Run module and build inventory for the selected target: `go list -m -json all`, `go list -deps -json ./cmd/vivy-shared`, and the existing Garden/embed-smoke compile path. Inspect CGO sources/imports for actual linked native dependencies.

- [ ] **Step 3:** Build/inspect the baseline shared recipe in disposable staging. Record C compiler/GOOS/GOARCH/CGO flags, native loader requirements and missing inputs. This is a baseline probe; the new cognitive Generation remains DN-P-C work.

- [ ] **Step 4:** Write `closure-build-inputs.json` and link the inventory from ledger C2-5. Include a Windows/amd64 build/load command using the actual compiler; record unavailable runner as pending.

- [ ] **Step 5:** Commit the inventory and hand it to DN-P-C. New source pins invalidate the recorded pack result and require refreshed dependency/build evidence.

### Verification

From staged VIVY: `go test ./sdk/internal/... ./internal/embedded ./cmd/vivy-shared -count=1`; existing SDK pack and inspect commands above. Validate the inventory with `python3 -m json.tool` from DIVA after creation. Expected: exact source closure, sealed baseline manifest and header/library proof on the tested platform. Native failures are captured, not patched out of the manifest.

### Acceptance and handoff

Return source/toolchain/dependency records plus baseline artifact hashes. No full local-memory capability or Windows acceptance is claimed by inventory alone. DN-P-C consumes these inputs and reruns them on final commits.
