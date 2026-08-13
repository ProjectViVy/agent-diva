# 迭代摘要

- 目标：修复 Windows 打包版 `agent-diva-gui` 在启动网关或执行服务管理命令时弹出黑色控制台窗口的问题。
- 实现：在 Tauri 后端为 GUI 发起的 Windows 子进程统一设置 `CREATE_NO_WINDOW`，覆盖 `start_gateway`、`run_service_cli` 和通用命令捕获入口。
- 影响：仅调整 `agent-diva-gui` 内部的 Windows 子进程创建方式，不改变 CLI 命令语义，也不影响 Linux/macOS 行为。
- 结果：GUI 侧触发的 `agent-diva.exe gateway run` 与 `agent-diva.exe service *` 不再因控制台子系统而弹出黑窗。
