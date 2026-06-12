# Acceptance

1. 配置并启动 Diva gateway，确认 `channels.neuro_link.enabled=true`。
2. 配置并启动 OLV，确认 `system_config.neuro_link.enabled=true` 且能连接 Diva NeuroLink server。
3. 打开 OLV 前端页面，确认其已建立 `/client-ws` 连接。
4. 打开 Diva GUI，发起一条普通对话。
5. 确认 Diva GUI 正常收到最终回复。
6. 确认同一条最终回复被同步到 OLV，OLV Live2D 开始说话。
7. 确认 CLI/API/其他 channel 的回复不会自动触发 OLV 播报。
8. 在 OLV 已经播报时再次从 Diva GUI 发新消息，确认新一轮外部播报能继续触发，且不会走 OLV 自己的 agent 聊天链。
