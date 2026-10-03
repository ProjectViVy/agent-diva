# DN-6A — Native speech preferences, credentials and reference assets Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` task by task after implementation authorization. Use `superpowers:subagent-driven-development` only when delegation is explicitly authorized. Read the specification and this plan; check the parent index before starting.

**Goal:** Persist speech settings securely and manage bounded new reference audio without legacy pet commands or file access.
**Architecture:** Add a narrow native speech module using probe-selected OS keyring dependencies and atomic native preferences/assets. No Agent config/database ownership moves into Rust.
**Tech Stack:** Tauri/Rust, OS keyring, atomic JSON/config, bounded files
**Spec:** [DN-C2](../p0-design.md), especially the sections cited below. [Contract ledger](../backend-separation-contracts.md) is the shared schema authority.
**Epic / requirements:** DN-6 / R-2, R-5, R-8.
**State and immediate dependencies:** [authoritative index](../index.md#executable-story-package). The index owns readiness; this file does not duplicate status.
**Baseline:** DIVA `f5866a0`, VIVY `1db8b55`, Laputa `dc6066e`; exact pins are in the index. Reconcile changed source before execution.

## Global Constraints

- One VIVY Agent; one Garden owner per configured profile; no restored Rust business crates, HTTP domain daemon, duplicate authority or hidden writer fallback.
- Keep the five C exports and `vivy_call` / `vivy:event`; speech bytes stay outside Go JSON. No old-data import, local voice, generic resources or pet-host restoration.
- Native final acceptance is Windows x64 and belongs to the owner; engineering evidence and unavailable checks are reported separately.
- Read each owning repository's instructions and claim its lock before edits. Paths below are relative to that repository. New files are **proposed**; generated output is regenerated, never hand-edited.
- Use current [execution constraints and handoff](../index.md#execution-contract). No product test or implementation has been performed by generating this plan.

## Review Focus

- Key-store unavailable/locked has no sample/plaintext fallback or success echo.
- Preference conflict or failure after key set is visible partial success.
- Traversal/symlink/foreign IDs cannot access arbitrary files.
- Deleting an in-use reference cannot race synthesis into missing bytes.
- Unsupported model/reference configuration is rejected without dropping it.

## Task 1: Deliver the Story boundary

### Files

- **DIVA / modify existing:** `agent-diva-gui/src-tauri/Cargo.toml`
- **DIVA / modify existing:** `agent-diva-gui/src-tauri/Cargo.lock`
- **DIVA / modify existing:** `agent-diva-gui/src-tauri/src/lib.rs`
- **DIVA / modify existing:** `scripts/ci/check_vivy_backend_boundary.py`
- **DIVA / create proposed:** `agent-diva-gui/src-tauri/src/speech/mod.rs`
- **DIVA / create proposed:** `agent-diva-gui/src-tauri/src/speech/config.rs`
- **DIVA / create proposed:** `agent-diva-gui/src-tauri/src/speech/credentials.rs`
- **DIVA / create proposed:** `agent-diva-gui/src-tauri/src/speech/assets.rs`
- **DIVA / create proposed:** `agent-diva-gui/src-tauri/src/speech/commands.rs`

### Interfaces

Consumes DN-0S IPC/keyring/native dependency fixtures. Produces C2-4 config/credential/reference commands and `diva.speech/v1` DTOs; register exact main-window commands using snake_case args. Preference CAS, OS key operations and asset manifest are separate transactions.

Key namespace fixed at startup from app/fresh profile/closed provider enum; presence/availability only on read. Reference import accepts Raw bytes with ≤2 KiB `x-diva-asset-meta`, no path/URL. Validate WAV/MP3 signature/MIME, ≤10 MiB/file, ≤20 files/100 MiB; native ID/digest/owned safe path. `AssetLease` holds a reader; delete marks pending until release. Configured deleted/missing reference is explicit error.

### Ordered steps

- [ ] **Step 1:** Add native unit cases `credential_unavailable_no_fallback`, `config_revision_conflict`, `asset_traversal_rejected`, `asset_bounds`, `leased_delete_pending`; assert secrets absent from config_get/errors and actual lease cleanup.

- [ ] **Step 2:** Run shell speech tests red after adding empty test module; implement strict DTO validation/atomic revision CAS and trusted main-window caller checks. Pin only DN-0S-proven dependency/features in Cargo.lock.

- [ ] **Step 3:** Implement fixed OS credential slots, presence-only readback and separate set/delete commands. No arbitrary service/profile string and no env/plaintext/sample store is used by installed product.

- [ ] **Step 4:** Implement bounded owned asset import/list/read/delete and lease lifetime; atomic manifest replace, safe native-generated names, validated bytes and stale config reference errors.

- [ ] **Step 5:** Register only this Story's exact config/credential/asset commands; update native boundary gate in the same commit. Do not weaken legacy business dependencies or dormant pet exemptions.

- [ ] **Step 6:** Run focused/full shell checks plus available OS-store reopen/asset smoke; commit `feat(speech): own secure preferences and reference assets`.

### Verification

`cargo test --manifest-path agent-diva-gui/src-tauri/Cargo.toml speech -- --nocapture`; `just shell-clippy`, `just shell-bridge-test`; `python3 scripts/ci/check_vivy_backend_boundary.py`. Full shell checks need native WebKit/OS inputs. Expected: CAS/asset/credential contract tests pass, real installed-store failures remain explicit and no secrets enter readback/logs.

### Acceptance and handoff

Return config/command schema version, dependency/features pin, available OS-store evidence and lease/asset fixtures. DN-6B owns admitted request references; DN-6C adds settings UI. Native checks not runnable here remain named DN-8C obligations.
