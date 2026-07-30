# Acceptance

## User checks

1. Open root `TODOLIST.md`.
2. Confirm **Active Plan** shows G2D six scenarios and Evolution freeze only as
   current mainline work.
3. Confirm there is **no** long unchecked “Plan/TODO P1 core policy / P2 / P3”
   review section; instead an Archive Index points at disposition tables.
4. Confirm open items you still care about remain:
   - GMH-30..53
   - Skill/SOP
   - reliability flaky/MSRV items
   - Plan residual (matrix tests, rebuild phase, empty verify, materialize, dead comments)
   - Wave3 / Mask / UX deferred
5. Open
   `docs/archive/todolist/plan-todo-p1-p3-disposition-2026-07-30.md` and verify
   historical bullets have CLOSED/SUPERSEDED/OBSOLETE/OPEN-RESIDUAL labels.
6. Open
   `docs/archive/todolist/completed-through-2026-07-30.md` and verify GMH-00..24
   completion index is recoverable.

## Pass criteria

- Main list is readable as “current phase + unfinished work”.
- Completed history is archived, not deleted without pointer.
- Residual Plan debt is rewritten to current file paths.
