# Acceptance

1. Configure heartbeat retry settings under `heartbeat` and confirm config show/diff surfaces the new fields.
2. Trigger a heartbeat decide failure and confirm retries log warnings with bounded backoff before returning or emitting an explicit `error` outcome.
3. Put presence into `Distracted` or `Gone` and confirm effective cadence uses `presence.distracted_heartbeat_multiplier` instead of a fixed slow constant.
4. Put presence into `Away` and confirm heartbeat does not execute ticks until the next re-check cycle.
