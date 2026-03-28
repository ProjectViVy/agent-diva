# 验收步骤

1. 运行 Windows GUI 打包脚本，确认打包前校验会检查 `gui-bundle-manifest.json` 和 `agent-diva.exe` staged 路径。
2. 安装 NSIS 包，确认安装过程不再出现“安装 Gateway 服务失败”的安装阶段弹窗。
3. 安装 MSI 包并首次启动 GUI，确认 GUI 可以自动拉起本地 Gateway，控制台健康检查变为在线。
4. 在 GUI 中验证“启动网关/停止网关”仍可正常工作。
5. 在需要时再手动测试 Windows Service 安装；若失败，不应影响 GUI 与 Gateway 的正常使用。
