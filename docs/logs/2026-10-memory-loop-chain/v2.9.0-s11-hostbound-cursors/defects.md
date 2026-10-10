# S11 defect and blocker record

## MEM-S11-01 — continuation cursor and stable paging

**Status:** Resolved for developer verification.

**Original evidence:** v2.8 recorded that `Service.SearchCards` ignored `CardQuery.Cursor` and returned no `NextCursor`, making valid foreign-cursor replay impossible.

**Resolution:** Mentle search now consumes bounded offset cursors and emits continuation cursors. Garden encrypts and authenticates cursor contents with a per-adapter AES-GCM key and request context. Stable score tie-breaking prevents page order from shifting between requests. Unit tests cover page progression, tampering, query binding, adapter binding, and score ties. The VIVY actual-App test obtains a valid A cursor and verifies B rejects it with no data or continuation returned.

## MEM-S11-02 — race-enabled hostile-memory recall

**Status:** Open; cause not established.

**Contract affected:** A hostile ordinary memory must reach the actual model request as untrusted user data, without gaining authority.

**Evidence:** The ordinary actual-App test passes and confirms the synthetic instruction-shaped string appears in user data and not in system content. Two race runs failed with empty candidates and `material read failed`; the final timing run took 38.14 seconds. A test-only observer measured the underlying provider calls at 3,542 ms and 2,342 ms, both longer than ContextHost's 750 ms source timeout. The observer measures provider completion, which may occur after ContextHost has timed out; this supports a timeout/cancellation interaction but does not establish why Garden returned the generic read error. Raw outputs: `raw/s11-injection-race-final.jsonl` and `raw/s11-injection-race-timing.jsonl`; both package exits: 1.

**Impact:** Injection-boundary behavior is verified in ordinary mode, not yet under race.

**Next action:** Preserve the underlying native read error through a test-only diagnostic path or collect source-stage timings, then determine whether the issue is instrumentation, timeout configuration, or a real read-path defect before changing production behavior.

## Remaining S11 work

The backend disconnect/read-only/disk-full/index-fault recovery matrix has not started. Workspace A/B authorization remains untested and is not represented by the current fixed `diva` cognitive profile. S09 remains Planned; do not promote S11 from Planned.
