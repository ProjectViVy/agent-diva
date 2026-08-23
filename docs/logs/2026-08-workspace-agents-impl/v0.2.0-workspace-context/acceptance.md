# v0.2.0 WorkspaceContext 工作区合同：验收步骤

> 状态：骨架（Wave C1 完成后补全）

1. 不带 `--workspace` 在项目目录启动 CLI，确认实际 workspace 为该目录。
2. 带 `--workspace DIR` 启动，确认所有工具相对路径与 Shell 默认 cwd 落在 DIR。
3. config 保留旧默认 `~/.agent-diva/workspace` 启动，确认出现一次性迁移提示。
4. 在空外部目录启动，确认未生成 `PROFILE.md`/`TASK.md`/`masks/` 等模板。
