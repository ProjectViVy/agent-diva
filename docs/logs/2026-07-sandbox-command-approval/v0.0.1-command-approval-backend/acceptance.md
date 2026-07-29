# Acceptance

1. Start an agent-mode command call that succeeds inside the sandbox; confirm no approval request is created.
2. Trigger a recoverable sandbox denial and subscribe to `/api/command-approvals/events`; confirm `command_approval_requested` includes command, cwd, reason, scope, creation time, and timeout.
3. Resolve the request through `POST /api/command-approvals/:approval_id` with `approve_once`; confirm the suspended call resumes exactly once.
4. Repeat with `approve_session`; confirm only the same session, exact command, and cwd reuse the decision.
5. Reject, stop the chat, or allow the request to expire; confirm the command does not execute and the pending query is empty.
6. Enter Plan mode and confirm `exec` is absent from the tool registry.
