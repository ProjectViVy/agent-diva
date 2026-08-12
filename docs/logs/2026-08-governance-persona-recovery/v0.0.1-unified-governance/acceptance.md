# Unified Memory Governance Acceptance

## Automated acceptance

- A tool-created reviewable Memory proposal appears in the shared approval ledger exactly once.
- Repeated GET list/detail/persona requests do not append approval events.
- A pending mapping whose request is absent from the shared ledger is restored with the same ID.
- A legacy private `allowed` receipt is not copied into the shared ledger.
- An approved proposal missing an authoritative receipt becomes `needs_attention`.

## Desktop interaction acceptance

1. In the current rebuilt GUI, create a Persona/Memory proposal and confirm that one Drawer
   approval is shown.
2. Approve it and confirm apply succeeds without `approval request not found`.
3. Restart and confirm no duplicate request is created.

The read-only real-workspace API smoke passed. These state-changing visual steps remain open for
the user's current manual-test session.
