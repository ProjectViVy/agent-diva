# Acceptance

1. Approve a plan with Compact, Clear, or Retain and verify the returned execution initialization state is Pending.
2. Continue through the dedicated runtime command; verify no visible user bubble is created.
3. For Compact, confirm a failed quality gate blocks the implementation provider call; retry remains explicit.
4. Restart the backend and continue the same execution; verify its boundary and summary are restored.
5. Send a mismatched plan revision or execution identifier and confirm continuation is rejected.
6. Run the DeepSeek provider-set JSON test and confirm `deepseek-v4-pro` is stored without a `deepseek/` prefix.
