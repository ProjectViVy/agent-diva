# Migration revision contract summary

The real-profile G2D preflight found that a successful import reported the
post-import revision as both the expected revision and the base for another
record-count increment. Reports now expose the actual before/after revisions.

Rollback manifests retain an explicit rolled-back state. Replay validates the
store revision and deterministic record presence, while a reapply after
rollback safely reuses the verified pre-import backup.
