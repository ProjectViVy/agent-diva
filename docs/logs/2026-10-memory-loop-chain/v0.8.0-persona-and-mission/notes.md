# Decisions

Reuse cached one-owner native domain/ledger with a per-run Mission wrapper. Optional BindForRun on existing App lazy domain keeps the current public Bundle/SDK contract and uses persisted run pins. Host effect/Lookup checks cover generic execution; native owner additionally checks inside the bounded mutation gate shared with human Persona writes/review decisions. Client lifetime read lock precedes authority gate consistently. No inference lock.

Zero is unassigned, not wildcard. Admission checks newly resolved pins instead of construction-time pins. Old-run human receipts remain queryable; workflow stale-pin recovery stays refused, with no reset/retry/state forgery.

The foreign-admission unit now uses a fresh controller: effectful cancellation of the original window may honestly remain fenced, so it cannot serve as a guaranteed new-admission fixture. Original cancel remains issued; no fence is erased.

Persona model derives proposal/actual base revision exclusively from actual request. Mission response gate is in-process test infrastructure released before fixture shutdown; no production provider configuration or new remote recovery command.
