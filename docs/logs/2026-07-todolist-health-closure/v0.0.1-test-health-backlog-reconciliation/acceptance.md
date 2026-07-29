# Acceptance

1. Start the embedded desktop gateway and poll `/api/health`; either `200 OK` or the established dependency-startup `502 Bad Gateway` proves the HTTP server is reachable.
2. Close the handle and confirm shutdown completes within the test timeout instead of hanging.
3. Trigger shutdown immediately after startup and confirm the background gateway thread exits cleanly.
4. Run the focused GUI, Core, and Agent regression commands listed in `verification.md`.
5. Inspect `TODOLIST.md` and confirm only reproducible current-baseline items remain actionable; superseded Plan/TODO review packets are explicitly historical.
