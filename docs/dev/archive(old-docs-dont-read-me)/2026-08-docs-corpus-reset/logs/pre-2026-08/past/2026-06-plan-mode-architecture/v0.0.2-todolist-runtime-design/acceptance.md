# Acceptance

## User-Facing Acceptance Steps

- Open `docs/dev/agent-plan/todolist-runtime-architecture.md`.
- Confirm it defines TodoList as generated from Plan rather than as a separate
  planning system.
- Confirm it keeps Kanban, dispatcher loops, profile workers, worker claiming,
  and parallel execution out of scope.
- Confirm it defines `.agent-diva/plans/<plan-id>/todo.json` and `todo.md`.
- Confirm it defines typed TodoList, TodoItem, TodoStatus, and TodoEvent drafts.
- Confirm it explains PlanStep versus TodoItem.
- Confirm it defines a future upgrade path from `Plan -> TodoList -> Kanban`.
- Confirm `docs/dev/README.md` links to the new architecture document.

## Acceptance Result

Accepted when the above documentation checks pass and no code changes are
included in the scoped diff.
