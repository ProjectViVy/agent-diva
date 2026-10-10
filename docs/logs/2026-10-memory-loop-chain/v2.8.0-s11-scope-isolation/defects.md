# S11 defect and blocker record

## MEM-S11-01 — memory card search does not produce continuation cursors

**Contract affected:** diva.cognitive.memory.search accepts a cursor and returns a next_cursor; S11 requires checking that a cursor from another scope/host cannot expose content.

**Evidence:** The first real-App probe requested limit=1; the result contained one card and an empty next_cursor. Source inspection of laputa/mentle/facade/cards.go confirms CardQuery.Cursor is not consumed and Service.SearchCards returns no NextCursor. The Garden adapter's outer cursor validation can reject forged cursors, but it has no valid inner continuation token to wrap.

**Impact:** A valid Profile A cursor cannot be minted, so cross-host replay is not verified. The forged-cursor denial is covered, but it does not satisfy the foreign-cursor case.

**Next action:** Define bounded pagination semantics and host binding for the cursor, add Mentle/Garden tests, then repeat the actual-App Profile A/B check. No cursor implementation was changed in this checkpoint.

## MEM-S11-02 — race-enabled recall could not read native memory evidence

**Contract affected:** The injected hostile-memory case must reach the actual model request as untrusted user data.

**Evidence:** The race-enabled actual-App run recorded material read failed for both source queries. The ordinary run of the same case passed and confirmed the hostile excerpt in the model request's user context. The scope-race run also completed but did not invoke the memory source.

**Root cause:** Not established. The ContextHost source timeout defaults to 750 ms, which may interact with race instrumentation, but that is a hypothesis rather than a confirmed cause.

**Impact:** The injection boundary is verified in the ordinary run, not under race. Do not count the race failure as a product fix or as passing injection evidence.

**Next action:** Preserve the underlying native read error in a test-only diagnostic path or reproduce with source-stage timing, then determine whether the issue is test instrumentation, timeout configuration, or a real read-path defect before changing production behavior.

## Remaining S11 work

The backend disconnect/read-only/disk-full/index-fault recovery matrix has not started. Workspace A/B behavior and valid foreign-cursor replay are also open. S09 is a direct prerequisite and remains Planned; do not promote S11 from Planned.
