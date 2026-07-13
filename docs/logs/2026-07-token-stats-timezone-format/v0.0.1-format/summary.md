# Token Stats Timezone Format

## What was changed
Fixed a display issue in the `TokenStatsPanel.vue` where the timeline X-axis and tooltips rendered raw UTC RFC3339 timestamp strings (e.g., `2026-07-13T17:00:00+00:00`). Implemented local timezone formatting so that the display aligns with the user's actual browser local time.

## Impact range
- `agent-diva-gui/src/components/console/TokenStatsPanel.vue`: Added `formatTimeBucket` and `formatTimeBucketTooltip` using `Date.toLocaleTimeString` and `Date.toLocaleDateString` for localized display.
