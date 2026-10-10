# S11 developer checkpoint: host-bound cursors and stable search pages

## Result

The previous S11 probe could not obtain a real continuation cursor because Mentle search ignored its cursor input and never returned `next_cursor`. Mentle now emits bounded offset cursors (maximum 1,000 canonical cards); Garden wraps the inner cursor and visible-item offset in a per-adapter AES-GCM token. The token authenticates the bound scope, destination, authorized read scopes, collection, and query. Its offset and inner cursor are encrypted, and a token from another adapter instance or a changed query is rejected.

A race run exposed a second issue: equal-score lexical and vector results could change order between page requests, causing repeated cards. Mentle now breaks equal-score ties by stable ID before assigning search ranks and after score fusion. Regression tests cover repeated ties, page progression, tampered encrypted cursors, changed queries, and cross-adapter replay.

The VIVY real-App scope test now obtains a valid Profile A continuation cursor and verifies that Profile B rejects it with `invalid_scope`, returning no items or next cursor. The ordinary scope and injection tests both pass. The instruction-shaped injection string is generated for this test and stored only in its temporary fixture root; a loopback model stub verifies message framing. No live user memory or model is involved. The scope race test passes but did not invoke the recall source in this sample. A final injection race run is recorded separately below and remains diagnostic evidence only.

## Verification

- Mentle facade and hybrid search: normal and race tests pass. Raw outputs: `raw/s11-mentle-normal.txt`, `raw/s11-mentle-race.txt`.
- Garden Mentle adapter: normal and race tests pass. Raw outputs: `raw/s11-garden-normal.txt`, `raw/s11-garden-race.txt`.
- VIVY actual-App combined run: `TestMemoryLoopScopeAndHostBinding` and `TestMemoryLoopMemoryInjection` pass; 2 pass, 0 fail, 0 skip, exit 0. Raw output: `raw/s11-scope-and-injection-cursor-final.jsonl`.
- VIVY injection-only ordinary rerun after adding the timing observer: 1 pass, 0 fail, 0 skip, exit 0. Raw output: `raw/s11-injection-normal-timing.jsonl`.
- VIVY scope race: 1 pass, 0 fail, 0 skip, exit 0. It exercised valid foreign-cursor rejection and confirmed no Profile A content in Profile B's model input, but the ContextHost recall source was not invoked. Raw output: `raw/s11-scope-isolation-cursor-race-final.jsonl`.
- VIVY injection race: two runs failed with `material read failed`; a test-only timing probe measured provider call wall times of 3.542s and 2.342s in the last run. See `raw/s11-injection-race-final.jsonl`, `raw/s11-injection-race-timing.jsonl`, and `defects.md`.

All integration tests use separate test-local SQLite roots, a loopback synthetic model, and the Go diagnostic overlay. They are developer checks, not a sealed candidate or real-model evaluation.

## Limits

S11 remains Planned. Its direct prerequisite S09 is still Planned; backend disconnect/read-only/disk-full/index-fault recovery and workspace A/B authorization have not been tested. The product currently fixes the cognitive profile identity to `diva`, so storage-root isolation does not establish workspace isolation. Race-enabled hostile-memory injection did not yet establish a successful source-to-model path; see the open defect. No formal acceptance, merge, push, or release is claimed.
