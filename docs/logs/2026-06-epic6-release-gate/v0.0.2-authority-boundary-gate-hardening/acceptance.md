# Story 6.5 Boundary Gate Hardening Acceptance

1. Run the authority boundary guard as a standalone Laputa test and confirm it fails on forbidden authority reads/writes.
2. Run `governance_proof_loop` and confirm it invokes the same authority boundary guard before passing.
3. Run `just epic6-release-gate` and confirm it includes `authority_boundaries` before the remaining Epic 6 checks.
4. Confirm the direct-write guard no longer depends on a broad production-file allowlist.
