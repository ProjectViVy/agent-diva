# Release

## Delivery mode

- Not released in this iteration.

## Reason

- The change is implemented and locally validated, but workspace-wide `just test` is currently blocked by unrelated pre-existing test failures outside the GUI VRM change scope.

## Suggested release path

1. Resolve or explicitly quarantine the unrelated workspace test failures.
2. Re-run:
   - `just fmt-check`
   - `just check`
   - `just test`
   - GUI smoke for Diva Pet VRM open flow
3. Ship as the next GUI patch release once the above gate is green.
