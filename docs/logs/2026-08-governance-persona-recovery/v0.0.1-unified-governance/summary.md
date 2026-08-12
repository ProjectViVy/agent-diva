# Unified Memory Governance Summary

## Outcome

Production Memory/Laputa writes now use the Manager-owned `ApprovalCoordinator` as the single
approval authority. The Agent no longer creates approval events in the private
`.laputa/governance.sqlite3` ledger while the Manager reads `.laputa/governance.db`.

Startup reconciliation restores reviewable requests missing from the shared ledger with their
stable request IDs. It never imports a legacy allow receipt. An approved proposal without an
authoritative receipt is moved to `needs_attention` for explicit recovery.

Laputa list, detail, and Persona workspace reads now project existing governance state without
submitting new approval events. A failed projection is isolated to that proposal.

Commits:

- `78e2bcf5 fix: unify memory governance authority`.
- `ab4705e4 fix: register autodream proposal approvals`.

The second commit places AutoDream approval registration at its proposal-creation boundary before
the run becomes terminal, preserving side-effect-free GET behavior.
