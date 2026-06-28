# Acceptance

1. Start `agent-diva --config <expanded-config> gateway run`.
2. Confirm `GET /api/health` returns `200 ok`.
3. Edit only hot-reloadable fields in the watched config file.
4. Wait at least one watcher poll interval and confirm reload logs appear while `/api/health` stays `200 ok`.
5. Replace the watched config with malformed or validation-invalid content.
6. Wait at least one watcher poll interval and confirm reload-failure logs appear while `/api/health` still returns `200 ok`.
7. Restore a valid hot-only config edit and confirm a later reload succeeds without restarting the gateway.
8. Change only restart-required fields such as `gateway.port`.
9. Wait at least one watcher poll interval and confirm the gateway logs a restart-required warning, keeps serving health checks, and still accepts a later hot-only reload after the restart-only edit is removed.
