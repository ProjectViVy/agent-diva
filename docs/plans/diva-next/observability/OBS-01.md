# OBS-01 — Terminal/file logging and prettylog Implementation Plan

> **For agentic workers:** After specification review and implementation authorization, use `superpowers:executing-plans`. Read this plan and OBS-D1 first. Delegation follows repository instructions, not this heading alone.

**Goal:** Make terminal output readable without changing redirected/file semantics or losing redaction.
**Architecture:** Existing logging.Setup owns independent handlers; standard slog.NewMultiHandler fans out sanitized records.
**Tech stack:** Go 1.26.4, slog, prettylog, existing x/term.
**Epic / requirements:** OBS-A / O1
**Specification:** [architecture.md](architecture.md), OBS-D1. **State/evidence:** [parent index](../index.md).
**Immediate inputs:** None; release after OBS-D1 review. DN-L consumes the owned logger later; ABI work is not a unit-test prerequisite.
**Global constraints:** one VIVY authority; existing Eino execution and Recipe/Generation path; no legacy fallback, production data, synthetic usage, guessed wire adapters or fabricated acceptance. VIVY code changes require its AGENTS and pinned-capability check. DIVA writers claim LOCK.md before edits and serialize shared files.
**Path convention:** VIVY paths are relative to the selected agent-vivy checkout; DIVA paths to agent-diva. New paths explicitly marked proposed. Source baselines are in OBS-D1.

## Files and exact seam

Modify VIVY `internal/logging/{logging.go,logging_test.go,redact.go,redact_test.go}`, `internal/config/{config.go,config_test.go}`, `cmd/vivy/main.go`, relevant command tests and `go.mod/go.sum`. Add proposed `internal/logging/console.go` / `console_test.go` only if descriptor detection needs a test seam. Preserve `cmd/vivy/run.go` stdout behavior. Use existing logging documentation rather than a second setup guide.

Keep `Setup(opts Options) (*slog.Logger, Effective, io.Closer, error)` and `ResolveEffective(level,format string) (Effective,error)` compatibility. Add proposed `NewBootstrap(stderr *os.File) *slog.Logger` for pre-config errors using the same console/redactor seam without a file; add console resolution internally; do not break callers of the existing exported resolver. Options gains ConsoleFormat; Effective reports resolved console/file selection. Consume [D1](architecture.md#d1--split-console-and-file-handlers). Produce an exact dependency pin, passing handler/config tests and an embedding ownership note for DN-L.

## Ordered tasks

- [ ] Add failing `TestSetupSeparatesConsoleAndFileFormat`: fake terminal gets pretty text while the daily file is valid JSON. Add redirected JSON/no ANSI, explicit pretty/json/text overrides, NO_COLOR, Stdout=false, level filtering, attrs/groups and strict env/config validation cases. Inject the console descriptor/writer only in a package-local test seam.
- [ ] Resolve prettylog at merged commit `8b8e037` with Go; record the resolved version/sum. Check its Handler signatures and existing merged regression tests before integration. No new prettylog feature or unrelated release is needed.
- [ ] Replace MultiWriter's shared format with two handlers and standard slog.NewMultiHandler. Keep one outer redactor. Add `TestRedactionResolvedValuesAcrossBothSinks` with nested LogValuer/error/Stringer sentinel secrets; assert neither captured sink contains them.
- [ ] Test and preserve midnight rotation, retention, flush/close, handler write errors and config reload scope. Setup failure returns an error; no silent fallback that loses the file sink.
- [ ] Route main bootstrap through this seam; test pre-config errors on stderr and headless assistant output without diagnostic stdout. Give DN-L the one-init/process-close ordering, without modifying its ABI/Generation lane.
- [ ] Update logging config/docs and commit `feat(logging): split console formatting from file output`.

## Verification and handoff

From VIVY: `go test ./internal/logging ./internal/config ./cmd/vivy`, `go test -race ./internal/logging`, `go vet ./internal/logging`; then required `just ci` for changed runtime/config contracts. Expected red: new TTY/file separation and console override cases fail on the baseline. Expected green: all focused cases pass; JSON parsing succeeds, sentinel/ANSI assertions pass, no race or swallowed write error. Use temporary log/data directories; never inspect production logs.

Return source commit, module version/sum, resolved config examples, sanitized terminal + redirected + file samples, test output and DN-L ownership requirements. Review focus: independent format selection, redactor resolution, close ordering and unchanged headless stdout.
