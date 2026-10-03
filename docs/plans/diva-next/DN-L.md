# DN-L — Sealed Go shared library Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` after plan review and implementation authorization; no delegation is implied. Read both this plan and the shared design before execution.

**Spec:** [p0-design.md](p0-design.md), revision P0-D1. **State/dependencies:** [index.md](index.md) is authoritative.
**Baselines:** DIVA `d96e396d1641a5e7636e30e1f6cdbd0e597b62c8`; VIVY `5347032d8f18a047b67e85761c0dcc48728bee7c`.
**Global constraints:** one VIVY authority; no legacy Manager fallback; no production-data testing; do not change approved scope to close a gate. New paths below are proposed. Record actual commands/results and scoped commits; never claim mocks as live acceptance.

**Goal:** Build and inspect a DIVA Generation DLL with a safe C ABI and durable embedded lifecycle.
**Architecture:** Keep cgo wrappers thin; one embedded Go host delegates all business work to the existing control peer.
**Tech stack:** Vue/TypeScript, Tauri/Rust, Go VIVY as applicable to this Story.

## Files and contracts

VIVY existing: `internal/app/app.go`, `internal/app/facehost.go`, `internal/app/facehost_test.go`, `sdk/internal/frontend_v1.go`, `sdk/internal/frontend_v1_test.go`, `sdk/generation/manifest.go`, `sdk/internal/assembly/` only if its verified target contract requires change. Proposed: `internal/embedded/host.go`, `internal/embedded/host_test.go`, `cmd/vivy-shared/main.go`, `cmd/vivy-shared/exports.go`, `recipes/diva.vivy.yml`, `tests/ffi/smoke.c`. Generated output is not hand-edited.

Consumes DN-0 core RPC/ABI/target freeze. Produces all five P0-D1 exports, generated header, sealed manifest, DLL, artifact hash and C smoke evidence. `internal/embedded.Host` owns App, Peer and event queue; Rust never imports Go internals. Proposed Go surface: `Open(ctx, options) (*Host,error)`, `Call(ctx,method,params)`, `Poll(ctx)`, `Close() error`; exact Go types follow existing rpc/config types frozen in DN-0.

## Ordered tasks

- [ ] Add tests for invalid/duplicate Init, incompatible ABI, wrong/stale handle, partial startup cleanup, gatewayless lifetime before cancellation, concurrent Call/Poll/Shutdown, repeated shutdown and queue overflow. Demonstrate the premature-return assertion against the baseline before fixing its owner.
- [ ] Implement one lifecycle owner using `NewWithAssembly(..., WithoutGateway())`, selected generated Assembly and `DialControl`. Decide WithoutEars solely from the frozen recipe/channel requirements. Keep worker/scheduler lifetime explicit; correct the early-return path without breaking normal gateway shutdown. Preserve Journal terminal writes before storage close.
- [ ] Implement the exported cgo wrappers and library-owned JSON allocations; recover ordinary wrapper panics to a redacted internal error, without claiming recovery from fatal Go runtime errors. No callback pointers, no Go pointers and no dynamic unload. Input/queue bounds use DN-0 constants. Poll overflow reports a gap without blocking RPC dispatch.
- [ ] Extend existing SDK pack parsing/build/inspect with proposed `--target shared` (existing executable target stays default). Compile `./cmd/vivy-shared` with c-shared using the same generated overlays and embedded Generation manifest. Seal/verify artifact kind and ABI/header identity using the existing artifact metadata conventions; if this requires a public manifest change, obtain its review before editing that contract. Add DIVA recipe from the frozen capability ledger.
- [ ] Add compiler/inspection regression tests for executable compatibility, DLL identity, tampered DLL/header, missing modules and failed builds leaving no published Generation. Ensure control actions do not fall back to unsealed embedder behavior.
- [ ] Build on the frozen native target; C smoke loads/links the generated header and exercises Init → initialize → session → turn → approval → cancel → Poll → Shutdown → Free. Repeated allocations are checked for ownership failures. Commit scoped code and evidence under VIVY's contribution rules.

## Verification

From VIVY: `go test ./internal/embedded ./internal/app ./internal/rpc ./sdk/internal/... -count=1` (new embedded package after creation). Build SDK using `go build -o vivy-sdk.exe ./sdk`. Proposed command after implementing the flag: `vivy-sdk.exe pack --recipe recipes/diva.vivy.yml --target shared --output dist/diva-shared`; then existing `vivy-sdk.exe inspect-artifact dist/diva-shared`. The frozen target C compiler runs `tests/ffi/smoke.c` with generated includes/link inputs; DN-0 must record its exact command before Ready.

Expected: native DLL loads, manifest agrees with initialize, memory ownership holds, events retain backend IDs/sequence, cancellation is durable, shutdown finishes within the existing policy. No test executed in this planning session. Stop on missing native toolchain, unresolved generation-format review, unsafe unload or unbounded close.
