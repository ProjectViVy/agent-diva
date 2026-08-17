# v0.2.0 退役通道后端拆除 — 验收

## 默认构建（退役通道不编译、不路由）

1. `cargo build -p agent-diva-channels`：编译成功，且产物不包含
   slack/whatsapp/matrix/irc/mattermost/nextcloud 任何适配器符号。
2. 运行 `agent-diva` 网关并在配置中把退役通道设为 `enabled = true`：
   - `configured_channel_names` 不返回退役通道，消息不会被路由过去；
   - 启动日志不出现退役通道的初始化/启动块；
   - 对退役通道下发运行时更新 → `build_updated_handler` 返回 None（未知通道）。
3. 现役通道（telegram/discord/feishu/dingtalk/qq/email/neuro-link）行为不变。

## 恢复构建（feature 重新启用）

1. `cargo build -p agent-diva-channels --features channel-slack,channel-whatsapp,channel-matrix,channel-irc,channel-mattermost,channel-nextcloud-talk`
   编译成功，6 个 handler 重新导出。
2. `cargo test -p agent-diva-channels --features channel-whatsapp`：
   `whatsapp_bridge_integration` 参与编译并可运行。
3. 防回退测试中各退役通道断言带 `#[cfg(not(feature))]`，恢复后不产生
   错误失败。

## GUI 侧（前置迭代 v0.1.1 已验收）

设置页不再展示 6 个退役通道；后端拆除后 GUI 无任何调用面变化
（GUI 已在 `56802e12` 移除退役通道入口）。
