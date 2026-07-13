# Usage Rate Removal

## What was changed
Removed the "Usage Progress / Usage Rate" bar from the Token Statistics UI in `TokenStatsPanel.vue`. The previous calculation assumed a fixed 200K token budget, which did not make sense when viewing aggregated statistics across varying time periods and sessions, since the context window budget is per-session rather than a global cap across all requests.

## Impact range
- `agent-diva-gui/src/components/console/TokenStatsPanel.vue`: Removed `getUsagePercentage` and the `div.usage-progress` template block along with its corresponding scoped CSS.
