# Acceptance

1. Start the manager runtime and verify `GET /api/health` returns `200` only after runtime initialization completes, and `503` for a bare handler/test state with no cron-ready signal.
2. Trigger a direct CLI chat or TUI request and confirm provider audit records now appear in the workspace audit JSONL stream.
3. Upload invalid skill ZIPs (malformed archive, missing `SKILL.md`, invalid content, blocked/quarantined content) and confirm the audit stream records stable `skill_name` values.
4. Open the GUI audit page and verify the structured/raw tabs, empty states, aria labels, and raw-log copy feedback resolve through i18n keys with no hardcoded English fallback.
