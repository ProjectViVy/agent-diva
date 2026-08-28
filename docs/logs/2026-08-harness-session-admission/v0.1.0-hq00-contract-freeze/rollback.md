# Rollback

HQ-00 changes only tests and documentation. Roll back its two focused commits to remove the characterization
module and contract records. No runtime data, configuration, database, or external service rollback is needed.

If only the written contract is reconsidered, revert the documentation commit while retaining the baseline
tests; do not begin HQ-01 until a replacement contract is approved.
