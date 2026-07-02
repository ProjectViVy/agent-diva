# Release

- 后端发布需要更新 `agent-diva-manager`、`agent-diva-agent`、`agent-diva-cli` 与 `agent-diva-gui`。
- 若以打包 GUI 交付，需重新构建 Tauri 应用，使新的 MCP commands 与设置页一并生效。
- 若仅更新后端，`/api/mcps` 接口与运行时 MCP 热更新能力需要随 manager/agent 新二进制一起部署。
- 本次不涉及数据库迁移；配置兼容通过 `tools.mcp_manager.disabled_servers` 的新增字段保持向后兼容。
