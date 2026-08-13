# Iteration Summary

## Outcome

Closed the load-sensitive supervised executor terminal-state race recorded during the test-health backlog reconciliation.

## Changes

- Wrapped `claim_next_supervised` in an explicit SQLite transaction and committed the `UPDATE ... RETURNING` result before returning the claimed record to the executor.
- Added a five-second SQLite busy timeout for transient writer contention.
- Delayed the first heartbeat until the configured heartbeat interval instead of issuing an immediate competing write.
- Propagated completion, failure, and cancellation persistence errors from `tick()` instead of logging and returning success.
- Replaced fixed sleeps in external cancel/lost tests with bounded polling for the observable `Running` state.

## Impact

Supervised runs can no longer have `tick()` return successfully while the durable record remains `Running` because claim visibility or terminal persistence failed.
