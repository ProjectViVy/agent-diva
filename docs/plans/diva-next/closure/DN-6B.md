> **Historical under DN-W3 (2026-10-04).** This file preserves prior scope,
> contracts and evidence. Its Tauri/C ABI/Rust host execution steps and
> scheduling/readiness statements are inactive. Use the [current index](../index.md)
> and its W0–W7 plans. Domain behavior and explicitly deferred scope remain
> reference material; prior package evidence is not Wails acceptance.

# DN-6B — Bounded cloud providers and native request cancellation Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` task by task after implementation authorization. Use `superpowers:subagent-driven-development` only when delegation is explicitly authorized. Read the specification and this plan; check the parent index before starting.

**Goal:** Recognize WAV and synthesize MP3 through SiliconFlow/MiniMax with bounded HTTP, request fencing and cleanup.
**Architecture:** Reuse protocol mappings in closed Rust provider adapters and Tauri async runtime. Admit by window/session/generation before HTTP; freeze config/key/reference and release every request path.
**Tech Stack:** Tauri/Rust, reqwest, Raw Request/Response, provider fixture server
**Spec:** [DN-C2](../p0-design.md), especially the sections cited below. [Contract ledger](../backend-separation-contracts.md) is the shared schema authority.
**Epic / requirements:** DN-6 / R-2, R-5, R-6, R-9.
**State and immediate dependencies:** [authoritative index](../index.md#executable-story-package). The index owns readiness; this file does not duplicate status.
**Baseline:** DIVA `f5866a0`, VIVY `1db8b55`, Laputa `dc6066e`; exact pins are in the index. Reconcile changed source before execution.

## Global Constraints

- One VIVY Agent; one Garden owner per configured profile; no restored Rust business crates, HTTP domain daemon, duplicate authority or hidden writer fallback.
- Keep the five C exports and `vivy_call` / `vivy:event`; speech bytes stay outside Go JSON. No old-data import, local voice, generic resources or pet-host restoration.
- Native final acceptance is Windows x64 and belongs to the owner; engineering evidence and unavailable checks are reported separately.
- Read each owning repository's instructions and claim its lock before edits. Paths below are relative to that repository. New files are **proposed**; generated output is regenerated, never hand-edited.
- Use current [execution constraints and handoff](../index.md#execution-contract). No product test or implementation has been performed by generating this plan.

## Review Focus

- Cancel before/after admission and after settlement must not leak or admit stale context.
- HTTP 200/business error/malformed hex/oversize streams never become successful audio.
- Wrong rate/channels/sample width/duration is rejected before cloud upload.
- New config/references cannot alter an admitted request snapshot.
- Quit rejects admission, aborts/join requests, and reports deadline failure honestly.

## Task 1: Deliver the Story boundary

### Files

- **DIVA / modify existing:** `agent-diva-gui/src-tauri/src/lib.rs`
- **DIVA / modify existing:** `agent-diva-gui/src-tauri/src/lifecycle.rs`
- **DIVA / modify existing:** `scripts/ci/check_vivy_backend_boundary.py`
- **DIVA / modify existing:** `scripts/ci/check_legacy_frontend_calls.mjs`
- **DIVA / create proposed:** `agent-diva-gui/src-tauri/src/speech/service.rs`
- **DIVA / create proposed:** `agent-diva-gui/src-tauri/src/speech/providers.rs`
- **DIVA / create proposed:** `agent-diva-gui/src-tauri/tests/speech_contract.rs`

### Interfaces

Consumes DN-6A preferences/credentials/AssetLease and DN-0S provider/IPC fixtures. Produces exact `speech_context_set`, Raw `speech_transcribe`, binary `speech_synthesize` and idempotent `speech_cancel`. `SpeechIdentity`/`SpeechFailure` are C2-4; no release-audio/cache/temp-file command.

Admission atomically checks trusted window + session/generation, reserves request_id and one STT/one TTS slot. Config snapshot/credential/reference selection belongs to that admission; mutex never spans keyring/HTTP. Context advance aborts old requests and rejects late admission. Successful TTS is bounded MP3 `ipc::Response`; STT JSON echoes identity/no_speech. Native diagnostic event is safe `speech:diagnostic` to main window only.

DN-C2 guards: 120 s mono16k PCM16 WAV/8 MiB; JSON STT64 KiB/text16 KiB; TTS4000 code points/16 KiB; MP3≤16 MiB/MiniMax hex JSON≤34 MiB; connect10 s/total120 s. Validate HTTPS origin without URL credentials/query/fragment, no cross-host redirects/retries/failover. Provider-specific tighter guards come from DN-0S.

### Ordered steps

- [ ] **Step 1:** Add `stale_context_cannot_admit`, `cancel_before_http`, `late_completion_discarded`, `oversize_body_bounded`, `minimax_business_error`, `quit_with_inflight` to proposed native fixture tests. Assert active slots/leases return to zero on all terminal paths.

- [ ] **Step 2:** Run tests red; implement the context/admission registry and owned abort/join cleanup, then closed provider clients. Reuse Tauri async runtime; do not introduce a daemon or native queue.

- [ ] **Step 3:** Implement Raw metadata/WAV validation, multipart STT, MP3 SiliconFlow TTS and strict MiniMax status/hex decode under streaming caps. Use probe-fixed raw model/region/reference mappings; no translation parameter.

- [ ] **Step 4:** Emit only phase/provider/safe code/HTTP status/elapsed/byte counts/identity; redact before emission. Integrate hide/context/Quit invalidation before resource close; no billing/refund claim from local abort.

- [ ] **Step 5:** Register exact native commands and AST allowlist together; negative fixtures reject dynamic/generic invokes, browser HTTP and restored pet_*. Keep new production speech outside the broad dormant exemption.

- [ ] **Step 6:** Run provider fixtures/shell/bridge/boundary tests and commit `feat(speech): add bounded cloud media and cancellation`. Authenticated provider quality is reserved for later owner acceptance.

### Verification

`cargo test --manifest-path agent-diva-gui/src-tauri/Cargo.toml --test speech_contract`; `just shell-test`, `just shell-clippy`, `just shell-bridge-test`; `node scripts/ci/check_legacy_frontend_calls.mjs --selftest`; `python3 scripts/ci/check_vivy_backend_boundary.py`. Expected: real HTTP fixture shapes/error caps, request/lease cleanup, closed native inventory and preserved ABI; unavailable native dependencies are explicit blockers.

### Acceptance and handoff

Return native command/event fixture version, stream-limit/cancel-race evidence and teardown contract. DN-6C may use it only after accepted IPC/provider fixtures. Do not claim cloud/model/audio quality from offline fixture success.

