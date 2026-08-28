# Acceptance

1. 当前 `dev` 包含审批 TTL 临近过期提示。
2. M3 已完成实现不再由独立旧 worktree 提供来源。
3. Workspace 已完成实现继续保留在 `dev`，旧的 `feat/workspace-agents-impl` 分支已删除。
4. 根工作树原有 dirty 文件未被 stage、覆盖或回滚。
5. 当前仍可通过 `git worktree list` 看到根 `dev`，且无 m3/workspace 旧 worktree。
