# Release — v0.1.0 channels-settings-repair

## 发布方式

本迭代为本地桌面产品修复，随下一次桌面端构建生效：

1. 网关侧（切片 5/6）：重新编译并重启网关进程（`just diva-gate` 或对应服务重启），
   新 `apply_channel_update` 分支与 `get_channels_handler` 回退才生效。
2. GUI 侧（切片 1-4）：重新构建并启动桌面端（开发：`cd agent-diva-gui && npm run
   tauri dev`；发布：既有 Tauri 打包流程）。

## 版本与分支

- 分支：`agent-diva-pro`（未 push，待用户指示）。
- 提交：按切片分笔 Conventional Commit（见 summary）。

## 回滚

- 各切片提交相互独立，可按 commit 逐一 revert。
- 网关行为变化仅在 runtime 失败回退与四通道更新接受上；revert 切片 5/6 的提交即可
  恢复旧行为，不影响 GUI。

## 未随本次发布

- 向导连接测试、卡片删除后端接入（TODOLIST：CHANNELS-WIZARD-TEST-DELETE）。
- config-status 四通道状态补齐（TODOLIST：CHANNELS-STATUS-COVERAGE）。
