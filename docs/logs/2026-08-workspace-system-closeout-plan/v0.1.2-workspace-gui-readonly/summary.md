# WS-01：唯一 WorkspaceContext 与只读 GUI

本阶段把后端的权威 `WorkspaceContext` 以单一只读快照接入桌面 GUI：

- Tauri 新增 `get_workspace_status` 命令，直接读取 Manager `/api/workspace`。
- 前端 `useWorkspaceContext` 提供模块级共享快照，统一处理启动加载、刷新、错误保留和
  generation 竞态，页面组件不再自行解析工作区路径。
- Topbar 新增 `WorkspaceChip` 与 Popover，展示工作区 basename、完整路径、来源以及
  `AGENTS.md` 状态；设置页新增只读 Workspace Settings。
- General Settings 移除重复的 workspace 路径回显，workspace 信息统一归入专用表面。
- 本阶段明确不提供路径编辑、热切换、候选目录预检或隐式迁移；这些能力由后续 WS-02/03
  负责。

实现位于隔离 worktree `feat/workspace-system-closeout`，未修改共享根工作树中的既有业务改动。
