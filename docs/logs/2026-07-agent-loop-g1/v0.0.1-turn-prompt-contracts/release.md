# Release

No deployment or push was performed. The commits are local Conventional
Commits. There is no configuration, database, session, Manager API, or transport
schema migration.

Rollback is commit-level. Reverting prompt contracts restores the prior prompt
text; reverting bilingual validation removes English aliases but does not
rewrite stored plans.
