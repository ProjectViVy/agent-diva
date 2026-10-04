# Documentation publication and archive

User authorization covers this plan, paired source archive references and
new documentation branches. Publication branch in both repositories:
docs/wails-migration-20261004. Main and archive branch heads stay at the
respective frozen source commits.

GitHub tree/commit/ref operations publish a focused documentation commit
with the frozen main commit as parent; only docs/backlog/mutex files are
included. Author/committer attribution is checked against the human account
before attaching the commit to the branch. Remote branch/commit and content
hash reads are required before reporting publication complete.

Source archive branches archive/tauri-cabi exist and resolve to the paired
frozen main SHAs. Requested annotated tags archive/tauri-cabi-20261004 remain
pending tag-capable access; exact operations and peeled-SHA checks are in
the archive record. The connected tool exposes no tag creation method; the
local Git transport is unauthenticated. No release is published as a tag
workaround.

No PR, issue comment, merge, product release, native installer publication,
force-push or main reset is performed. Existing branches, release assets and
old data are untouched. This is not a binary archive or a rollback acceptance
record. W6 destructive retirement cannot proceed until tags and W5 evidence
are verified.

