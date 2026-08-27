# Workspace 默认降级与无边框入口

## 变更

- workspace 状态尚未返回或请求失败时，入口显示“默认工作区”，不再长期显示“工作区加载中”。
- 404 等真实错误保留在入口 title 与 Popover 中，并继续提供刷新动作。
- 移除聊天底栏 WorkspaceChip 的描边，不改变点击区域、hover 或错误状态圆点。
- 不从配置或路径猜测运行时 workspace，不在 404 时伪造具体 root。

## 原因

前端更新而 Gateway 仍是旧进程时，`get_workspace_status` 会收到 `/workspace` 404。该状态属于可恢复错误，不应呈现为持续 loading。
