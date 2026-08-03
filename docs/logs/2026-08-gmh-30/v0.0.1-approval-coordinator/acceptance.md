# Acceptance

1. Plan, Sandbox, and Memory requests can enter one coordinator contract.
2. Safe allows and hard denials do not create approval ledger records.
3. Human-required requests become durable Pending states idempotently.
4. Decision retries are idempotent and version conflicts remain fail closed.
5. Serialized ledger events contain no domain payload.
6. Receipt consumption and transport/UI migration remain explicitly deferred.
