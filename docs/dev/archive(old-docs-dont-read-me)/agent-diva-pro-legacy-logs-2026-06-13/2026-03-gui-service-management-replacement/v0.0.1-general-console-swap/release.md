# 发布说明

- 本次变更为 GUI 页面结构与前端组件复用调整，不涉及发布流程变更。
- 若需要发布，沿用既有 `agent-diva-gui` / Tauri 打包流程即可。
- 由于 `just check` 当前受工作区既有 lint 问题阻塞，建议在修复 `agent-diva-manager/src/server.rs` 的未使用导入后再执行完整 CI 门禁发布。
