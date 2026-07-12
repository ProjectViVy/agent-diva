# Tool Protocol Guard

## Summary

- Summary-only requests through the OpenAI-compatible provider now explicitly send `tool_choice: "none"` when no tools are supplied.
- Agent-loop streaming and final response paths suppress DSML/XML-like internal tool protocols.
- A suppressed protocol produces a deterministic maximum-iteration status summary and never executes the pending tool request.

## Impact

Protects GUI and channel users from raw provider tool-call templates while preserving normal structured tool execution.
