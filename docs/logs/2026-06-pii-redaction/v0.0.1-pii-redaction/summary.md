# Summary

- Added `agent-diva-core/src/security/pii.rs` with 8-category PII detection and `[REDACTED:<Kind>]` replacement output.
- Integrated PII redaction into agent-loop inbound message text, attachment-derived text parts, streamed deltas, tool results, and final outbound content/reasoning.
- Emitted `AgentBusEvent::PiiRedacted` per detected PII kind and updated runtime-log assertions to match the new redaction format.

# Impact

- User and tool text that matches email, phone, API key, credit card, SSN, IP, URL, or contextual name patterns is now automatically redacted before it is persisted or returned.
- Audit consumers can distinguish redaction categories through `PiiRedacted.kind` instead of a generic secret count.
