# Acceptance

1. Run `agent-diva onboard` and confirm a config file plus workspace templates are created.
2. Run `agent-diva config path` and confirm config/runtime/workspace paths are shown.
3. Run `agent-diva status --json` and confirm stdout is valid JSON.
4. Run `agent-diva channels status --json` and confirm each enabled channel reports readiness and missing fields.
5. Run `agent-diva config doctor` against:
   - a healthy instance and confirm exit code `0`
   - an instance with missing provider credentials and confirm exit code `2`
6. Run `agent-diva cron add ...` with `--config <path>` and confirm the cron file is written under that instance runtime directory.
