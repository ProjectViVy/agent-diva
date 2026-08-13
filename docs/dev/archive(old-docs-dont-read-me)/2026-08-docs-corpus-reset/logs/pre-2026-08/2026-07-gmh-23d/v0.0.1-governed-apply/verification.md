# GMH-23D Governed Apply Verification

Passed:

- `cargo test -p agent-diva-laputa --test governed_apply` (3/3)
- `cargo check -p agent-diva-laputa`
- `cargo test -p agent-diva-manager --lib` (68/68)
- `cargo check -p agent-diva-manager`
- `cargo check -p agent-diva-gui`
- `pnpm test -- EvolutionView.test.ts` (16/16)
- `pnpm build`
- `just fmt-check`
- `just check`

`just test` compiled the workspace and progressed through the test binaries,
but the bounded runner closed its output pipe at 120 seconds; Cargo then
reported Windows broken-pipe error 232 while listing `agent-diva-files` tests.
This is an incomplete gate, not an observed assertion failure. Focused changed
crate tests above passed.

The isolated `CARGO_TARGET_DIR` Rust 1.80 attempt remains blocked before project
compilation by the existing lockfile resolving `base64ct 1.8.3`, whose
Edition-2024 manifest cannot be parsed by Cargo 1.80. This iteration did not
change the lockfile.

`cargo tree -p agent-diva-laputa` contains the existing `sqlx-sqlite 0.7.4`
stack and no `rusqlite`, Mentle, or LLVM dependency.

The focused fixtures cover digest rebinding/revocation, mandatory human
approval, invalid reusable grants for critical risk, receipt binding, typed
apply atomicity, and typed apply idempotency.

Real desktop acceptance is mandatory because this iteration changes visible
HITL behavior.
