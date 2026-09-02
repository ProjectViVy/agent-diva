# Release

本迭代只在隔离 worktree `feat/channel-epic` 交付，不推送、不合入主线；按 CHANNEL-EPIC
约定在 C6 完成后一次性原子合入。

生产行为无需新增配置：Manager 使用现有 `config_dir` 创建
`super-channel-events.db`，Neuro-Link 仍固定 loopback；无身份、端口或认证配置变更。
