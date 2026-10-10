# Native ACTMEM failure and archive retry

An actual filesystem directory at an isolated profile's ACTMEM head path forces native read/projection failure. The actual primary completes and canonical source survives, while the evolution source prefix remains zero. The public ACTMEM read fails visibly. A real Service delete with a server-side deadline waits for projection and fails without deleting durable Session/user-message rows.

Restore the exact original head (or original absence): the existing worker retries the original native activity, releases the original source sequence, and preserves ingestion ID, canonical ID/revision/count without another model request. A subsequent real public session/delete archives the original fact/session. Test only; no backend substitute, SQL seed, permission simulation or production changes.

This is native ACTMEM I/O failure and archive preservation evidence, not the complete readonly/ENOSPC/index/crash-cut matrix. A failed delete intentionally keeps admission sealed, so public history reconciliation refuses; the original durable Store proves rows remain.
