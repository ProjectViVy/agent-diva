# 发布说明

- Windows GUI 打包继续使用 `scripts/package-windows-gui.ps1`。
- 本次发布不再依赖安装阶段执行 Windows Service 注册；安装器只负责交付 GUI 与 Gateway runtime 文件。
- 若用户需要后台常驻或开机自启，应在安装完成后通过 GUI 中的服务管理能力显式启用。
