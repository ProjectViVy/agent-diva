# Plan mode exit boundary (P0)

## Problem

GUI plan mode exited after explore turns even when no plan report was produced. Root causes:

1. `ChatView` watched `executingPlan.plan_id` and forced `execMode = agent` on any restore.
2. `get_active_plan` picked a **global** Approved report, projecting it as Execute after every turn.

## Fix

- Session-scoped `get_active_plan(sessionKey)`: AwaitingApproval for this session, else report bound to this session’s active execution; **no global Approved fallback**.
- Remove executingPlan→agent watch; leave plan only on explicit approve (or manual mode switch / revoke keeps plan).
- `restoreActivePlanRuntime` always passes current session key.

## Exit contract

| Event | execMode |
|-------|----------|
| Explore / demux pending | keep plan |
| User approve | agent |
| User manual Agent/Ask | as selected |
| restore other session | must not change mode |

## Delivery scope

- Made plan reports and active execution lookup session-scoped across the agent, manager/Tauri bridge, and GUI.
- Preserved plan mode through exploration and pending-approval states; only explicit approval starts execution mode.
- Added plan-report demux, history, approval-card, execution-state, mode-exit, and local-storage regression coverage.
