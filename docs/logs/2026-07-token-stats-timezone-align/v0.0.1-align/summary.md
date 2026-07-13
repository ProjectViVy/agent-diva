# Token Stats Timezone Alignment

## What was changed
Implemented dynamic timezone-aware queries for token statistics. 
- The frontend now sends the browser's local timezone offset (`tz_offset`) to the backend.
- The backend aligns "1d" (Today) to local midnight, rather than `Utc::now() - 24 hours`.
- Added a `half_hour` interval with 30-minute granularity, which correctly applies to "1d" timelines as requested by the user.

## Impact range
- `agent-diva-gui/src/api/tokenStats.ts`: Added `tzOffset` payload and `half_hour` interval.
- `agent-diva-gui/src-tauri/src/commands.rs`: Pass `tz_offset` through the Tauri IPC commands.
- `agent-diva-manager/src/handlers/token_stats.rs`: Implemented `tz_offset` parameter parsing and local midnight alignment in `since_for_period`. Added `half_hour` bucket generation and aggregation in `timeline_handler`.
