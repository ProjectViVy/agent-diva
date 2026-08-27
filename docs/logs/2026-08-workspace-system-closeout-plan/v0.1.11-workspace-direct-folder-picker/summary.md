# Workspace 直接目录选择

## 变更

- WorkspaceChip 的“切换工作区”不再跳转设置页，直接打开系统原生目录选择窗口。
- 选中的目录先经过既有 `inspect_workspace` canonicalize、可读性与 AGENTS 检查，再调用现有原子切换动作。
- 取消目录选择不改变当前工作区；选择和切换期间禁用重复操作。
- 目录选择或预检失败通过全局 Toast 反馈；原子切换错误继续由 App 权威动作反馈。

## 边界

本次只修正入口行为，不改变 workspace/session 持久化合同或原子切换事务。
