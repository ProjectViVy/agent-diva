# Acceptance

1. Open the GUI with an empty session list and verify the right sidebar shows session entries rather than delayed history placeholders.
2. Start a new session without sending a message and confirm a draft entry appears immediately.
3. Send the first user message and confirm the same session entry updates its preview, timestamp, and message count without waiting for `refreshSessions()`.
4. Wait for the first assistant response and confirm the session title changes from the draft/fallback title to the generated title when generation succeeds.
5. Rename the session manually, restart the GUI, and confirm the manual title persists without being overwritten by automatic generation.
