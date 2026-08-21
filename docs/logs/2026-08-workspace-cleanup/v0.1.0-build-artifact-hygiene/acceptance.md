# 验收步骤

1. 执行 `git status --short --untracked-files=all`，确认没有构建残留或其他未跟踪文件进入列表。
2. 执行 `git check-ignore -v target agent-diva-gui/src-tauri/gen`，确认规则来源为根 `.gitignore`。
3. 执行 `git log -1 --oneline`，确认最新提交为工作区清理提交。
4. 在确认远程分支和提交范围后，再执行用户授权的 `git push`。
