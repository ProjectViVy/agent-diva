# Release — v0.1.2 channel editor GUI

无需独立发版。变更只在 GUI 前端：

1. 重新编译 / 启动桌面 GUI（`npm run tauri dev` 或安装新构建）以加载新页面。
2. 网关无需重启：`update_channel` 契约未改。
3. 不 push。合并进 `agent-diva-pro` 后随下一次 GUI 打包生效。
