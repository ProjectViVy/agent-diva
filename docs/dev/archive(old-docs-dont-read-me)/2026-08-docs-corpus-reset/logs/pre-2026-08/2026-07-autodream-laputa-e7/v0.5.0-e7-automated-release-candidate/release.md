# Release

This commit records an automated release candidate, not a production
deployment.

Remaining release boundary:

1. perform the deferred G2D+ real-desktop scenarios on the new build;
2. preserve payload-free request/proposal/receipt/changelog/audit/rollback IDs;
3. stop and fix any duplicate execution, unrecoverable state, stale approval or
   silent authority fallback;
4. only then mark desktop acceptance complete.

No push is performed.
