> **Historical under DN-W3 (2026-10-04).** This file preserves prior scope,
> contracts and evidence. Its Tauri/C ABI/Rust host execution steps and
> scheduling/readiness statements are inactive. Use the [current index](../index.md)
> and its W0–W7 plans. Domain behavior and explicitly deferred scope remain
> reference material; prior package evidence is not Wails acceptance.

# DN-8C — Prepare clean native installation and owner acceptance Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` task by task after implementation authorization. Use `superpowers:subagent-driven-development` only when delegation is explicitly authorized. Read the specification and this plan; check the parent index before starting.

**Goal:** Deliver an identifiable native candidate and reproducible Windows x64 acceptance steps for the owner.
**Architecture:** Finish engineering build/load/clean-install checks on available targets, then hand off the full integrated product. Preserve owner final acceptance as a distinct outcome.
**Tech Stack:** Windows x64 DLL/FFI/WebView2/installer; Linux recorded comparison
**Spec:** [DN-C2](../p0-design.md), especially the sections cited below. [Contract ledger](../backend-separation-contracts.md) is the shared schema authority.
**Epic / requirements:** DN-8 / R-1, R-3, R-4, R-5, R-6, R-7, R-8, R-9.
**State and immediate dependencies:** [authoritative index](../index.md#executable-story-package). The index owns readiness; this file does not duplicate status.
**Baseline:** DIVA `f5866a0`, VIVY `1db8b55`, Laputa `dc6066e`; exact pins are in the index. Reconcile changed source before execution.

## Global Constraints

- One VIVY Agent; one Garden owner per configured profile; no restored Rust business crates, HTTP domain daemon, duplicate authority or hidden writer fallback.
- Keep the five C exports and `vivy_call` / `vivy:event`; speech bytes stay outside Go JSON. No old-data import, local voice, generic resources or pet-host restoration.
- Native final acceptance is Windows x64 and belongs to the owner; engineering evidence and unavailable checks are reported separately.
- Read each owning repository's instructions and claim its lock before edits. Paths below are relative to that repository. New files are **proposed**; generated output is regenerated, never hand-edited.
- Use current [execution constraints and handoff](../index.md#execution-contract). No product test or implementation has been performed by generating this plan.

## Review Focus

- Installed Windows DLL/header/calling convention must match the candidate hashes.
- WebView codec/mic permission/key store may differ from developer fixtures.
- Hide/reopen retains one host and approvals but does not auto-replay audio.
- Quit/drain deadlines and restart unknown-effect blocks remain visible.
- No user acceptance, merge or release is inferred from a green build or agent report.

## Task 1: Deliver the Story boundary

### Files

- **DIVA / modify existing:** `agent-diva-gui/src-tauri/tauri.conf.json`
- **DIVA / modify existing:** `.github/workflows/ci.yml`
- **DIVA / modify existing:** `docs/plans/diva-next/index.md`
- **DIVA / create proposed:** `docs/plans/diva-next/closure/owner-acceptance.md`
- **DIVA / verify existing:** `agent-diva-gui/src-tauri/tests/bridge_lifecycle.rs`

### Interfaces

Consumes DN-M-C scoped evidence/pending rows and immutable candidate transitively from DN-P-C. Produces engineering handoff: installer/library/header/Generation/source/toolchain hashes, fresh install/start/uninstall paths, native dependencies, local key/model/region setup, evidence matrix and owner checklist.

Owner checklist covers real model/tools/approval/cancel/reopen/Goal/image regeneration; Persona/current-frozen/Mission/ACTMEM/scoped memory/review/strategy cancel/crash recovery; logs/trajectory/usage; mic→editable STT→normal send→both TTS→playback/interrupt/hide/Quit. Use owner sample phrases to assess recognition/listening; no invented accuracy/latency percentage. No migration/realtime call/local voice added.

### Ordered steps

- [ ] **Step 1:** Verify immutable candidate/pins. Build/install in a clean Windows x64 runner using recorded native/compiler/WebView requirements; run FFI lifecycle and clean-start checks. If Windows access is absent, retain this gate Blocked while completing the handoff material.

- [ ] **Step 2:** Exercise available engineering native paths: library load/initialize, no legacy-home access, profile setup and required package resources, one host/loop, pending approval hide/reopen, active media cancellation/Quit and bounded teardown.

- [ ] **Step 3:** Write `owner-acceptance.md` in the same plan set with the concrete setup steps/checklist in DN-C2 §11. Each scenario names expected visible behavior, candidate hash and engineering result/pending limit.

- [ ] **Step 4:** Prepare a safe diagnostic export procedure using existing bounded reads/screenshots, redaction and exact repro IDs. Do not add an arbitrary filesystem export/resource subsystem.

- [ ] **Step 5:** Finish all authorized engineering fixes/checks before handing the owner the candidate. Record unavailable key/device/Windows checks as pending. User supplies their own credentials locally during acceptance.

- [ ] **Step 6:** Commit the handoff documentation/artifact manifest. Move DN-8 to owner-review only when engineering gates are met; mark product accepted only after the owner's recorded outcome. Release/merge is a separate explicitly authorized action.

### Verification

On the actual Windows x64 candidate run current `just shell-bridge-test`, `just shell-test`, `just shell-clippy`, `just tauri-build` with the recorded Windows toolchain and native environment. Run the installed checklist above; existing bridge_lifecycle tests exercise process/FFI behavior. Linux evidence remains target-scoped. Expected: engineering installation/lifecycle/codec/key-store proof and a complete owner handoff; missing access never becomes a pass.

### Acceptance and handoff

Return immutable installer location/hash and requirement-by-requirement engineering versus owner states. Completion of this planning package creates no installer and supplies no final acceptance. Do not request owner final acceptance until the concrete build and engineering evidence are ready.

