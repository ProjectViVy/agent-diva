# Summary

- Added bounded retry/backoff for heartbeat decide calls with shared behavior across `trigger_now()` and background ticks.
- Extended `HeartbeatConfig` with `decide_max_retries`, `decide_backoff_ms`, and `decide_max_backoff_ms`, plus validation and reload/diff coverage.
- Moved cadence timing math out of fixed `HeartbeatRhythm` constants so heartbeat intervals now follow current presence plus `presence.distracted_heartbeat_multiplier`.
- Synced CLI config fixture/tests with the current provider schema while extending heartbeat config coverage.
