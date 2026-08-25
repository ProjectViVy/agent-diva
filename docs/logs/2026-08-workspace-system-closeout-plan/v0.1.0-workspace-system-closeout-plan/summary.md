# WORKSPACE 系统收尾规划

## 本轮结果

2026-08-26 正式启动 WORKSPACE 系统收尾，根 `TODOLIST.md` 已建立 WS-00～WS-06
可执行 WBS。2026-08-26 为规划启动日，2026-08-27 至 2026-09-07 为八个工作日
实施窗口；2026-09-08 只作为阻断问题缓冲。

本轮仅完成计划、范围和排期冻结，不修改生产代码。后续实施必须在基于最新 `dev` 的新隔离
worktree 中进行，不直接续写旧 `agent-diva-workspace-agents` 工作树。

## 冻结范围

- 接管并审计 `feat/workspace-agents-impl` 的 Wave A–D 后端提交。
- 以运行时 `WorkspaceContext` 作为唯一权威，向 GUI 提供 WorkspaceChip、设置页和只读
  AGENTS.md 状态。
- 以停止→保存→重建→恢复实现工作区切换，禁止热换 root 和半切换。
- 会话历史按 `Workspace → Channel → Root Session → Branch/Subagent` 展示；层级必须来自
  持久化 lineage，旧会话只作为 legacy root。
- 本期不建设跨工作区全局历史索引，不编辑 AGENTS.md，不隐式迁移或复制会话。

## 关键路径

`WS-00 后端合入 → WS-01 唯一 GUI 快照 → WS-02 候选预检 → WS-03 原子切换`

`WS-00 → WS-04 lineage DTO → WS-05 历史树`，两条路径最终汇合到 `WS-06` 纵向验收。
