# S11 developer checkpoint: scope isolation and memory injection

## Result

VIVY now has two actual-App integration cases for the S11 scope and untrusted-memory boundary. The ordinary diagnostic-overlay run passed both cases with zero skips. Profile A and Profile B used separate config/data roots and separate real App/Garden/Mentle compositions. Profile A created two memories; Profile B's search and observed recall returned no Profile A content. The host rejected A's session, card, receipt, stale revision, and a forged cursor without returning protected data.

The injection case wrote a synthetic hostile instruction through the ordinary capture and reflection path, then retrieved it through the actual Mentle ContextSource. The captured model request placed the instruction in user data and not in the system message. Mission content and revision, trigger-policy state, and the effect-receipt list remained unchanged after the recall turn.

VIVY test commit: 382525d9 (test(app): verify memory scope isolation and injection boundaries).

## Verification

- Ordinary combined run: TestMemoryLoopScopeAndHostBinding and TestMemoryLoopMemoryInjection; 2 pass, 0 fail, 0 skip; package exit 0. Raw output: raw/s11-scope-and-injection-final2.jsonl; observed exit: raw/s11-scope-and-injection-final2-exit.txt.
- Scope race run: 1 pass, 0 fail, 0 skip; package exit 0. It confirmed no Profile A fact in Profile B's model input, but the ContextHost memory source was not invoked in that run. Raw output: raw/s11-scope-isolation-race8.jsonl; exit: raw/s11-scope-isolation-race8-exit.txt.
- Injection race run: failed; package exit 1. The real recall trace returned material read failed for both source queries, so the hostile memory did not reach the model request in that run. Raw output: raw/s11-memory-injection-race1.jsonl; exit: raw/s11-memory-injection-race1-exit.txt.
- First cursor probe: failed because a page cursor was absent. This exposed a real implementation gap rather than a passing isolation result. Raw output: raw/s11-scope-isolation-race1.jsonl; exit: raw/s11-scope-isolation-race1-exit.txt.

The test uses a diagnostic Go overlay, real local SQLite-backed App/Garden/Mentle code, and a loopback synthetic model. It is developer evidence, not a sealed candidate or real-model evaluation.

## Limits

TestMemoryLoopBackendRecovery has not been implemented or run. The S09 prerequisite remains Planned, workspace-scope behavior is not exercised, and the product currently fixes the cognitive profile identity to diva; these tests isolate storage roots while keeping that same profile identity.

Mentle card search accepts a cursor field but does not return a next cursor. A valid Profile A cursor therefore cannot be obtained for a cross-host replay test; the app test only verifies rejection of a forged cursor. See defects.md. S11 remains Planned, and no formal acceptance or candidate gate is claimed. No push, merge, or release was performed.
