# Verification

## Automated

- `cargo test -p agent-diva-core hot_reload -- --nocapture`
- `cargo test -p agent-diva-tooling module::tests -- --nocapture`
- `cargo test -p agent-diva-manager runtime::task_runtime::tests -- --nocapture`
- `cargo check -p agent-diva-core -p agent-diva-tooling -p agent-diva-manager`

## Manual QA

- `target/debug/agent-diva.exe --config <expanded-config> gateway run`
  - Edited only hot-reloadable fields and observed watcher reload logs plus `/api/health` returning `200 ok` before and after the edit.
- `target/debug/agent-diva.exe --config <expanded-config> gateway run`
  - Replaced the watched config with malformed JSON, observed parse-failure logs, and confirmed `/api/health` stayed `200 ok`; then restored a valid hot-only config and observed reload logs again.
- `target/debug/agent-diva.exe --config <expanded-config> gateway run`
  - Changed `gateway.port`, observed `Config change requires restart; skipping live reload`, kept `/api/health` at `200 ok`, then restored baseline plus a hot-only edit and observed a later hot reload succeed.

## Evidence

- `.omo/ulw-loop/evidence/wave1-e2-s4-c001-gateway-hot-reload.txt`
- `.omo/ulw-loop/evidence/wave1-e2-s4-c002-gateway-reload-failure-preserves-state.txt`
- `.omo/ulw-loop/evidence/wave1-e2-s4-c003-restart-required-watcher-regression.txt`
