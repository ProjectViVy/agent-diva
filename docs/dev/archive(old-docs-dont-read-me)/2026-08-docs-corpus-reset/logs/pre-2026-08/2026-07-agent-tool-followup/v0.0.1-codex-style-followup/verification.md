# Verification

```text
cargo test -p agent-diva-agent synthesize_tool_turn
cargo test -p agent-diva-agent resolve_empty_final
cargo test -p agent-diva-agent truncate_for_tool_summary
```

| Check | Result |
| --- | --- |
| synthesize_tool_turn_summary_lists_recent_tools | pass |
| resolve_empty_final_prefers_plan_approval_over_tools | pass |
| resolve_empty_final_uses_tool_synthesis | pass |
| resolve_empty_final_generic_when_no_tools | pass |
| truncate_for_tool_summary_limits_chars | pass |

Manual (deferred): execute approved plan ending with `exec` → expect model summary or Chinese tool synthesis; never the English placeholder.
