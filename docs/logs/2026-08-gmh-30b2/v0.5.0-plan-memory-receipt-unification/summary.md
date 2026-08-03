# GMH-30B2 Summary

## Outcome

Plan approval and Memory apply now share the production governance coordinator
opened from `.laputa/governance.db`. Both domains bind durable decisions to a
canonical digest, consume one-time receipts before side effects, and recover
incomplete work without persisting Plan markdown or Memory proposal patches in
the governance ledger.

## Changes

- Manager bootstrap opens one governance coordinator and injects it into
  Sandbox, Plan, and Memory approval paths.
- Plan revisions create digest-bound `PlanExecute` requests. Editing a revision
  revokes the previous request. Approval first persists an execution context,
  then consumes the receipt, and finally materializes the canonical Plan/TODO
  transaction idempotently.
- Memory apply first persists the prepared journal, consumes the receipt, marks
  the journal as pending apply, and only then performs the typed Memory change.
- Startup recovery paginates the current workspace. Pending work remains
  pending; dangling Allowed work is revoked and resubmitted; prepared Consumed
  work is completed exactly once; missing recovery material fails closed.
- Domain mapping tables retain recovery material while governance events remain
  payload-free.

## Impact

This closes GMH-30B2 only. Manager HTTP/SSE reason-code evolution remains
GMH-31, GUI approval UX remains GMH-32, and integrated release acceptance
remains GMH-33.
