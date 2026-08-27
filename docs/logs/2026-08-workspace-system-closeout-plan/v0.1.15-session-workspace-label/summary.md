# Session 工作区标签一致性

本阶段修复默认目录重置后当前 session 被误标为“默认工作区”的问题。

- Manager `/api/workspace` 新增 `uses_default_workspace`，用活动 runtime 根目录与当前持久化默认目录比较；默认目录改变而 runtime 未重启时，状态会明确标记为脱钩。
- GUI WorkspaceChip 对脱钩 runtime 显示实际目录名，不再把 `configured` source 直接当成默认标签。
- 目录选择器即使选择当前根目录，也会记录为 `explicit-cli` 的 session 选择；GUI 状态刷新保留该显式意图。
- 设置页保存/重置默认目录后主动刷新当前 session 状态，因此默认配置和 session runtime 仍是两个独立 authority。
