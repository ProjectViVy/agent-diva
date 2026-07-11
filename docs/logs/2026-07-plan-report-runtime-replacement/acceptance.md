# Acceptance

## Accepted outcomes

- PLAN and TODO are separated in the main runtime path:
  - plan mode creates Markdown report artifacts;
  - execution TODO tools only appear when an approved execution session is active.
- The old `/api/plans*` gateway routes and manager handlers are removed.
- GUI plan history no longer renders legacy step/TODO/evidence structures.
- Plan approval UI can approve, edit, refresh, and select execution context policy.
- Read-only plan-mode prompt no longer asks the agent to call legacy planning tools.

## Deferred cleanup

- Physical deletion of all legacy core planning modules is deferred because some compatibility tests and types still compile against them.
- The old Tauri compatibility command names remain as shims over plan reports to avoid unnecessary GUI churn in this iteration.
