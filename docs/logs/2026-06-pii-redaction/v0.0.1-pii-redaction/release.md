# Release

- No special deployment step is required beyond shipping the updated binaries.
- This change is internal to core security and agent-loop message handling.

# Rollout Notes

- Monitor consumers of `AgentBusEvent::PiiRedacted` for the new `kind` values: `Email`, `Phone`, `ApiKey`, `CreditCard`, `SSN`, `IP`, `URL`, `Name`.
