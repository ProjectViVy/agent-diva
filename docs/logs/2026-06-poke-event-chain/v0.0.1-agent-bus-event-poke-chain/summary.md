# Summary

- Implemented fine-grained `AgentBusEvent` with 8 variants in the core bus layer.
- Kept existing streaming `AgentEvent` broadcast compatible by splitting it into `AgentEventEnvelope`.
- Wired event emission into agent loop decision/tool paths and heartbeat execution paths.
- Added tests for bus event subscription, presence transition emission, heartbeat emission, and agent-loop audit event emission.
