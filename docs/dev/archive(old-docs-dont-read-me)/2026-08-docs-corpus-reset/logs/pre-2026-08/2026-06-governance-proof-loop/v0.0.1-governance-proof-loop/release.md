# Story 6.4 Release

No deployment was performed in this iteration.

This story adds release-gate tests only. The minimum gate before Epic 5 prompt/report consumption is considered safe is:

1. `cargo test -p agent-diva-laputa governance_proof_loop`
2. `cargo test -p agent-diva-laputa governance_direct_write_guard_only_allows_laputa_owned_authority_paths`

Broader workspace promotion still depends on resolving unrelated blockers already recorded in `TODOLIST.md`.
