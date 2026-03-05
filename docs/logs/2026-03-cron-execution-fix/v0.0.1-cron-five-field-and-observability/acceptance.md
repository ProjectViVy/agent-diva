# Acceptance

1. Create a cron job with 5-field expression (example: `*/1 * * * *`) through GUI or API.
2. Confirm job `nextRunAtMs` is non-null in cron job list.
3. Keep gateway running until due time.
4. Observe gateway logs containing cron trigger/completion entries.
5. If channel is `api` with `to=gui`, verify GUI receives background callback message.
6. Manually run same job once and confirm run path still works.

7. Create a cron task with channel=gui and verify GUI still receives background callback (via mapped API cron stream).
8. Create a cron job and verify due trigger produces a fresh agent response (not raw payload echo), confirming it executed through agent loop.
9. Verify a cron-triggered run cannot create new cron jobs from inside the same turn (no recursive schedule growth).
10. Verify session history stores cron trigger input as `system` role, not `user`.
11. In cron-triggered turn, verify non-cron tools (e.g. web search/fetch) can still execute.
12. Verify cron tool call is rejected with guard error during cron-triggered turn.
13. From chat A, attempt removing job ID that belongs to chat B; verify tool returns context-mismatch error and job B remains.
14. Remove job ID belonging to current chat; verify job is removed and no further due triggers occur.
