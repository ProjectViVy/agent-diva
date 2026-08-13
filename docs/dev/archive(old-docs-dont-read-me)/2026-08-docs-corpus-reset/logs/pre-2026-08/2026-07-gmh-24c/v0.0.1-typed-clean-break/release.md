# GMH-24C release

This slice is committed locally as one Conventional Commit and is not pushed.
Production rollout requires an explicit offline Migration apply/verification,
configuration switch to `typed`, restart validation, and the deferred G2D
desktop acceptance. There is no online fallback to a removed runtime.

Configuration rollback may select `legacy` only for an existing installation
before production migration acceptance. Once typed cutover is accepted, use
the Migration manifest/backup and governed rollback facilities; do not dual
write.
