# Acceptance

## 已通过

- 当前 workspace 状态在 Manager、Tauri 与 GUI 间保持同一 canonical root。
- 无 `AGENTS.md`、存在、截断和 legacy/default 状态有明确 DTO/GUI 映射。
- 候选预检、取消、旧响应竞态和切换 guard 不会覆盖当前已生效 workspace。
- Session root/branch/subagent lineage 使用持久化合同；legacy 会话不猜造父子关系。
- 原子切换覆盖成功路径、运行态阻塞、Gateway 重建和失败回滚逻辑的自动化测试。
- Rust/GUI 全量门禁、Tauri/Manager/CLI 聚焦测试通过。

## 人工验收通过（2026-08-27）

用户已在真实 Windows Tauri 桌面窗口完成工作区验收并确认通过。验收覆盖以下场景：

1. 默认工作区重置后，当前 session 仍显示其实际显式目录，不被默认配置覆盖。
2. 聊天入口选择当前目录时，仍保留显式 session 工作区身份。
3. 刷新、设置页保存/重置和新建聊天均保持同一 session authority 工作区。
4. 工作区入口、目录选择和状态展示在真实桌面 GUI 中可正常使用。

详细的本轮标签回归验收记录见同一收口计划下的 `v0.1.15-session-workspace-label/acceptance.md`。
