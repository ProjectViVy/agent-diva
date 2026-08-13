# Acceptance

Automated acceptance:

1. Equivalent existing workspace path spellings resolve to one identity.
2. ExperienceJournal retains its historical identity and remains readable.
3. Manager, AgentLoop, Recall, governed apply, and Migration use that identity.
4. A raw-path typed store produces a backup and prepared/applied manifest.
5. Only workspace scope fields change; payload and revisions remain stable.
6. Corrupt, foreign, missing, or unexpected identities fail closed.
7. Rollback restores the verified legacy database and records `rolled_back`.

Final real-desktop acceptance remains deferred until all E7 automated gates are
closed, as requested.
