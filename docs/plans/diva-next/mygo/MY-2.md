# MY-2: Verify native speech, VRM and Windows installation parity — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: after implementation authorization,
> use **superpowers:executing-plans** task by task. Delegation is not selected.
> Steps use checkbox syntax; this planning publication does not start code work.

**Goal:** Verify native speech, VRM and Windows installation parity.
**Architecture:** Reuse accepted Go speech and frontend behavior; implement only the MyGo native identity/media adapter and package the inspected executable.
**Tech Stack:** Pinned MyGo v0.2.4, isolated Go 1.27.1, public VIVY SDK,
retained Vue/TypeScript and Windows WebView2.
**Spec:** [MY-D1](design.md), revision MY-D1 / 2026-10-04.
**Requirements:** MY-R3, MY-R4.
**Status/dependencies:** [Authoritative index](index.md#stories-and-readiness).
**Baseline:** DIVA 5444795a2d9db31e158c2cf009d64697e6289e50;
VIVY fc559e6b03ce4e65c0099b9745855dccc4fb067e.

## Global Constraints

Planning only is authorized. Preserve Wails delivery, one Runtime/Journal,
sealed Generation, public SDK-only embedding, native sender authority, existing
DTOs and permissions. Root context differs from page context.
Close budget is five seconds; Next batch maximum is 500 and Host queue capacity
is 10,000. Speech limits/namespace are exactly MY-D1 / W3-3.
Proposed source paths do not exist at the inspected baseline.
Integrate accepted predecessor outputs and reconcile source before execution.
Source reads/mock tests do not replace native, provider or owner acceptance.


## Review Focus

Forged native sender metadata must fail; navigation/hide/cancel must prevent stale
audio; provider error JSON must never be played; keyring failure must stay explicit;
installers must not rebuild or replace the delivery product.

## Task 1: Adapt the accepted speech service to MyGo native media

**Files, proposed:** Create internal/desktop/mygo/media_http.go and media_http_test.go;
extend bindings.go and agent-diva-gui/src/platform/mygo-host.ts.
**Read-only reused:** W4 internal/speech/*, existing src/api/speech.ts and
src/features/voice/useVoiceController.ts.
**Consumes:** Accepted W4 concrete Go service signatures; MY-0 native metadata notes;
W3-3 raw route, capability, DTO and cancellation semantics.
**Produces:** Same facade behavior and capability-bound raw media for MyGo.

- [ ] Record exact accepted W4 constructors and request/context methods in the task's implementation evidence before coding; this task stays blocked until that native-independent API exists.
- [ ] Add failing native/media tests for foreign/revoked capability, browser-forged window ID, metadata beyond 2 KiB, per-file/count/total quota, corrupt WAV, provider JSON error and late success after generation change. Reuse W4 vectors/constants.
- [ ] Issue capability only via trusted primary bound context; resolve protocol sender from MY-0-proven framework metadata, not CallerWindow on an arbitrary HTTP context. Authenticate before body allocation and validate again before persistence/playback.
- [ ] Wire all preserved config/credential/context/cancel/asset calls and four raw routes to the accepted speech service; preserve no-store, keyring namespace, CAS, opaque IDs and leases. Add no generic provider HTTP/key export to the browser.
- [ ] Run staged Go race tests, retained speech/controller tests and Windows cross-window/navigation/keyring probes. Expected: invalid calls return typed errors and no stale/foreign audio persists or plays.

## Task 2: Check retained product paths in the real native host

**Files:** Existing src/features/voice/useVoiceController.test.ts and
src/state/vivy-session.test.ts (extend only if a candidate regression needs coverage);
observations.md plus implementation iteration evidence.
**Consumes:** MY-1 candidate, accepted W3 frontend behavior and W4 speech adapter.
**Produces:** Native scenario matrix with pass/fail/pending and concrete diagnostics.

- [ ] On fresh disposable profiles verify streaming chat, approval, cancel,
console/readback, gap/reload recovery and VRM rendering using the same retained
assets/scenarios as the baseline. Verify microphone permission grant/deny,
capture, playback and lip sync; mocks remain labeled local-provider.
- [ ] During active model/speech work exercise hide/reopen, reload and explicit
Quit; verify one runtime, no replayed/stale speech, stopped browser audio and
truthful close diagnostics. Exercise secondary process/profile admission.
- [ ] When operator-held credentials are available run SiliconFlow STT and
SiliconFlow/MiniMax TTS through the native adapter; record redacted outcomes.
Unavailable credentials/Windows devices leave those rows pending.
- [ ] Return scenario evidence, actual failures and affected gates; do not fix
unrelated business behavior or report native acceptance from unit tests.

## Task 3: Package and install the exact sealed executable

**Files:** Accepted W6 Windows packaging templates/resource files (exact paths
must be recorded from its accepted output); candidate build report and ledger.
**Consumes:** MY-1 inspected executable; accepted W6 packaging-only entry and
experimental application identity from MY-0.
**Produces:** Isolated installed Windows candidate plus executable/hash provenance.

- [ ] Include required icons/resources before the sealed compile; package its
exact executable with accepted packaging-only tooling. Do not run mygo build
or a stock desktop task that recompiles the app.
- [ ] Verify installer input/output executable hashes and Generation identity,
install under the experimental identity/profile, and confirm the Wails product
installation/credentials are untouched.
- [ ] Re-run retained native smoke from the installed candidate, including
WebView2/runtime prerequisites and uninstall/profile behavior. Persist the
artifact and environment IDs in evidence, not this branch's tracked source lock.
- [ ] Run scoped/required gates for actual source changes, document pending
native/cloud/owner rows, commit focused changes and update the index.

## Acceptance and blockers

This Story requires accepted W4 service signatures and W6 packaging-only output;
neither can be assumed from their plans. Failure to preserve native trust/sealing
blocks the candidate. Missing installed/provider evidence prevents promotion.
Return installed artifact provenance and the complete scenario matrix to MY-3.
