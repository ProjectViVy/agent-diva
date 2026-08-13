# Acceptance

Acceptance checks:
- New session save creates readable JSONL content.
- Existing session replacement leaves valid final content with the latest messages.
- Injected failure before rename leaves the previous session file readable and unchanged.
- Failed save cleans up temporary scratch files.

Result: accepted by automated `agent-diva-core` session tests.
