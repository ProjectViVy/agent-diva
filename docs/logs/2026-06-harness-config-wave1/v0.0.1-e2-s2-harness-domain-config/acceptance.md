# Acceptance

1. Run `agent-diva --config <file> config show --format json` with explicit non-default `security`, `presence`, `heartbeat`, `audit`, `pii`, and `injection` values.
2. Confirm the JSON output includes those root sections with the configured values and exits successfully.
3. Run `agent-diva --config <file> status --json` with invalid harness-domain values such as `security.max_actions_per_hour = 0` or `presence.active_timeout_s = 0`.
4. Confirm the command exits non-zero and the validation error names the invalid harness field.
5. Run `agent-diva --config <legacy-file> config show --format json` on a legacy file without `config_version` or harness sections, then run it again on the same file.
6. Confirm the first run persists `config_version = 2` plus the defaulted harness sections and the second run succeeds from the migrated file without changing the legacy selections.
