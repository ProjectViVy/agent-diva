# Acceptance

1. Submit aggregates with request IDs in non-sorted order.
2. Read them in bounded pages and observe stable lexicographic cursor order.
3. Replay at a later instant and observe derived expiry without loading every
   aggregate in one query.
4. Call lifecycle methods through `ApprovalCoordinator` with ledger CAS and
   idempotency semantics preserved.
