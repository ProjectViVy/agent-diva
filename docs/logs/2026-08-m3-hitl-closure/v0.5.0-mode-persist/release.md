# Release

本 slice 为 GUI 前端持久化，无独立发布物。

## 部署方式

随 `agent-diva-gui` 前端发布。模式存于浏览器/Tauri localStorage，无需后端。

## 说明

- 无停机、无迁移、无后端变更。
- 跨设备/清除 localStorage 后回退默认 `smart`。