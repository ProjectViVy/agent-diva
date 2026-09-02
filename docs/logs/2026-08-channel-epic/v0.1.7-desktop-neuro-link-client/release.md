# Release

本迭代只在隔离 worktree `feat/channel-epic` 交付，不推送、不合入 `dev`；按
CHANNEL-EPIC 约定，C6 完成 clean break、全量门禁和人工验收后一次性原子合入。

生产配置面不变：Neuro-Link 继续复用 Manager loopback，未增加身份、host、port 或 auth
配置；GUI 端点从现有 Tauri `get_gateway_status` 获取。C4 的 commit 可以独立审阅，不能单独
作为生产 cutover；发布前必须完成 acceptance.md 中的桌面冒烟及 C6 旧链路删除检查。
