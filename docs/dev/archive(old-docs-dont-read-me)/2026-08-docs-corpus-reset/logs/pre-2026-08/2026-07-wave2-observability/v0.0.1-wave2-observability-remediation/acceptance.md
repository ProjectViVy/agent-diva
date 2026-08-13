# Acceptance

1. Start a gateway runtime for a workspace and confirm the workspace now creates `.agent-diva/audit/`.
2. Trigger audit-producing actions such as a tool lookup failure, a cron run, and a manager chat/provider call.
3. Query `GET /api/logs` twice with `limit=1`; confirm the second request with `next_cursor` advances to the next event instead of repeating the first page.
4. Query `GET /api/audit/events` on a mixed `gateway.log` file and confirm non-`audit` targets are excluded.
5. Review `TODOLIST.md` and confirm the remaining Wave 2 residual risks are explicitly recorded.
