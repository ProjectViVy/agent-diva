# 发布说明

## 发布方式

- 该改动属于 GUI 与本地 Tauri 桥接能力增强。
- 发布时需要重新构建 `agent-diva-gui`，确保新的前端代码与 `src-tauri` 命令一并打包。

## 注意事项

- MiniMax 首版依赖有效的 API Key。
- MiniMax 使用系统音色 `voiceId`，不支持参考音色导入或复刻音色。
- 若使用自定义 `baseUrl`，需要确保其对应 MiniMax WebSocket 网关可达。
