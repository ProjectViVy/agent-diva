# G0 Verification

## Focused evidence

The following commands passed on Windows / Rust stable on 2026-07-29:

- `cargo test -p agent-diva-agent characterization_normal_turn_emits_final_response_without_error` — 1 passed.
- `cargo test -p agent-diva-manager g0_` — 2 passed.
- `cargo test -p agent-diva-core test_concurrent_create_and` — 2 passed.
- `cargo test -p agent-diva-laputa renders_applied_laputa_sections_and_excludes_unapplied_proposals` — 1 passed.
- `cargo test -p agent-diva-manager report_projection_uses_canonical_store_for_revision_approval` — 1 passed.
- `cargo test -p agent-diva-sandbox approval_coordinator` — 5 passed.
- `cargo test -p agent-diva-sandbox test_on_failure_retry_requires_cached_approval` — 1 passed.
- `cargo test -p agent-diva-agent test_tool_assembly_enqueue_background_task_with_run_store` — 1 passed.
- `cargo test -p agent-diva-agent subagent_run_handler::tests` — 3 passed.
- `pnpm test -- src/api/capabilities.test.ts` — passed.
- `pnpm build` — passed.

The independent Mentle lane owns the corrected prompt-rebuild contract. G0 does
not preserve the retired `L2 Palace Memory` prompt route.

## Gate commands

- `just fmt-check`: passed.
- `just check`: passed.
- `just test`: failed only at `handlers::health::tests::health_benchmark_ci_gate_stays_within_budget`; the full run measured 5.1021421 seconds for 500 requests.
- Focused health benchmark rerun: passed in 0.20 seconds, demonstrating load sensitivity.
- Full GUI test: 54 files and 429 tests passed before the Lucide integration.
- GUI production build: passed with the existing large-chunk warning.
- Embedded-gateway lifecycle smoke: 11 tests passed.

## Known baseline observations

- G1 is blocked until the full workspace gate is green.
- Vite reports the existing large-chunk warning.
- Rust reports the existing future-incompatibility notice for `imap-proto`.
- StepFun real-endpoint verification remains credential-blocked.
