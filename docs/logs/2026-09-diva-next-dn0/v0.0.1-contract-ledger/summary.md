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
- Pending (no Go/just/node on host): wire fixture capture;
  `go test ./internal/app ./internal/rpc -run 'LoopbackControl|Gatewayless|Subscribe' -count=1`.
  DN-1/DN-L stay out of Ready per contract doc §9.

## Governance

- LOCK.md held for DN-0 scope during work; released on commit.
- No implementation code touched; ledger doc + fixture + TODOLIST + index only.
