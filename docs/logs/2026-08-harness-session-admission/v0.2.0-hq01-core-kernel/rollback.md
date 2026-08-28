# Rollback

Revert the HQ-01 kernel commit and its documentation closeout commit. The kernel has no AgentLoop consumer,
configuration entry, persisted state, database schema, or public wire projection, so rollback requires no data
or operator migration.

The independent HQ-00 characterization-timeout stabilization may be retained because it changes only the
maximum wait for expected test progress and keeps the serialization assertion intact.
