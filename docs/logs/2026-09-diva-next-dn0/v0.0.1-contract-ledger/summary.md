# DN-0 inventory + contract freeze — iteration v0.0.1

Scope: `docs/plans/diva-next/DN-0.md` tasks 1–6, read-only ledger story.
Deliverables: `docs/plans/diva-next/backend-separation-contracts.md`,
`docs/plans/diva-next/fixtures/core-rpc.json` (capture-pending marker).

## What was done

- Full frontend invoke/listen inventory (158 sites, 150 unique commands, 25
  listen events) reconciled against the retired `0fd005a1` registry (168
  `commands::*` + 1 direct `set_splash_complete`): 21 dead registrations, 1
  phantom invoke (`list_subagent_results`), 4 listeners + 2 emitters with no
  counterpart, 2 direct provider HTTP calls bypassing invoke.
- Every invoked command traced to a VIVY RPC/action at pin `5347032d` or
  labeled native / backend gap / retire pending approval; core 14-method
  surface schemas verified against `internal/rpc/control.go` (params, results,
  error codes, notification shapes).
- ABI v1 bounds frozen (frame 4 MiB, queue 10 000 sticky-gap, poll ≤500,
  call timeout 120 s, shutdown ≤ `shutdownGrace`=5 s, header
  `VIVY_ABI_VERSION 1` identity check).
- Embedded wiring evidence: `app.New`+`WithoutEars`/`WithoutGateway`+
  `DialControl` (net.Pipe JSONL, facehost.go:137) is the sanctioned seam;
  `App.Run` is not usable as embedded liveness (returns immediately when
  gatewayless, app.go:1723-1734).

## New findings ledgered to TODOLIST

- `EMBEDDED-SWEEPER-OWNERSHIP` (sev-P0): InteractionSweeper/CronScheduler are
  Run-scoped; embedded path never starts them → approval/question expiry and
  cron dead in embedded mode. DN-L must decide host ownership.
- `RUN-CANCEL-RESTART-NOTFOUND` (P2): `run/cancel` after restart →
  CodeNotFound; bridge must map it (pair with `background/recover`).
- `APPROVAL-CANCEL-VOCAB` (P2): no approval cancel decision; `cancel_approval`
  mapping needs owner call (deny+reason vs expires_at).
- `VIVY-CONTRACT-BLOCKED` updated: mask has no dedicated RPC (maskcontract
  errors only via module actions).

## Verification performed / pending

- `python -m json.tool fixtures/core-rpc.json` — valid.
- **Executed after env provisioning** (Go 1.26.8 at `~/toolchains/go`):
  `go test -tags vivy_headless ./internal/app -run
  'LoopbackControl|Gatewayless|DN0Capture' -count=1` → `ok agent-vivy/internal/app
  0.398s`. `internal/rpc` has no tests matching that pattern (subscribe
  coverage lives in `internal/app` and `work_subscription_test.go`).
- Fixture `core-rpc.json` is now a real captured transcript (38
  request/response records + 18 `run/event` notifications) via uncommitted
  harness `internal/app/dn0_capture_test.go` in the VIVY pin checkout.
- Capture corrections applied: `session/get` returns `{session, messages}`;
  `CodeNotFound = -32004`; `session/delete` → `{deleted:true}`.
- Still pending: Windows `windows/amd64` c-shared build + header/FFI +
  packaged acceptance; `just`/node not installed (not needed for this Story).

## Governance

- LOCK.md held for DN-0 scope during work; released on commit.
- agent-diva: docs only (ledger doc + fixture + TODOLIST + index + logs).
- agent-vivy checkout: one uncommitted test harness
  `internal/app/dn0_capture_test.go` used to produce the fixture; left in the
  working tree as generation evidence — owner decides adopt-or-delete.
