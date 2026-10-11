# TODOLIST

Updated: 2026-10-11. Current DIVA/VIVY/Laputa follow-ups. Non-current deferred
entries and historical context remain in [`defer.md`](defer.md).

- [ ] **Unified context policy / DIVA Recap UI** — Implement and verify
  [VIVY #58/#57 alignment](docs/architecture/memory-policy-alignment.md):
  one authorized subject-level free-form Recap, full inspect/edit/delete UI,
  revision conflicts for concurrent AI/human edits, commit-before-raw-context
  unload with raw evidence preserved, targeted active deletion, and normally
  hidden inspect/update/delete Agent tools explicitly activated via deferred
  discovery. Define subject binding, conflict/recovery/deletion contracts and
  discovery mechanics; preserve Work/reconcile/capture compatibility. LIFE is
  the default-enabled first pilot; coding is excluded in phase 1. No second
  runtime/authority/compaction path, automatic frozen writes or backend migration.
- [ ] **Capability-aware backend and product validation** — BML-first validation,
  then owner-led Mentle validation through DIVA using the same abstraction;
  no DIVA backend switch or dual writes. Preserve developer recall,
  correction/deletion and six-cut proofs; close installed Windows/real-model,
  unknown/partial-effect, backend recovery and workspace matrices on one fixed
  candidate. [Memory-loop status](docs/plans/diva-next/memory-loop/README.md)
  remains authoritative; redesign/re-estimate V10/S05 and V19/S08 without
  changing pinned historical evidence. Agent ACTMEM read tools remain a gap.
- [ ] **DIVA consumer compatibility with VIVY main** — VIVY retired
  `sdk/host/v1` and SDK `shared`/`go-host` targets, but pinned DIVA
  `internal/desktop/app.go` and `scripts/build-desktop.py` still use them.
  Define and validate the replacement seam/candidate before changing source
  pins; no parallel runtime or pipeline. See the
  [runbook limitation](docs/plans/diva-next/memory-loop/runbook.md).
- [ ] **DIVA PR #21 author attribution** — The merged PR was created by
  `mastwet`, but its commits resolve to `mas19192` via `0104988com@gmail.com`.
  Attribution remains unresolved; only revisit correction after an explicit
  decision to rewrite shared `main`.

Completed integration follow-ups (preserved):

- [x] **Laputa PR #5 / VIVY PR #46 dependency sequence** — Both merged;
  Laputa merge `4b2bec2`, VIVY merge `59a673ac`. VIVY PR #48 merged at
  `e67e6ff`, including BML-FIRST-INIT-CANCEL.
- [x] **VIVY CI run #77 follow-up** — Historical failure remains evidence.
  Exact VIVY `ce33bc8` [run 38095646661](https://github.com/ProjectViVy/agent-vivy/actions/runs/38095646661)
  passed all four checks, including the full supported DIVA memory lane.
  This does not close installed-product acceptance or implement the new policy.
