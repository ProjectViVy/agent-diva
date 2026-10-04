# MY-1: Build the sealed MyGo Host and Agent transport — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: after implementation authorization,
> use **superpowers:executing-plans** task by task. Delegation is not selected.
> Steps use checkbox syntax; owner authorization is recorded in the index.

**Goal:** Build the sealed MyGo Host and Agent transport.
**Architecture:** Use one research lifetime owner over the public SDK and a page-scoped Channel adapter; reuse the retained frontend contracts.
**Tech Stack:** Pinned MyGo v0.2.4, isolated Go 1.27.1, public VIVY SDK,
retained Vue/TypeScript and Windows WebView2.
**Spec:** [MY-D1](design.md), revision MY-D1 / 2026-10-04.
**Requirements:** MY-R2, MY-R3, MY-R4.
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

Duplicate subscriptions must not duplicate Next readers; a cancelled page must
not close Host; quit must not block the UI thread; partial startup must not
publish ready; generated/package tooling must not bypass sealing.

## Task 1: Implement the single lifetime owner and native callbacks

**Files, proposed:** Create cmd/diva-mygo/main.go,
internal/desktop/mygo/owner.go, app.go, owner_test.go and app_test.go.
**Consumes:** Accepted W1 public Host contract and MY-0 native API notes.
**Produces:** NewOwner, Start, Call, Subscribe and Shutdown signatures from MY-D1.

- [ ] Add failing TestPageCancelKeepsHost, TestQuitClosesOnce, TestStartupFailureUnwinds and TestShutdownTimeoutIsUnclean; fake only the public Call/Next/Close port and acquired resources in tests. Assert one open/close, zero ready on partial failure, preserved Host after page cancel, and no new Host after timed-out close.
- [ ] Implement ownership with an app root context and admission state; delegate runtime work to the public Host. Main-thread callbacks schedule bounded background teardown; a guard allows the final native Quit exactly once.
- [ ] Use MY-0-proven single-instance/primary-window APIs before opening the data root; rejected second instances activate the primary. Hide/reopen preserves Host; reload/hide revokes speech context.
- [ ] Run race tests through W2's staged consumer modfile. Expected: all named lifecycle assertions pass; native shutdown/second-instance behavior is independently reproduced on Windows.

## Task 2: Adapt RPC, events and the retained TS facade

**Files, proposed:** Create internal/desktop/mygo/bindings.go, bindings_test.go,
agent-diva-gui/src/platform/mygo-host.ts, mygo-host.test.ts and generated/mygo/client.ts.
**Existing:** Modify agent-diva-gui/src/platform/desktop-host.ts and src/api/vivy/transport.ts.
**Consumes:** W3-1 Call/Next/error contract; existing VivyTransport, VivyCallRequest,
WireEvent and public speech facade; accepted W3 frontend seam.
**Produces:** RuntimeBindings.Call/Subscribe and createMygoTransport(): VivyTransport.

- [ ] Add failing TestDuplicateSubscriberRejected, TestSlowPagePreservesGap and TS listener-race/dispose/reload tests. Assert one Next reader, sticky gap/refetch semantics, no automatic mutation retry and no Host close from frontend disposal.
- [ ] Bind the primary native CallerWindow only; normalize CallRequest/CallReply with unchanged numeric errors and default 120-second call deadline. Never trust browser window IDs.
- [ ] Implement one page-scoped Channel subscription and atomic reader admission. Cancellation releases Next/Send/reader ownership; slow frontend may fill the bounded Host queue but never blocks runtime producers. Map gap/lost to the existing WireEvent union.
- [ ] Generate the client into the prescribed path. Replace framework imports only behind the facade/transport; retain client/store/component behavior. Regenerate and check drift; run retained frontend tests plus the new focused tests.

## Task 3: Produce an inspectable candidate without a second build

**Files:** Research-only modifications to accepted go.mod/go.sum,
build/vivy-sources.lock.json and scripts/build-desktop.py; reuse accepted frontend assets embedding.
These files are proposed at baseline, supplied by W2/W3 before this task.
**Consumes:** W2 staged source lock, temporary consumer modfile, assembly overlays,
Go-host pack and Inspect; candidate compiler/client pins from MY-0.
**Produces:** Sealed executable and untracked input/artifact report for MY-2.

- [ ] Add a bounded wrapper option --host-package with default ./cmd/diva and candidate ./cmd/diva-mygo; pass it to the existing pack target. Keep the delivery default and checksum/sealing/source validation.
- [ ] Pin MyGo/Go 1.27.1 on the research branch only. Generate bindings using the staged modfile/overlay with no service startup, then build the frontend once and pack the real candidate entrypoint.
- [ ] Verify go list -deps for the candidate: no VIVY internal imports or Wails host in its runtime import closure. Record any actual CGO dependencies instead of assuming none.
- [ ] Run `python scripts/build-desktop.py --mode test --host-package ./cmd/diva-mygo`,
then `--mode build --host-package ./cmd/diva-mygo`, and VIVY Inspect on the returned artifact. Expected: sealed inputs/generation/frontend bytes match and tampered/unsealed inputs fail.
- [ ] Record candidate pins, test/native/build outputs and commit only the scoped adapter/build changes; update index evidence. Product parity remains MY-2/MY-3.

## Acceptance and blockers

W1/W2/W3 are unaccepted at inspected baseline. Before execution reconcile actual
SDK signatures, wrapper flags and generated asset location with their accepted
source. Public contract drift or inaccessible required native APIs blocks work;
do not create another backend or weaken sealing to make the sample launch.
Return one owner/reader proof, race/frontend results and the inspected artifact.
