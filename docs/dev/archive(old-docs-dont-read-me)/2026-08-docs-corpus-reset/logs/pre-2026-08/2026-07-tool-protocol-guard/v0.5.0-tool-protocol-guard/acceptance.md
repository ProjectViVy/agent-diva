# Acceptance

1. Run a tool-heavy task until its normal tool-iteration limit is reached.
2. If the final provider response contains DSML or XML tool protocol text, verify no raw protocol appears in streamed GUI text or the final response.
3. Verify the user instead sees a maximum-iteration status summary.
4. Verify the pending tool (for example `write_file`) is not executed.
5. Verify normal structured OpenAI-compatible tool calls continue to execute before the iteration limit.
