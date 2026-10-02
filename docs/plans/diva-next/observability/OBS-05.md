# OBS-05 — Bounded diagnostic reads and GUI capture API Implementation Plan

> **For agentic workers:** After specification review and implementation authorization, use `superpowers:executing-plans`. Read this plan and OBS-D1 first. Delegation follows repository instructions, not this heading alone.

**Goal:** Provide usable runtime/GUI diagnostic data without turning log files into domain state or allowing arbitrary paths.
**Architecture:** Existing logging ownership supplies rotating families; existing control RPC validates bounded queries and writes.
**Tech stack:** Go logging/file I/O, existing control peer/RPC authorization.
**Epic / requirements:** OBS-A / O5
**Specification:** [architecture.md](architecture.md), OBS-D1. **State/evidence:** [parent index](../index.md).
**Immediate inputs:** OBS-01: accepted file ownership, format separation and redactor. DN-L must later expose these owned services in the embedded package.
**Global constraints:** one VIVY authority; existing Eino execution and Recipe/Generation path; no legacy fallback, production data, synthetic usage, guessed wire adapters or fabricated acceptance. VIVY code changes require its AGENTS and pinned-capability check. DIVA writers claim LOCK.md before edits and serialize shared files.
**Path convention:** VIVY paths are relative to the selected agent-vivy checkout; DIVA paths to agent-diva. New paths explicitly marked proposed. Source baselines are in OBS-D1.

## Files and exact seam

Modify VIVY `internal/logging/{logging.go,redact.go}` only at reviewed ownership seams; add proposed `internal/logging/{diagnostics.go,diagnostics_test.go}`, `internal/rpc/diagnostics.go` / `diagnostics_test.go`, and scoped handler/capability changes in `internal/rpc/control.go`. App supplies the service through its existing owned dependency pattern; inspect `internal/app/app.go` constructors before wiring. Do not import internal packages from the external DIVA host.

Expose proposed `diagnostics/logs(DiagnosticQuery): DiagnosticPage` and `diagnostics/gui/append(GuiLogBatch): GuiLogAck` exactly per [D5](architecture.md#d5--bounded-diagnostic-rpc-proposed). Package-local logging interface: `Read(ctx context.Context,q DiagnosticQuery) (DiagnosticPage,error)`, `AppendGUI(ctx context.Context,batch GuiLogBatch) (GuiLogAck,error)`, `Close() error`; Go types use snake_case JSON tags matching D5. The owning Setup/service assembly chooses file roots, never the caller.

## Ordered tasks

- [ ] Add failing tests for missing capability, invalid source/date/cursor/limit, forbidden paths, symlink escape and cancellation. Implement D5's named page/batch/queue/record/scan limits with boundary tests: 500 records, 8KiB per record, 500 × 8KiB scan/batch bytes. Supply verified bounds/fixtures to DN-0's freeze; do not silently widen them. No arbitrary path parameter is accepted.
- [ ] Implement runtime/GUI family enumeration, bounded JSON/text decoding, stable file-offset IDs, server-issued source/date/identity cursors and gap detection on rotation/retention/truncation. Test empty/date-not-found, malformed line, filter scan ceiling and clipping. Read does not return secrets from old lines.
- [ ] Reuse the owned daily writer for a distinct GUI family. Validate/sanitize records before write; test reported accepted count, write failure, partial write error with accepted count, rotation/retention and repeated close. Do not publish successful acknowledgments before a write succeeds.
- [ ] Register RPCs/capabilities in the existing control handler; preserve peer authorization and existing error shape. Add closed/shutdown, concurrent read/write and disconnected-call tests; no recursive logging through this service.
- [ ] Wire lifetime to the app's existing closer ordering. Record DN-L initialization options and shutdown requirements. Leave global audit explicitly unresolved instead of mapping it to diagnostic lines.
- [ ] Commit `feat(diagnostics): add bounded runtime and gui log access`.

## Verification and handoff

`go test ./internal/logging ./internal/rpc ./internal/app`, `go test -race ./internal/logging ./internal/rpc`; required `just ci`. Expected red: baseline has no producer methods and unsafe/missing acknowledgments cannot meet fixtures. Expected green: caps/limits/containment/error/rotation cases pass with temp directories; concurrent reads/writes retain bounded behavior.

Return exact JSON fixtures including empty/gap/partial-write errors, named resource bounds and rationale, service ownership/close order, commit and commands to OBS-08/DN-L. Review focus: read containment, cursor invalidation, redaction, bounded scan, truthful write acknowledgment and no global-audit equivalence claim.
