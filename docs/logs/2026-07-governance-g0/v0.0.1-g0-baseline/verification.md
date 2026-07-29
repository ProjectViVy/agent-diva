# G0 Verification

## Focused evidence

- `cargo test -p agent-diva-agent characterization_normal_turn_emits_final_response_without_error`
- `cargo test -p agent-diva-manager g0_`
- `cargo test -p agent-diva-core concurrent_create_and`
- `cargo test -p agent-diva-agent --features mentle --lib test_register_default_tools_rebuild_keeps_active_mentle_prompt`
- `pnpm test -- src/api/capabilities.test.ts`
- `pnpm build`

All focused tests passed on 2026-07-29:

- Agent characterization: 1 passed.
- Manager G0 contracts: 2 passed.
- Todo concurrency: 2 passed.
- Mentle prompt rebuild regression: 1 passed.
- Embedded gateway health/shutdown smoke: 1 passed.
- GUI suite: 54 files and 429 tests passed.
- GUI production build passed with the existing large-chunk warning.

## Gate commands

Final results:

- `just fmt-check`: passed.
- `just check`: passed.
- `just test`: failed only at `handlers::health::tests::health_benchmark_ci_gate_stays_within_budget`; the run measured 5.1021421 seconds for 500 requests.
- Focused health benchmark rerun: passed in 0.20 seconds, confirming load sensitivity rather than a deterministic functional failure.
- Full GUI `pnpm test`: passed, 54 files and 429 tests.
- GUI production `pnpm build`: passed with the existing large-chunk warning.
- `cargo test -p agent-diva-gui embedded_gateway_serves_health_endpoint -- --nocapture`: passed.

## Known baseline observations

- Vite reports existing large-chunk warnings; this iteration does not change bundle composition.
- Rust reports the existing future-incompatibility notice for `imap-proto`.
- StepFun real-endpoint verification remains credential-blocked and stays open in `TODOLIST.md`.
- The Manager health benchmark is a new G0 blocker in `TODOLIST.md`; G1 must not start while the full gate is red.
