# Workspace system closeout — v0.1.7

日期：2026-08-27

## 本迭代结果

WS-00～WS-05 的 WORKSPACE 系统实现已在隔离 worktree 完成并通过自动化收口验证：

- 运行时以 `WorkspaceContext` 作为唯一 workspace 权威来源，并由 Manager、CLI、Tauri 和 GUI 贯通。
- GUI Topbar/Settings 显示当前 canonical workspace 与 `AGENTS.md` 状态。
- 候选目录支持原生选择、canonicalize、可读性与 AGENTS 预检，旧响应不会覆盖新候选。
- workspace 切换具备运行态 guard、串行锁、Gateway 停止/重建、目标 workspace 校验和失败回滚。
- Session lineage 合同持久化 `workspace/channel/root/parent/kind/branch_label`，GUI 历史按 workspace → channel → lineage 分层展示。
- 修复 Windows 下 SQLx SQLite 路径 URL 拼接造成的 BML 503，并恢复 Skill CAS 测试的合法 JSON 输入。

## 收口状态

自动化出口与真实桌面 G2D+ smoke 均已完成；用户于 2026-08-27 确认工作区重置、当前 session 显示、显式同路径选择、刷新和新建聊天场景通过，WS-06 及其合并待办已收口。

详细命令与结果见同目录 `verification.md`，用户验收步骤见 `acceptance.md`。
