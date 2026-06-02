# Acceptance

## Product Acceptance

- Phase A-PRE clearly gates Phase A (`Plan + TodoList`).
- Backend session durability risks are captured, not just GUI cache risks.
- Confirmed P0 findings are separated from lower-priority or unsupported
  findings.
- The document makes backend session history the authoritative source when
  available.
- GUI cache is explicitly a fallback path, not a competing truth source.

## Engineering Acceptance

- Implementation phases are actionable and ordered:
  1. confirmed call-site audit;
  2. durable inbound user-message save;
  3. atomic save and load-failure safety;
  4. consolidation ordering;
  5. backend-first GUI loading;
  6. GUI cache invalidation and optimistic reconciliation;
  7. regression tests.
- The document lists relevant backend and GUI code areas.
- Validation expectations include backend tests, GUI tests, and manual smoke
  checks.
