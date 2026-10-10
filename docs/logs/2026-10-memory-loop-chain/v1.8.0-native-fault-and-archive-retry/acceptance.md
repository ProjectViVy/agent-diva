# Acceptance

When the owned native ACTMEM head is unreadable, canonical source survives, source high watermark remains zero, and public ACTMEM read refuses. Archive deletion cannot complete or erase original Session/user rows. Restoring the original head lets the existing native worker finish the same activity without recapture. Public deletion then succeeds and its capsule contains the original fact/session.
