# Trend Chart Period Change

## What was changed
Changed the default token usage timeline period to display data for the past 1 hour ("1h") with a per-minute ("minute") interval interval bucket, as requested by the user. Previously, the default was 1 day ("1d").

## Impact range
- `agent-diva-gui/src/api/tokenStats.ts`: Added `1h` period and `minute` interval types. Changed default arguments to `1h`.
- `agent-diva-gui/src/components/console/TokenStatsPanel.vue`: Added `1h` option to the UI selector array and set it as the initial active state.
- `agent-diva-manager/src/handlers/token_stats.rs`: Updated `since_for_period` to resolve `1h`, modified the handler `interval` defaulting to support `minute` granularity.
