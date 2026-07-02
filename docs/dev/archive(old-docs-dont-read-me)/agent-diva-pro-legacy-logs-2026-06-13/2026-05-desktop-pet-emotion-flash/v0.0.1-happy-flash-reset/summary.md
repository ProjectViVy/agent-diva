# Summary

- 修复桌宠 `happy` 等情绪常驻问题，改为收到事件后瞬时闪现约 1 秒再回到 `neutral`。
- 去掉 `deriveMoodFromMessages()` 对历史消息的持久化回扫，宠物页 mood badge 只反映最新 agent 消息。
- 抽离桌宠情绪事件签名判断，避免同一条 agent 回复因深度变化被重复发送到 `desktop-pet-emotion`。
- `vrmExpressionEnabled = false` 时，桌宠窗口统一保持 `neutral`，表情开关真正生效。
