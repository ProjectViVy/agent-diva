# DN-P-C — Repack and inspect the integrated closure Generation Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` task by task after implementation authorization. Use `superpowers:subagent-driven-development` only when delegation is explicitly authorized. Read the specification and this plan; check the parent index before starting.

**Goal:** Produce a reproducible sealed library/header/native candidate containing the accepted closure source changes.
**Architecture:** Use existing Recipe→Assembly→pack→inspect→stage→Tauri build. All source/UI/native pins travel together; no developer-sibling assumptions or alternate packaging pipeline.
**Tech Stack:** Go shared SDK target, Tauri/Rust, pinned frontend/native builds
**Spec:** [DN-C2](../p0-design.md), especially the sections cited below. [Contract ledger](../backend-separation-contracts.md) is the shared schema authority.
**Epic / requirements:** DN-P / R-2, R-6, R-8.
**State and immediate dependencies:** [authoritative index](../index.md#executable-story-package). The index owns readiness; this file does not duplicate status.
**Baseline:** DIVA `f5866a0`, VIVY `1db8b55`, Laputa `dc6066e`; exact pins are in the index. Reconcile changed source before execution.

## Global Constraints

- One VIVY Agent; one Garden owner per configured profile; no restored Rust business crates, HTTP domain daemon, duplicate authority or hidden writer fallback.
- Keep the five C exports and `vivy_call` / `vivy:event`; speech bytes stay outside Go JSON. No old-data import, local voice, generic resources or pet-host restoration.
- Native final acceptance is Windows x64 and belongs to the owner; engineering evidence and unavailable checks are reported separately.
- Read each owning repository's instructions and claim its lock before edits. Paths below are relative to that repository. New files are **proposed**; generated output is regenerated, never hand-edited.
- Use current [execution constraints and handoff](../index.md#execution-contract). No product test or implementation has been performed by generating this plan.

## Review Focus

- Packed manifest must identify actual source and all selected action/observer grants.
- Header/DLL/Generation tampering or target mismatch fails inspect/load.
- Clean build cannot depend on developer home/sibling checkouts.
- No model download, silent backend change or historical-home access at startup.
- Windows DLL/FFI/media acceptance cannot be inferred from Linux unit/build results.

## Task 1: Deliver the Story boundary

### Files

- **VIVY / modify existing:** `recipes/diva.vivy.yml`
- **VIVY / verify existing:** `sdk/internal/frontend_v1.go`
- **VIVY / verify existing:** `cmd/vivy-shared/exports.go`
- **DIVA / modify existing:** `agent-diva-gui/src-tauri/vivy-runtime/generation.json`
- **DIVA / modify existing:** `agent-diva-gui/src-tauri/vivy-runtime/vivy_abi.h`
- **DIVA / modify existing:** `agent-diva-gui/src-tauri/vivy-runtime/README.md`
- **DIVA / modify existing:** `agent-diva-gui/src-tauri/resources/vivy.default.yaml`
- **DIVA / modify existing:** `agent-diva-gui/src-tauri/tauri.conf.json`
- **DIVA / modify existing:** `.github/workflows/ci.yml`
- **DIVA / modify existing:** `justfile`

### Interfaces

Consumes final code commits from cognition/chat/console/voice and DN-0P clean staging inventory. OBS-08 code is supplied transitively through DN-6C's recorder dependency. Produces final Recipe/Generation, target library+generated header, source/native/input/output hashes, command/action/grant inventory, candidate installer and engineering build manifest.

All five C exports/ABI guards remain intact. Fresh profile paths are explicit; no old-data import/read/erase. Stage only the artifact files expected by existing build scripts. Library binaries stay in the existing artifact channel/gitignore policy; do not force large/native binaries into Git. New module pins invalidate old installed proof. This Story provides the candidate, not owner/product acceptance.

### Ordered steps

- [ ] **Step 1:** Freeze accepted code commits and refresh DN-0P transitive inventory/toolchain. Verify source changes since acceptance invalidate affected evidence before packing.

- [ ] **Step 2:** Run existing SDK shared pack/inspect from clean staged sources with exact DIVA recipe. Include selected cognitive adapter/actions/observer, OBS producers and no retired Manager; record all source/library/header/Generation hashes.

- [ ] **Step 3:** Stage inspected target output with `just shell-stage-runtime <artifact-dir>`, update packaged defaults only for the fresh selected recipe, and verify header/library/manifest consistency.

- [ ] **Step 4:** Run frontend/bridge/full native checks and Tauri candidate packaging. Extend existing CI/just recipes only for required native/boundary checks; preserve deterministic offline fixtures and separate optional live-key checks.

- [ ] **Step 5:** Load the candidate on available targets: initialize→session→safe turn→approval→cancel→hide/reopen→Quit; confirm exact capabilities/Generation, one loop and fresh-home ownership. Record Windows runner/device prerequisites still missing.

- [ ] **Step 6:** Commit scoped recipe/manifest/config/build evidence, upload the candidate through the existing authorized artifact mechanism when available, and return immutable candidate identifiers to OBS-09. No merge/release/owner acceptance is implied.

### Verification

VIVY: `go test ./internal/app ./internal/runtime ./internal/embedded ./cmd/vivy-shared ./sdk/internal/... -count=1`; `go run ./sdk pack --recipe recipes/diva.vivy.yml --target shared --output <task-artifact-dir>`; `go run ./sdk inspect-artifact <task-artifact-dir>`. DIVA: `just ci`, `just shell-test`, `just shell-clippy`, `just tauri-build`, both boundary gates. Run required full owning-repo CI before integration PR. Expected: clean-source sealed target build/load and consistent hashes; absent target checks remain pending.

### Acceptance and handoff

Return candidate artifact location/hash, exact source/toolchain/recipe/Generation/platform, commands/results and fresh start steps. OBS-09 separately checks consumer correctness on this candidate; DN-M-C audits scope/boundaries and DN-8C prepares the owner handoff.
