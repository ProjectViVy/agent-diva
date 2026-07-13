# Token Stats Tooltip Enhancement

## What was changed
Improved the `title` tooltip for the Token Statistics timeline chart bars. Previously it only showed the overall "total tokens". Now it displays the total, input, and output tokens separately on different lines for better clarity, using the localized string keys.

## Impact range
- `agent-diva-gui/src/components/console/TokenStatsPanel.vue`: Modified the `:title` attribute template literal on `.timeline-bar` to include multi-line token breakdown using `t('tokenStats.totalTokens')`, `t('tokenStats.inputTokens')`, and `t('tokenStats.outputTokens')`.
