# Acceptance

GMH-40 is accepted when all of the following are true:

- Ask and Assist Mask expose only read-only operations and forged mutations do
  not enter the registry executor.
- runtime config rebuilds preserve phase, execution and background context;
- cron cannot recursively schedule itself;
- subagents cannot spawn, schedule, enqueue background work or alter plans;
- an execution without TODO evidence cannot verify successfully;
- exact pre-materialized step TODOs replay without duplicates, while mismatched
  TODOs fail closed;
- the core planning policy remains the sole transition matrix.

All criteria have automated evidence. Real desktop acceptance remains deferred
until the complete E7 product path is green.
