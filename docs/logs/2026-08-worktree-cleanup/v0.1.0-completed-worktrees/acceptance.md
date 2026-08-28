# Acceptance

从用户视角确认：

1. 根目录仍可作为 `dev` 工作树使用。
2. 已完成内容对应的旧 worktree 和残留副本不再出现在磁盘或 `git worktree list` 中。
3. 仍有未提交改动的 `agent-diva-m3-hitl` 未被删除。
4. 未合并的 `feat/workspace-agents-impl` 分支仍可用于后续恢复或审阅。
5. 根工作树原有未提交文件保持原状。
