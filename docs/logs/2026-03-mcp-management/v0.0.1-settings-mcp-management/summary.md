# Summary

- 在 GUI 设置首页新增独立 `MCP` 入口，不挂载到“通用”页。
- 新增 `McpSettings` / `McpManagementCard`，支持查看当前启用 MCP、表单创建编辑、JSON 文件导入、原始 JSON 编辑、启用禁用、删除和状态刷新。
- `agent-diva-manager` 新增 MCP 管理 service 与 `/api/mcps` 系列接口，返回配置与连接状态，并在变更后立即触发运行时 MCP 热更新。
- `agent-diva-agent` 新增 `RuntimeControlCommand::UpdateMcp`，运行时会替换现有 `mcp_*` 工具集合，无需重启。
- `agent-diva-core` 在 `tools` 下新增 `mcp_manager.disabled_servers` 元数据，用于保留现有 `mcp_servers` 结构并支持启用/禁用。
