# Acceptance

1. Start the local Manager and desktop GUI with Sandbox approval policy set to `on-failure`.
2. In a GUI chat, request a harmless command that the sandbox deterministically refuses.
3. Confirm the card shows the full command, cwd, reason, source session, and remaining time.
4. Reject it and confirm the command does not execute.
5. Trigger it again, choose **Allow once**, and confirm it executes exactly once.
6. Trigger the same command and cwd again, choose **Allow for session**, and confirm the exact match is reused only in that session.
7. Trigger an approval in another GUI session. Confirm the current chat offers navigation but no decision buttons; switch to the source session and resolve it there.
8. Restart the approval event connection or the GUI while a request is pending and confirm the request returns after pending reconciliation.
9. Stop the source chat and confirm its approval disappears and cannot be submitted.

Expected: stale or duplicate decisions do not execute commands; global or cross-session allow rules are never created.
