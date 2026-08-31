# Release

There is no product release or deployment for C5-P. The output is an isolated-branch architecture
and handoff package.

- Commit only documentation, TODOLIST bookkeeping, and lock records on `feat/channel-epic`.
- Do not merge `dev`, push, package, deploy, or enable a second channel runtime.
- Receiving agents implement C5-I/V from this freeze using isolated worktrees and focused commits.
- C6 remains the only production cutover and atomic merge point.

Rollback is a focused revert of the C5-P documentation commit; no runtime or persisted data needs
migration.
