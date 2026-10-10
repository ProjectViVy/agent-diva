# S10 developer checkpoint: C06 canonical commit before derived-index completion

## Result

C06 now has ten independent race-enabled process-crash samples. Each sample stopped the owned App process after Mentle committed the canonical memory, atomic mutation receipt, and pending index outbox row, but before the index job was claimed. A new process replayed the outbox job. The original canonical target and revision remained unchanged, the public memory search found the record after restart, index health returned to `ok`, and the public atomic receipt reported `index_status=ready`.

The interrupted DIVA workflow remained `recovery_required` with `unknown_outcome` and watermark 0. It did not issue another model request or create another canonical memory. Every sample retained exactly two canonical memories: the captured source and one effect.

The run first exposed `MEM-S10-01`: after a successful index job was deleted from the outbox, `MutationStatus` still returned the receipt's default `pending` index state. The focused test failed on that state even though the index-health view was `ok` and search could retrieve the canonical record. Laputa now reports `ready` when no outbox row remains and returns catalog read errors rather than silently hiding them.

## Verification

- C06 crash cut: race-enabled `go test -count=10`; 10 pass, 0 fail, 0 skip. JSON evidence: `raw/c06-canonical-before-index-race10.jsonl`.
- C03, C04, C05 after the receipt fix: one race-enabled sample each; all pass, 0 skip. Evidence: `raw/c03-c05-after-c06-fix-race1.jsonl`; observed exit code: `raw/c03-c05-after-c06-fix-exit.txt`.
- Laputa `mentle/facade`: full package test passed. Evidence: `raw/c06-laputa-facade-tests.txt`.
- VIVY production App build and `git diff --check`: passed. Evidence: `raw/c06-vivy-build.txt`.
- Race-log event verification: 10 test passes, no fail/skip, one successful package result. Evidence: `raw/c06-race10-validation.txt`.

The C06-only pre-index hook is compiled through the diagnostic Go overlay; it adds no product control surface. Both projects run the real App, Garden, and Mentle code against test-local SQLite and a loopback model fixture. The test waits for the actual SQLite organism-lease expiry after killing the child.

Commits: VIVY `1489825d`; Laputa `58ba973`.

## Limits

This is developer evidence, not formal S10 acceptance or a sealed candidate. S10 remains Planned. C01–C05 have separate developer checkpoints; unknown and partial-effect handling still needs the remaining S10 acceptance work, and the same-candidate/source-sealing gates remain open. No push, merge, or release was performed.
