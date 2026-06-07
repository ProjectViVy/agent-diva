# v0.0.3 Debug Gateway Bundle Acceptance

## Acceptance Steps

- Run `agent-diva gateway run --debug` and confirm startup prints the run id, debug directory, and raw-output warning.
- Send at least one message through the foreground gateway and confirm the debug run directory contains `manifest.json`, `events.jsonl`, `raw.jsonl`, and `gateway.log`.
- Confirm `raw.jsonl` includes full provider/tool/MCP boundary payloads available at the current code boundary.
- Run `agent-diva gateway bundle` and confirm it creates `debug-bundles/debug-bundle-<run-id>.zip`.
- Confirm ordinary non-debug gateway startup still uses the existing logging behavior.
