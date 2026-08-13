# Release

发布前准备：

1. 在 Diva 配置中开启 `channels.neuro_link.enabled=true`，并确保 `host/port` 对 OLV 可达。
2. 如 Diva 配置了 `channels.neuro_link.allow_from`，需要把 `olv-avatar` 加入允许列表。
3. 在 OLV `conf.yaml` 的 `system_config.neuro_link` 中开启：
   - `enabled: true`
   - `host`: 指向 Diva NeuroLink server 地址
   - `port`: 指向 Diva NeuroLink server 端口
   - `sender: 'olv-avatar'`
   - `chat: '__olv_avatar__'`

发布方式：

- 先部署/启动 Diva gateway。
- 再启动 OLV 服务，使其后台 NeuroLink bridge 完成注册。
- 最后启动 OLV 前端页面和 Diva GUI，开始 GUI 对话。

注意事项：

- 本轮只同步 GUI 最终回复，不同步 delta。
- OLV bridge 启动后会持续重连 Diva NeuroLink server。
- 若 OLV 前端没有活跃 WebSocket 客户端，收到 `speak` 时只会记录 warning，不会缓存离线播报。
