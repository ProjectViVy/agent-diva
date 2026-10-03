# v0.4.0 — DN-LC verification

Plan gate: `go test ./sdk/internal/assembly ./internal/app
./internal/observerhost ./internal/modules/diva-cognitive
./internal/cognitivecontract -count=1`

Results (agent-vivy @ 2f009eee, Go 1.26.4, `GOSUMDB=off`):

- `./internal/app` — ok 7.45s. New tests: omitted capability → nil
  bundle; selected capability without typed factory → init error
  (fail-closed); owned capture subscription without ObserverHost →
  error, with ObserverHost → composed host (sqlite temp backend).
- `./internal/observerhost` — ok.
- `./internal/modules/diva-cognitive` — ok 0.06s, 8 tests: relative
  data dir rejected; source/sink/mission arm without a memory backend;
  real durable capture against a live Garden owner + redelivery dedupe
  (same Seq/IngestionID); foreign binding rejected; AttachRuntime
  single-use; unarmed dispatch → `ErrUnarmed`; armed control actions
  (status/policy.get/policy.set/trigger/cancel) route to the armed
  cell; 20 providers' definitions match the descriptor inventory and
  fail closed on a foreign host.
- `./internal/cognitivecontract` — no test files (interface package).
- `./sdk/internal/assembly` — 3 FAILs are pre-existing env gaps:
  `TestGenerateNonEmptyUIAssemblyTypechecks`, `TestTask7*` need
  `ui/node_modules/.bin/tsc` which is absent; every non-UI test passes
  (`-skip 'UI|Task7'` → ok 0.59s).
- `go build ./...` repo-wide green.
- laputa garden: `GOSUMDB=off go test ./agentapi -count=1` ok with the
  new `BindEvolutionSource`.

Environment/known limits: no `ui/node_modules` on this VM (UI-typecheck
tests unrunnable); mentle models dir absent → retrieval degrades to
lexical/spool per garden design; live mentle writer path (Effects +
RunDomain) is exercised only when the selected backend is present —
`BoundDomain` keeps the eager writer check by design.
