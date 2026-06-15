# Story 6.4 Acceptance

1. Run `cargo test -p agent-diva-laputa governance_proof_loop`.
2. Confirm the test creates a proposal through `LaputaService`, approves it, applies it, reads snapshot and changelog data, rolls the changelog back, and leaves the proposal state as `reverted`.
3. Confirm the proof loop observes both `ProposalApplied` and `RollbackApplied` audit records and both `apply` and `rollback` changelog events.
4. Run `cargo test -p agent-diva-laputa governance_direct_write_guard_only_allows_laputa_owned_authority_paths`.
5. Confirm the guard fails if an EVO-DIVA runtime crate adds a direct authority-path write outside Laputa-owned boundaries.
6. Treat these two tests as the minimum release gate before Epic 5 prompt/report consumption is considered safe.
