# Acceptance

1. Send a message containing one or more supported PII values to the agent.
2. Confirm the stored/returned text replaces each match with `[REDACTED:<Kind>]`.
3. Trigger a tool response containing a supported PII value and confirm the tool result summary is redacted.
4. Subscribe to bus events or audit logs and confirm `AgentBusEvent::PiiRedacted` is emitted with the detected category and count.
