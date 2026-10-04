> **Historical under DN-W3 (2026-10-04).** This file preserves prior scope,
> contracts and evidence. Its Tauri/C ABI/Rust host execution steps and
> scheduling/readiness statements are inactive. Use the [current index](../index.md)
> and its W0–W7 plans. Domain behavior and explicitly deferred scope remain
> reference material; prior package evidence is not Wails acceptance.

# DN-0S — Probe native speech seams and provider mappings Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` task by task after implementation authorization. Use `superpowers:subagent-driven-development` only when delegation is explicitly authorized. Read the specification and this plan; check the parent index before starting.

**Goal:** Resolve Raw/Response, WebView audio, credential and provider uncertainties into a bounded compatibility record.
**Architecture:** Use a disposable native probe and fixture endpoints, not a second production transport. Keep DN-C2 WAV/MP3 and key-store decisions; request a design revision only if a required seam cannot be supported.
**Tech Stack:** Bundled Tauri v2, Web Audio, Rust keyring/reqwest, official provider references
**Spec:** [DN-C2](../p0-design.md), especially the sections cited below. [Contract ledger](../backend-separation-contracts.md) is the shared schema authority.
**Epic / requirements:** DN-0 / R-5, R-6, R-8.
**State and immediate dependencies:** [authoritative index](../index.md#executable-story-package). The index owns readiness; this file does not duplicate status.
**Baseline:** DIVA `f5866a0`, VIVY `1db8b55`, Laputa `dc6066e`; exact pins are in the index. Reconcile changed source before execution.

## Global Constraints

- One VIVY Agent; one Garden owner per configured profile; no restored Rust business crates, HTTP domain daemon, duplicate authority or hidden writer fallback.
- Keep the five C exports and `vivy_call` / `vivy:event`; speech bytes stay outside Go JSON. No old-data import, local voice, generic resources or pet-host restoration.
- Native final acceptance is Windows x64 and belongs to the owner; engineering evidence and unavailable checks are reported separately.
- Read each owning repository's instructions and claim its lock before edits. Paths below are relative to that repository. New files are **proposed**; generated output is regenerated, never hand-edited.
- Use current [execution constraints and handoff](../index.md#execution-contract). No product test or implementation has been performed by generating this plan.

## Review Focus

- Unsupported recorder decode is a visible device failure, not an empty transcript.
- Raw metadata is at most 2 KiB; audio stays outside the 4 MiB Go ABI.
- Unix key store may be locked or absent; plaintext fallback is forbidden.
- MiniMax HTTP 200 can still contain a business error or malformed audio hex.
- Configured regional host/model/reference compatibility cannot be silently repaired.

## Task 1: Deliver the Story boundary

### Files

- **DIVA / verify existing:** `agent-diva-gui/src-tauri/Cargo.toml`
- **DIVA / verify existing:** `agent-diva-gui/src-tauri/Cargo.lock`
- **DIVA / verify existing:** `agent-diva-gui/src/platform/desktop-host.ts`
- **DIVA / verify existing:** `agent-diva-gui/src/features/diva-pet/voice/services/tts/providers/minimax-provider.ts`
- **DIVA / create proposed:** `docs/plans/diva-next/fixtures/closure-speech.json`
- **DIVA / modify existing:** `docs/plans/diva-next/backend-separation-contracts.md`

### Interfaces

Consumes C2-4 and DN-C2 §§7,9,12. Produces source-pinned `closure-speech.json`: native versions/features/MSRV, Raw header acceptance, binary response behavior, recording codec→mono 16 kHz PCM16 WAV→MP3 playback results, OS-store results, reqwest TLS choice, regional endpoints/model/reference fixtures and pending checks. Test code stays in an isolated disposable probe until an owning implementation Story adopts it.

Refresh MiniMax's official nonstreaming HTTP documentation and preserve configured region/model IDs. Record request shape, `base_resp.status_code`, `data.audio` hex, MP3 response and error limits. Offline fixtures prove protocol handling; authenticated cloud quality is reserved for owner acceptance. No key is requested merely to make a planning probe green.

### Ordered steps

- [ ] **Step 1:** Read the installed Tauri/Rust and frontend lock versions. Build a disposable probe on those versions that echoes bounded Raw bytes and returns MP3 with `ipc::Response`; record byte equality and response type.

- [ ] **Step 2:** Exercise record/Stop/decode/resample/encode and MP3 playback on an available native host. Inspect WAV channels/rate/sample width/duration and decoded memory. Record Windows WebView2 or Linux WebKit as pending if unavailable.

- [ ] **Step 3:** Probe keyring 4.2.0 v1 with platform store features and the shell toolchain; test set/presence/read/delete/reopen plus locked/unavailable mapping using a non-secret sentinel. Fix dependency versions/features in the ledger after proof.

- [ ] **Step 4:** Refresh official SiliconFlow and MiniMax request examples into redacted offline fixtures, including system/reusable/inline references, invalid field/model pairs and error envelopes. Preserve STT recognition semantics with no translation or speculative language forwarding.

- [ ] **Step 5:** Choose the smallest supported reqwest TLS feature set after compile/link evidence. Record native toolchain and OS requirements; keep unmet target checks pending rather than selecting a different architecture silently.

- [ ] **Step 6:** Commit the compatibility record/fixtures. Gate DN-6A/B/C on the exact accepted seams; if a probe fails, revise only affected contracts and preserve independent chat/console work.

### Verification

Inspect current lockfiles; run `cargo check --manifest-path <disposable-probe>/Cargo.toml` and its targeted `cargo test` suite on the recorded target. The probe directory is task-local and never staged into product docs. Native record/play/key-store checks require actual OS access. Expected: bounded IPC byte equality, valid WAV/MP3, credential failure without leakage/fallback, and fixture-proven protocol mappings; missing access is pending.

### Acceptance and handoff

Return official reference URLs/check dates, sanitized fixtures, exact versions/features and observed OS results. Portable fixture preparation may complete without cloud credentials. This Story is accepted for downstream implementation only when required IPC/dependency seams have proof; absent Windows evidence remains a later DN-8C gate.

