# Summary

- 修复 `agent-diva-gui` 中“供应商 -> 新增模型”无法新增供应商的问题。
- 修复“手动添加模型”只更新前端状态、不会写回 provider 配置的问题。
- 为可用模型列表补充自定义模型删除按钮，并持久化删除结果。
- 在供应商侧栏新增“新增供应商”入口，支持创建自定义 OpenAI 兼容供应商。
- 新增 Tauri `create_custom_provider` 命令，将自定义供应商写入配置中的 `providers.custom_providers`。
- 新增 Tauri `add_provider_model` 命令，将手动添加的模型持久化到 provider 配置。
- 新增 Tauri `remove_provider_model` 命令，用于删除 provider 中的自定义模型。
- 创建成功后自动切换到新供应商，并保存默认模型到快捷模型列表。
