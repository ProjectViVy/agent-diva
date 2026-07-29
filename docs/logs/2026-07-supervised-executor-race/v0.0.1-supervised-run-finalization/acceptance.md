# Acceptance

1. Enqueue a supervised run with a registered handler and execute one tick; confirm the record is `Completed` when the tick returns.
2. Enqueue a run without a matching handler; confirm the tick returns only after the record is `Failed` with the missing-handler error.
3. Start a long-running handler, wait until the record is `Running`, then cancel it or mark it lost; confirm the executor exits within the bounded test timeout and preserves the external terminal state.
4. Run `just test` and confirm all supervised executor tests pass under the full workspace load.
