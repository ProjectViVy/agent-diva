# Acceptance

1. Prepare a valid baseline config file and a hot-change candidate file.
2. Run `agent-diva --config <base> config diff --new-config <candidate-hot> --format json`.
3. Confirm the JSON only contains `hot_reload_changes` for the changed harness/logging/tool paths.
4. Run `agent-diva --config <base> config diff --new-config <candidate-invalid> --format json`.
5. Confirm the command exits non-zero with a parse or validation error instead of a reload plan.
6. Run `agent-diva --config <base> config diff --new-config <base> --format json`.
7. Confirm both change arrays are empty for identical configs.
8. Run `agent-diva --config <base> config diff --new-config <candidate-restart> --format json`.
9. Confirm restart-only changes are reported under `restart_required_changes` and not misclassified as hot reload.
