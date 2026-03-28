# 迭代摘要

- 目标：修复 Windows GUI 安装流程，把“GUI 自管 Gateway 运行时”和“可选系统服务”彻底分离。
- 安装器调整：移除 NSIS 安装阶段的自定义服务钩子，不再在安装期间尝试注册或启动 Windows Service。
- 打包校验：增强 `bundle:prepare` 与 Windows 打包脚本，明确要求 `agent-diva.exe` 必须成功 staged 到 `src-tauri/resources/bin/windows/` 后才允许继续打包。
- GUI 文案：把设置页描述改为“Gateway Runtime”，说明 GUI 默认管理本地 Gateway，Windows Service 只是高级可选能力。
- 影响：MSI/NSIS 的成功标准回归为“GUI + Gateway runtime 已正确落地”，避免把服务安装失败误判为应用安装失败。
