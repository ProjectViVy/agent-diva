# Story 5.2 Mentle Governance Exclusion Acceptance

## Acceptance Steps

1. Run AutoDream output and report guardrail tests.
2. Confirm AutoDream writes artifacts, proposals, and reports without creating `memory/palace.db` or `.mentle`.
3. Run Laputa governance guardrails after unrelated test compile blockers are resolved.
4. Confirm proposal creation, apply, audit, rollback, and event paths stay file-first and do not require Mentle runtime state.
5. Run agent context Mentle tests.
6. Confirm default prompt assembly does not inject Mentle recall or `memtle_*` routing unless explicitly enabled outside governance.

## Current Result

Partially accepted at guardrail level. Final acceptance is blocked until full required validation passes.
