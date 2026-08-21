# GUI 工作区与 AGENTS.md 设计总结

本次在既有 Workspace/AGENTS.md 研究包上补充了 Tauri/Vue GUI 设计，不修改生产源代码。

核心决定：

- `NormalMode` Topbar 增加只读 WorkspaceChip，作为当前运行边界的可见锚点；
- `SettingsView` 增加独立 Workspace 页面，承载目录选择、路径来源、AGENTS 状态和摘要抽屉；
- workspace 切换采用停止、保存、重建 Gateway、恢复会话的完整流程，不做热切换；
- AGENTS.md 在 GUI 只展示来源、预算、digest、状态和安全说明，不编辑正文、不授予权限；
- 外部 workspace 不静默创建模板，不自动迁移 Session/Plan/Persona/Memory。

详细设计见：

`docs/research/workspace-agents-diva-adaptation-2026-08/gui-workspace-agents-design.md`
