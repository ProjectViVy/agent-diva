# Story 6.5 Boundary Gate Hardening

## Change

Hardened the Epic 6 release gate so the authority boundary guard is a shared hard gate instead of a file-specific scan with broad exemptions.

## Impact

- Added a shared `authority_boundary_guard` test helper in `agent-diva-laputa`.
- Removed the broad production-file allowlist from the direct-write guard path.
- Made `governance_proof_loop` call the authority boundary guard directly.
- Added `authority_boundaries` to `just epic6-release-gate`.

## Files

- `agent-diva-laputa/tests/authority_boundary_guard.rs`
- `agent-diva-laputa/tests/authority_boundaries.rs`
- `agent-diva-laputa/tests/direct_write_guard.rs`
- `agent-diva-laputa/tests/governance_proof_loop.rs`
- `justfile`
- `TODOLIST.md`
