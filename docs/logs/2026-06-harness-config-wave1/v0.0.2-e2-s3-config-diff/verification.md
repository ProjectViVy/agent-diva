# Verification

## Automated

- `cargo test -p agent-diva-core reload_plan -- --nocapture`
- `cargo test -p agent-diva-cli --test config_commands config_diff -- --nocapture`
- `cargo check -p agent-diva-core -p agent-diva-cli`

## Manual QA

- `target/debug/agent-diva.exe --config <base> config diff --new-config <candidate-hot> --format json`
  - Observed only `hot_reload_changes`, no `restart_required_changes`.
- `target/debug/agent-diva.exe --config <base> config diff --new-config <candidate-invalid> --format json`
  - Observed exit code `1` with serialization/parse error and no reload plan.
- `target/debug/agent-diva.exe --config <base> config diff --new-config <base> --format json`
  - Observed empty `hot_reload_changes` and `restart_required_changes`.
- `target/debug/agent-diva.exe --config <base> config diff --new-config <candidate-restart> --format json`
  - Observed restart-only classification for `gateway.port` and `providers.openai.api_key`.

## Evidence

- `.omo/ulw-loop/evidence/wave1-e2-s3-c001-hot-reload-diff.txt`
- `.omo/ulw-loop/evidence/wave1-e2-s3-c002-invalid-diff-candidate.txt`
- `.omo/ulw-loop/evidence/wave1-e2-s3-c003-noop-and-restart-diff.txt`
