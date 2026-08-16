# 发布说明

需要同时更新桌面端（Tauri 解码）与 Manager（Persona 信封）。只更新一端时：

- 新 GUI + 旧 Manager：Tauri 回退读取对象形 `status`，首次设置门可开。
- 旧 GUI + 新 Manager：旧解码仍把信封 `status: "ok"` 当成 payload，状态会坏；必须升级 GUI。

无配置迁移，无数据改写。未 push。回滚：还原本片提交即可。
