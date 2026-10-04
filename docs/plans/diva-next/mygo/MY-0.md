# MY-0: Establish a pinned native feasibility and observation baseline — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: after implementation authorization,
> use **superpowers:executing-plans** task by task. Delegation is not selected.
> Steps use checkbox syntax; owner authorization is recorded in the index.

**Goal:** Establish a pinned native feasibility and observation baseline.
**Architecture:** Resolve framework-specific API/build questions cheaply; record them in the existing ledger.
**Tech Stack:** Pinned MyGo v0.2.4, isolated Go 1.27.1, public VIVY SDK,
retained Vue/TypeScript and Windows WebView2.
**Spec:** [MY-D1](design.md), revision MY-D1 / 2026-10-04.
**Requirements:** MY-R1, MY-R4, MY-R6.
**Status/dependencies:** [Authoritative index](index.md#stories-and-readiness).
**Baseline:** DIVA 5444795a2d9db31e158c2cf009d64697e6289e50;
VIVY fc559e6b03ce4e65c0099b9745855dccc4fb067e.

## Global Constraints

Owner authorized inline implementation on 2026-10-04. Respect the index
readiness gates. Preserve Wails delivery, one Runtime/Journal,
sealed Generation, public SDK-only embedding, native sender authority, existing
DTOs and permissions. Root context differs from page context.
Close budget is five seconds; Next batch maximum is 500 and Host queue capacity
is 10,000. Speech limits/namespace are exactly MY-D1 / W3-3.
Proposed source paths do not exist at the inspected baseline.
Integrate accepted predecessor outputs and reconcile source before execution.
Source reads/mock tests do not replace native, provider or owner acceptance.


## Review Focus

Generation must not start services; raw request identity must be native;
candidate/native runtime package versions must match; packaging must preserve the
sealed executable; missing environments must remain pending.

## Task 1: Pin and prove the relevant upstream surface

**Files:** Modify observations.md and index.md. Proposed throwaway probe:
`experiments/mygo/probe/go.mod`, `main.go`, `mygo.json`, `frontend/`.
Do not retain the probe as a second product.
**Consumes:** MY-D1 candidate pin and W3-2/W3-3 boundary requirements.
**Produces:** Exact native API/signature/trust notes and GO/NO-GO/pending result.

- [x] Read pinned app.go/ipc.go/channel.go/protocol.go and platform permission APIs; record exact signatures for close/hide, navigation invalidation, caller identity, single instance, raw protocol and Channel cancellation.
- [x] Create a minimal probe at the candidate commit with a primary and a disposable foreign window; test forged JS window ID, external navigation and revoked/foreign media capability rejection using native request metadata.
- [ ] On Windows exercise hide/reopen, explicit quit during an outstanding call, page reload/channel cancel and raw WAV POST/response; record real WebView2/tool/OS versions and pending access separately.
- [x] Return the API notes and native trace; mark GO only for proven required APIs. Failure is a recorded NO-GO/pending result, not permission to relax W3-3.

## Task 2: Prove compiler, generation and packaging integration

**Files:** Same ledger/probe; branch-only `.github/workflows/mygo-probe.yml`
for Windows evidence; future W2/W3 build inputs are read-only.
**Consumes:** Candidate source pin; staged public SDK/build evidence when available.
**Produces:** Reproducible compiler/client/native prerequisites and a packaging recipe
that will consume the exact sealed executable.

- [x] In the isolated probe run `go version`, `go env GOOS GOARCH CGO_ENABLED`, `go mod download`, `go test ./...` and `go build ./...`; then repeat the framework-only build with CGO_ENABLED=0. Pin matching frontend package/CLI versions from actual upstream manifests; do not infer matching version numbers.
- [x] Run the configured TS generator with MYGO_GENERATE and an acquisition counter; verify no Host/native/service acquisition. Inspect stock mygo build for recompile/overlay behavior; document why it cannot replace SDK pack.
- [x] Record W2's required staging/modfile/overlay inputs for MY-1 Task 3 to verify once W2 is accepted. This standalone feasibility Story does not wait for W2 or claim that the public SDK already compiles.
- [x] Record the packaging-only requirements from W6 for MY-2 Task 3. Inspect the probe's application-identity/resource behavior; qualification of the sealed product installer stays in MY-2, after W6 is accepted.
- [x] Record actual failures, toolchain/native dependencies and relevant upstream links; update trigger/affected-gate rows, run scoped diff/link checks and create an English documentation/probe commit.

## Acceptance and handoff

The throwaway probe answers only the named questions. Return exact pins, commands,
native API notes, standalone generator acquisition result and packaging requirements.
MY-1 additionally requires accepted W1/W2/W3; this Story cannot manufacture them.
The initial plan publication ran no tools. [Run 001](run-001.md) now records
standalone compiler/client and Linux native evidence; Windows remains pending.

## Execution result — 2026-10-04

Task 1 remains partial: primary/foreign, revocation, untrusted data-URL navigation,
reload, channel cancellation, hide/reopen and outstanding-call Quit passed on
Linux WebKitGTK. The required Windows/WebView2 run has not happened. Task 2 is
complete as standalone feasibility/build documentation, without claiming a
sealed SDK or installed-product build. See [run 001](run-001.md).
