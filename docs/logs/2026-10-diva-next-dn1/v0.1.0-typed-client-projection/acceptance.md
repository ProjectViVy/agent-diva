# DN-1 v0.1.0 — Acceptance

Owner review focus (p0-design.md five adverse cases):

1. Duplicate/late events → `vivy-session.test.ts` dedup/buffer cases.
2. Window close during approval → pending interactions rebuilt from
   `approval/list` + `question/list` snapshots on reattach.
3. Cancel racing completion → terminal phase map; cancel error preserved.
4. Timeout after accepted mutation → `VivyCallError.unknownOutcome`,
   never auto-retried; reconcile by reads.
5. Shutdown while FFI active → covered by DN-5 lifecycle tests; client
   `close()` detaches listeners only.

Live streamed answer + pending-approval window reopen still requires a
provider (DN-2 acceptance); not claimed here.
