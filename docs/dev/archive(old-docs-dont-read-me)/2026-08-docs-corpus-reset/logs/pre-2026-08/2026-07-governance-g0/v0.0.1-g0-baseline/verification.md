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
- `just test`: final integrated run passed; the earlier load-sensitive health benchmark failure did not recur.
- Focused health benchmark rerun before the final gate: passed in 0.20 seconds.
- Full GUI test after the Lucide integration: 54 files and 429 tests passed.
- GUI production build: passed with the existing large-chunk warning.
- Embedded-gateway lifecycle smoke: 11 tests passed.

## Known baseline observations

- The final integrated workspace gate is green.
- Vite reports the existing large-chunk warning.
- Rust reports the existing future-incompatibility notice for `imap-proto`.
- StepFun real-endpoint verification remains credential-blocked.
