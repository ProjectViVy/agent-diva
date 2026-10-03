> 2026-10-03 scope amendment: [DN-C2](p0-design.md) and [index.md](index.md)
> take precedence over historical P0-D1 host/speech/import/acceptance premises.
> Tasks/evidence below retain their original scope and artifact pins. New
> closure work is not proved by historical implementation or acceptance.

# DN-0 — Inventory and contract freeze Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` after plan review and implementation authorization; no delegation is implied. Read both this plan and the shared design before execution.

**Spec:** [p0-design.md](p0-design.md), revision P0-D1. **State/dependencies:** [index.md](index.md) is authoritative.
**Baselines:** DIVA `d96e396d1641a5e7636e30e1f6cdbd0e597b62c8`; VIVY `5347032d8f18a047b67e85761c0dcc48728bee7c`.
**Global constraints:** one VIVY authority; no legacy Manager fallback; no production-data testing; do not change approved scope to close a gate. New paths below are proposed. Record actual commands/results and scoped commits; never claim mocks as live acceptance.

**Goal:** Produce an exhaustive semantic migration ledger and implementation-ready core contracts.
**Architecture:** Read both pinned repositories and retired DIVA code in git history; freeze shared boundaries once.
**Tech stack:** Vue/TypeScript, Tauri/Rust, Go VIVY as applicable to this Story.

## Inputs and outputs

Consumes current DIVA source and historical `0fd005a1`/origin/dev command registrations, plus current VIVY control/action schemas. Produces proposed `backend-separation-contracts.md` and `fixtures/core-rpc.json` in this plan directory. The fixture is a redacted transcript captured from the selected backend, not an invented protocol.

## Ordered tasks

- [ ] Enumerate every invoke/listen/direct provider call and wrapper in `agent-diva-gui/src/`, excluding tests and stories from production totals. Inspect retired Rust registrations through git history. Record source file/symbol, old behavior, old event/DTO, persistence/credential owner, current consumers and fixture for every row; do not use regex counts alone as completeness proof.
- [ ] Trace each row to the selected VIVY `internal/rpc/control.go` and action registration/schema or to a native-only shell operation. Record exact target, capability, input/output/error transformations, IDs/revisions, replay semantics, revision, acceptance and owning DN Story. Labels: existing backend / native / backend gap / explicit retirement pending approval / offline import. Required flags default to required, not silently optional.
- [ ] Capture core `initialize`, `session/create`, `session/list`, `session/get`, `session/messages`, `turn/start`, `run/get`, `run/log`, `run/subscribe`, `run/cancel`, `approval/list`, `approval/respond`, `question/list`, `question/respond`; verify their actual parameter/error/event schemas. Do not equate session/get metadata with message history. Investigate session deletion and planning/work review separately; no invented mapping to approval/respond.
- [ ] Freeze ABI v1 from P0-D1 with exact queue/poll/input bounds and shutdown behavior. Verify cgo/target/pack constraints and the gatewayless lifetime issue. Record chosen Windows target/toolchain and runnable verification host; if unavailable, leave native implementation blocked while completing the read-only ledger.
- [ ] Inspect the pinned Eino/runtime capability path and VIVY compiler/Generation requirements. Record the DLL build target and header/identity checks needed by DN-L; do not bypass sealed admission.
- [ ] Publish the contract record with unresolved domain APIs individually named and traced to source. Amend downstream plan file lists if the ledger reveals missed consumers; no business implementation in this Story.

## Verification and handoff

Run `rg -n 'invoke|listen|@tauri-apps|fetch\(' agent-diva-gui/src` as a discovery aid; compare source rows with AST/import traversal and historical command inventory. Every production consumer must have a disposition, including indirect calls. Validate fixture JSON with `python -m json.tool docs/plans/diva-next/fixtures/core-rpc.json`. On a provisioned Go host run `go test ./internal/app ./internal/rpc -run 'LoopbackControl|Gatewayless|Subscribe' -count=1` from VIVY and capture the core transcript. Missing environment is recorded as pending, not success.

Review focus: aliased invokes; direct network bypasses; similarly named but semantically different APIs; non-journaled events; required domains incorrectly marked optional. Return ledger coverage, frozen schemas, target/toolchain evidence and exact blockers. DN-1/DN-L do not become Ready from a ledger with unresolved core contracts.
