# Acceptance

## 已通过

- 当前 workspace 状态在 Manager、Tauri 与 GUI 间保持同一 canonical root。
- 无 `AGENTS.md`、存在、截断和 legacy/default 状态有明确 DTO/GUI 映射。
- 候选预检、取消、旧响应竞态和切换 guard 不会覆盖当前已生效 workspace。
- Session root/branch/subagent lineage 使用持久化合同；legacy 会话不猜造父子关系。
- 原子切换覆盖成功路径、运行态阻塞、Gateway 重建和失败回滚逻辑的自动化测试。
- Rust/GUI 全量门禁、Tauri/Manager/CLI 聚焦测试通过。

## 待人工验收

以下步骤需要真实 Windows Tauri 桌面窗口执行，完成前 WS-06 保持开放：

1. 启动 GUI，确认 Topbar 与 Settings 显示同一 canonical root。
2. 选择临时目录 A，准备 `AGENTS.md`，确认候选预览显示正确状态并完成切换。
3. 确认 Gateway 重建、Chat 输入恢复、Session 列表切换到 A；再切回原 workspace。
4. 删除 A 的 `AGENTS.md` 并重扫，确认状态变为 `missing` 且旧内容不残留。
5. 在 streaming、Plan、approval 或 HITL 期间尝试切换，确认被阻止且 root 不变。
6. 模拟 apply/rebuild 失败，确认旧 root、候选、页面位置和错误信息保留。
7. 用长路径和窄窗口检查无横向滚动、双重纵向滚动或 Footer 遮挡。
