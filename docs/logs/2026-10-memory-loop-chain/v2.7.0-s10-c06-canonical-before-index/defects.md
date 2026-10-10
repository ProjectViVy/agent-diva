# Defect record

## MEM-S10-01 — completed index reported as pending by mutation receipt lookup

**Contract affected:** A committed canonical mutation remains authoritative while its derived index is pending. Once the durable index outbox job completes, the scoped receipt reports the derived index as ready.

**Reproduction:** C06 killed the actual App child after the canonical transaction committed and before `applyIndexJob`. The restarted Mentle service replayed and deleted the pending outbox row. Public status returned `index_state=ok`, and public search returned the committed record, but `diva.cognitive.memory.receipt` still returned `index_status=pending`.

**Root cause:** `MutationStatus` initialized from the immutable receipt projection, whose index field defaults to pending. It only changed that state when an outbox row still existed; it did not interpret the no-row state that `completeIndexJob` uses after successful application.

**Fix:** `MutationStatus` now maps `sql.ErrNoRows` to `ready`; pending and failed rows retain their prior mappings. Other catalog query errors are returned instead of being silently ignored. Added `TestMutationStatusReportsReadyAfterOutboxCompletion`.

**Verification:** The focused unit test failed before the change and passed after it. C06 then passed 10/10 race samples; C03, C04, and C05 each passed one race-enabled regression sample. The full `mentle/facade` package passed.

**Commits:** Laputa `58ba973`; VIVY `1489825d`.
