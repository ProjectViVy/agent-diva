# GMH-23D Governed Apply Verification

Passed:

- `cargo test -p agent-diva-laputa --test governed_apply` (3/3)
- `cargo check -p agent-diva-laputa`
- `cargo test -p agent-diva-manager --lib` (68/68)
- `cargo check -p agent-diva-manager`
- `cargo check -p agent-diva-gui`
- `pnpm test -- EvolutionView.test.ts` (16/16)
- `pnpm build`

The focused fixtures cover digest rebinding/revocation, mandatory human
approval, invalid reusable grants for critical risk, receipt binding, typed
apply atomicity, and typed apply idempotency.

Workspace-wide gates and Rust 1.80 isolated-target validation are recorded at
final handoff. Real desktop acceptance is mandatory because this iteration
changes visible HITL behavior.
