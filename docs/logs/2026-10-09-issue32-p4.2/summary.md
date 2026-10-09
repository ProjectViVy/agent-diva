# Issue #32 P4.2 — Shared teardown and singleton handoff

## Scope

Implemented H5/H7 in the isolated `feat/issue32-desktop-gates` branch. DIVA
elects through Wails before opening the sealed VIVY host, queues a secondary
launch until the primary window is ready, closes admission and revokes the
window capability before teardown, and gives the pump, speech requests, and
host close one shared `CloseBudget` context.

The teardown worker keeps ownership after its caller deadline. A Wails drain
barrier is registered before `RuntimeService`; Wails' reverse service shutdown
therefore runs the bounded coordinator first and waits on the barrier before
releasing its single-instance lock. Partial composition cleanup preserves the
composition and cleanup errors. `cmd/diva/main.go` already exits nonzero after
composition or run errors, so it needed no source change.

## Changed behavior

- A second process is rejected by Wails before `hostv1.Open`, preventing it
  from opening another profile or Journal.
- Early second-instance notifications coalesce until window composition is
  complete; startup failure or shutdown discards them.
- Pump drain, speech shutdown, and host close share the original shutdown
  deadline. Errors remain discoverable with `errors.Is` and are retained after
  late worker completion.
- Wails retains the singleton lock until any teardown worker has finished,
  including work that continues after the bounded service callback returns.
- Speech shutdown accepts a caller-owned context while preserving the existing
  duration-based API.

## Delivery

Implementation and regression tests are committed as `85543ff4` on the
isolated feature branch. No push, merge, tag write, upload, or release was
performed. Native
two-process acceptance remains pending; see `verification.md` and
`acceptance.md`.
