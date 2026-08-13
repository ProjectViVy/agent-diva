# Acceptance

1. Start Manager and the desktop GUI with approval policy `on-failure`.
2. Trigger an eligible harmless command such as `git --version`; confirm the approval card shows the validated global rule.
3. Choose **Add to global safe list** and confirm the command executes once.
4. Trigger the exact command in another session and confirm no approval card is created.
5. Open Sandbox settings and confirm the rule shows its pattern, source, creation time, state, and justification.
6. Disable the rule and confirm the next exact command requires approval again; re-enable it and confirm reuse resumes.
7. Delete the rule and confirm it disappears and no longer applies.
8. Trigger `cargo build`, an interpreter command, a chained/redirection command, and a protected-path operation; confirm none offers global approval.
9. Submit a stale revision through the API and confirm it returns 409 without changing the rule.

Expected: only exact, backend-validated token patterns persist; session approval is never silently promoted to global scope.
