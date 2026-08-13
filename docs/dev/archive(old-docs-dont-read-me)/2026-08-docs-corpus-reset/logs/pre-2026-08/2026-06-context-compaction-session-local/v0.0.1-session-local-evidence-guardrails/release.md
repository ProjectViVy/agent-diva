# Story 5.3 Release

## Method

No deployment was performed in this iteration. This is a code-level governance guardrail change prepared for review.

## Release Notes

- AutoDream now treats context compaction capsules as secondary evidence.
- AutoDream refuses to persist output artifacts or proposal candidates that rely only on context compaction evidence.
- Shared evolution domain helpers define primary governance evidence semantics for reuse.

## Rollback

Revert the Story 5.3 commit. This removes the evidence validation helper, AutoDream validation call sites, and the secondary-evidence marker.

