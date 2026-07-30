# E7 Single Side-Effect Seam

## Outcome

GMH-40 is closed. AgentLoop now retains the active phase, execution session and
background-task context whenever runtime configuration rebuilds the tool
registry. Ask/Assist-Mask read-only state and scheduled origin are captured once
in the turn policy snapshot and reused at the final execution seam.

Plan verification now fails closed when no execution TODO evidence exists.
Exact one-to-one pre-materialized step TODOs are accepted idempotently, while
arbitrary preexisting TODOs remain a conflict. The obsolete duplicate transition
matrix in `PlanOrchestrator` was removed.

## Safety impact

- Ask and read-only masks cannot invoke mutating registry executors.
- Plan phases cannot bypass the capability policy through runtime tool rebuilds.
- Cron-triggered turns cannot recursively schedule cron work.
- Subagents cannot receive spawn, cron, background enqueue or plan-control tools.
- Cancellation remains a dynamic pre-execution check.

No external API, desktop key, user Memory payload or manual GUI action was used.
