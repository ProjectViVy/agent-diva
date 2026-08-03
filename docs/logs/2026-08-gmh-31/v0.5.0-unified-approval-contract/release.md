# Release and rollback

This story changes source contracts only. It does not deploy, push, start the desktop application, access a real profile, or call a provider.

The focused commit is the rollback unit. Reverting it removes the unified `/api/approvals` routes, durable SSE projection, Tauri bridge, and event pagination together while leaving the previously released domain governance coordinator and legacy endpoints intact. The append-only governance database requires no destructive migration rollback: the schema addition is an index-compatible event read path over the existing table.

GMH-32 and GMH-33 must consume the unified contract rather than adding another approval authority. A Windows release candidate is built only after both stories and the full automated M3 gates complete.
