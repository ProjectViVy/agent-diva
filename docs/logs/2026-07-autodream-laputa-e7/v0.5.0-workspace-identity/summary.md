# Canonical Workspace Identity

E7 now uses the existing ExperienceJournal canonical-path digest as the single
workspace identity across AgentLoop, Manager, typed Recall, governed apply, and
Migration. This preserves existing experience evidence while removing raw-path
typed authority IDs.

An existing typed store with the recognized legacy raw-path identity is upgraded
through a verified SQLite backup and one transaction. Memory content, content
digests, record revisions, store revision, tombstones, and supersedes edges are
not changed.

This is an E7 sub-slice. E7 remains open for side-effect seam audit,
observability, recovery drills, automated vertical E2E, and final release gates.
