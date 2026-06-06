# Acceptance

1. Configure `tools.exec.timeout` with a positive value.
2. Trigger any slow or hanging built-in tool that does not have its own internal timeout handling.
3. Verify the tool call returns a timeout error instead of hanging the agent loop indefinitely.
4. Trigger a normal fast tool call and verify behavior is unchanged.
5. Set `tools.exec.timeout = 0` in config validation scope and verify the configuration is rejected.
