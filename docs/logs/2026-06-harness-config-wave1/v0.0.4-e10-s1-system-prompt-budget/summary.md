# Summary

- Added explicit system-prompt budget measurement on the real request path before provider dispatch.
- Wrote a structured runtime trace event named `system_prompt_budget_measured` with measured tokens, reserve tokens, overflow status, and prompt character count.
- Added an overflow warning when the rendered system prompt exceeds `agents.defaults.context_budget_reserve_tokens`.
- Kept the existing whole-request compaction and overflow-retry path unchanged, then covered the coexistence with targeted agent-loop tests.
