# 完成归档：WORKSPACE 系统收尾正式关闭

关闭日期：2026-08-28

本记录将已完成的 WORKSPACE 系统收尾 WBS 从根目录活跃待办移入历史归档。归档内容仅用于
追溯，不再驱动新的实施或验收；如未来重新出现问题，须先重新验证，再建立新的活跃条目。

## 关闭范围

- [x] `WORKSPACE-SYSTEM-CLOSEOUT`：统一运行时 `WorkspaceContext`、AGENTS 合同、工作区
  切换和分层会话历史。
- [x] `WS-00`：接管并合入 workspace-agents Wave A–D，贯通 Manager、CLI、Tauri 和 GUI。
- [x] `WS-01`：GUI 唯一工作区快照、只读状态展示、聊天底栏入口和默认/显式身份显示。
- [x] `WS-02`：原生目录选择、canonicalize、可读性与 `AGENTS.md` 候选预检。
- [x] `WS-03`：停止→保存→重建→恢复的原子切换、运行态阻塞、失败回滚和 Gateway 生命周期。
- [x] `WS-04`：`workspace/channel/root/parent/kind/branch_label` 会话 lineage 合同及 API 投影。
- [x] `WS-05`：按 workspace → channel → root → branch/subagent 展示历史，并保留 legacy 语义。
- [x] `WS-06`：自动化门禁、真实 Windows 桌面验收、文档收口和归档。
- [x] `WORKSPACE-AGENTS-MD-INJECTION`、`WORKSPACE-GUI` 和 managed-path 旧条目已并入上述
  WBS，不再重复维护。

## 交付与验收结论

- `WorkspaceContext` 成为执行、Shell、Plan、Session 和 AGENTS 读取共同绑定的 canonical root。
- Workspace 候选预检、切换事务、会话隔离、legacy root 和真实 branch/subagent 均有自动化覆盖。
- `just ci`、`just gui-automated-check`、Tauri workspace guard、Manager `/api/workspace`、
  CLI effective workspace 和 Gateway lifecycle focused tests 已通过。
- 用户于 2026-08-27 在真实 Windows Tauri 桌面完成工作区重置、当前 session 显示、显式同路径
  选择、刷新和新建聊天验收，并确认通过。

## 分支、工作树与文档状态

- 当前 `dev` 是成果唯一主线；已完成的 `m3-hitl`、`workspace-agents` 旧分支和 worktree 已
  退休，不再作为后续开发入口。
- `LOCK.md` 已释放；本次关闭不推送远端、不发布安装包，也不删除用于追溯的研究包和迭代日志。
- 设计、验证、接受和分支退休记录仍保留在：
  `docs/research/workspace-agents-diva-adaptation-2026-08/`、
  `docs/logs/2026-08-workspace-system-closeout-plan/`、
  `docs/logs/2026-08-worktree-cleanup/`、
  `docs/logs/2026-08-branch-integration/`。

## 关闭边界

- `WS-CLI-LEGACY-DEFAULT-MIGRATION` 是独立的 sev-P3 兼容性后续项，继续留在活跃
  `TODOLIST.md`，不重新打开本 Workspace WBS。
- 根工作树中与本次归档无关的既有未提交文件保持原样，不纳入本次归档或清理。
