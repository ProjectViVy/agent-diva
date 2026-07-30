# TODOLIST triage — archive and residual rewrite

## Summary

Root `TODOLIST.md` was triaged as documentation-only governance:

1. Historical Plan/TODO P1–P3 review checklists (2026-07-11 baseline) received
   a full disposition table (CLOSED / SUPERSEDED / OBSOLETE / OPEN-RESIDUAL).
2. Completed GMH Phase 0–2 (through GMH-24), G0 reconciliation, completed
   deferred items, and the abandoned Deferred Review Program evidence moved to
   archive files.
3. The main backlog now keeps **Active Plan** (G2D + Evolution freeze), open
   GMH-30..53 stories, real deferred product debt, reliability items, and a
   short **Plan residual** list rewritten against current code paths.

## Impact

- Readers no longer mistake unchecked 2026-07-11 review bullets for active work.
- Severity labels use `sev-P0..P3` to avoid collision with “Plan phase P1/P2/P3”.
- Milestones M0–M2 marked complete in archive; M3–M5 remain open.
- No runtime code changed.

## Scale

| Metric | Before (approx.) | After |
|--------|------------------|-------|
| TODOLIST size | ~66 KB / ~529 lines | slim active-only |
| Open checkboxes (mixed historical) | ~91 | only real open work |
| Archive files added | — | 2 new + iteration log |

## Related paths

- `TODOLIST.md`
- `docs/archive/todolist/completed-through-2026-07-30.md`
- `docs/archive/todolist/plan-todo-p1-p3-disposition-2026-07-30.md`
