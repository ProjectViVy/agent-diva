# Release

## 发布方式

- 跟随下一版 `agent-diva-gui` 桌面端构建发布。
- 无需单独数据库迁移。

## 发布关注点

- 已有用户首次升级后，桌宠 ASR 会被一次性迁移为开启。
- 用户后续手动关闭 ASR 后，不会在下一次启动时再次被强制开启。
- 如 MiniMax 仍出现连接失败，应优先检查：
  - 最终 `ws_url`
  - API Key 是否有效
  - 网络/代理是否影响 WebSocket 握手

## 回滚说明

- 如需回滚，可回退 `agent-diva-core`、`agent-diva-gui`、`agent-diva-gui/src-tauri` 中本次迭代对应提交。
- 回滚后升级用户的本地迁移标记会保留，但不影响旧版本继续运行。
