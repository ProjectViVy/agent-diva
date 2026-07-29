# GMH-12 Acceptance

1. Confirm the ledger table is append-only and current state is replay-derived.
2. Confirm stored records contain neither domain payload nor evidence excerpt.
3. Confirm expected-version CAS and durable idempotency survive restart.
4. Confirm expired, denied, revoked, consumed, stale, or tampered approvals
   cannot authorize.
5. Confirm approve-once is consumed exactly once.
6. Confirm legacy Plan and Sandbox contracts serialize unchanged and adapters
   perform no persistence or runtime registration.
7. Confirm focused and complete workspace gates pass before delivery.
