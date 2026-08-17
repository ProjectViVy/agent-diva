# Release — retire unverified channels from GUI

- 交付形态：随下一次 GUI 构建/启动生效（Tauri dev 或生产构建 `npm run build`）。
- 无网关 / CLI / 配置迁移动作；后端通道代码与 schema 全部保留。
- 回滚方式：revert 本次 GUI 提交即可；或将 `channel-platforms.ts` 中
  `RETIRED_CHANNELS` 清空临时恢复展示。
- 未 push；按仓库约定等待用户指示。
