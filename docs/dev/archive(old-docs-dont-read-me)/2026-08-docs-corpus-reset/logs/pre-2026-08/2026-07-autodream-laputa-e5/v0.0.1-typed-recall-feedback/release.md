# E5 Release Notes

This is an internal architecture slice, not a standalone product release.

Operators gain no new automatic Memory-write policy. Approved proposals remain
mandatory. The new durable artifact is `.laputa/recall-feedback.json`, bounded
to 5,000 payload-free events and protected by the existing Laputa lock and
atomic-write mechanisms.

Rollback is the commit revert plus removal of feedback generated only by a test
profile when applicable. Typed authority data must not be manually deleted.
No push or deployment is performed.
