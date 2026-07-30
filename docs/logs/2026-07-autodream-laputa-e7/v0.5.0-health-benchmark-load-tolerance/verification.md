# Verification

- Failure reproduced inside the parallel workspace lane at 3.3496501 and
  5.599332 seconds for 500 requests.
- The same test passed in focused Manager runs.
- `just health-benchmark-check` is rerun after this change.
- The complete E7 aggregate gate is rerun from the beginning.

The benchmark is ignored only by the ordinary workspace suite and is mandatory
in the aggregate gate through `just health-benchmark-check`. The acceptance
ceiling remains deterministic and bounded at 10 ms per request.
