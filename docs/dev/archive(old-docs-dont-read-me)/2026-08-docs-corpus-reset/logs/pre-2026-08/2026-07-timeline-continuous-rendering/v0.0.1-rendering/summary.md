# Timeline Continuous Rendering

## What was changed
1. Removed the recently added `1h` (1 hour) period and `minute` interval logic, as it was deemed useless by the user. The default period has been reverted to `1d` (1 day).
2. Modified the backend timeline chart rendering logic (`timeline_handler` in `agent-diva-manager`) to generate a continuous sequence of time buckets starting exactly from the target past duration (e.g. `Utc::now() - Duration::days(1)`) up to the current exact time (`Utc::now()`).
3. Buckets without any token usage are now pre-filled with zero values, ensuring the frontend renders a continuous, correctly scaled trend chart that accurately visually represents empty periods.

## Impact range
- `agent-diva-manager/src/handlers/token_stats.rs`
- `agent-diva-gui/src/api/tokenStats.ts`
- `agent-diva-gui/src/components/console/TokenStatsPanel.vue`
