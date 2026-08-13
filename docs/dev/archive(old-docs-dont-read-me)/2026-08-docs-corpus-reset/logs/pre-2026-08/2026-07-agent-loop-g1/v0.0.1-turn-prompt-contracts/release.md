# Release

No deployment or push was performed. The commits are local Conventional
Commits. There is no configuration, database, session, Manager API, or transport
schema migration.

Rollback is commit-level. Reverting prompt contracts restores the prior prompt
text; reverting bilingual validation removes English aliases but does not
rewrite stored plans.

G1.6 adds no release-time action. Its four local commits can be reverted in
reverse order without data migration; no remote push was performed.
