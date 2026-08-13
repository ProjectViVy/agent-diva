# Token Stats Stacked Bar Chart

## What was changed
Transformed the Token Statistics timeline chart bars into stacked bar charts to visually distinguish between input tokens and output tokens within each time bucket. The top portion represents output (green) and the bottom portion represents input (blue).

## Impact range
- `agent-diva-gui/src/components/console/TokenStatsPanel.vue`: Modified the template to render two inner `div` elements within `.timeline-bar`. Added CSS classes `.timeline-bar-output` and `.timeline-bar-input` with appropriate gradient colors and hover states.
