# Acceptance — v0.1.0 channels-settings-repair

## 前置

1. 网关运行新版 agent-diva-manager（重新编译后 `just diva-gate` 或重启网关）。
2. 启动 GUI（`npm run tauri dev` 或安装新版桌面端）。
3. 配置文件中至少有一个已启用通道（如 telegram）用于对照。

## 验收步骤

### A. 卡片显示（核心缺陷回归）

- [ ] 打开 设置 → 频道，卡片视图显示 13 张卡片。
- [ ] 每张卡片显示平台名称（Telegram / Discord / 飞书 / 钉钉 / Email / Slack / QQ /
      Matrix / Neuro-Link / IRC / Mattermost / Nextcloud Talk / WhatsApp）。
- [ ] 品牌平台显示品牌图标，email/neuro-link/irc/matrix 显示兜底图标。
- [ ] 启用状态与配置文件一致（已启用通道不再显示"已禁用"）。

### B. 开关持久化

- [ ] 先点选 telegram 卡片（使其成为选中项），再切换 discord 卡片的开关。
- [ ] 配置文件 `channels.discord.enabled` 随之翻转；刷新页面后状态保持。
- [ ] 网关日志无 "Unknown channel" 警告；切换 neuro-link/irc/mattermost/nextcloud_talk
      同样持久化。

### C. 向导编辑

- [ ] 列表视图选择 slack（或 irc/mattermost/nextcloud_talk），详情区出现
      "通过配置向导编辑"按钮（不再是"请直接编辑配置文件"死文案）。
- [ ] 点击后向导打开且平台已选中；若该通道已有配置，凭据字段预填。
- [ ] 填写并保存后配置持久化；已有 discord 的 `allow_from` 等非向导字段不被重置。
- [ ] 向导平台选择网格不含 matrix（无凭据字段）；matrix 卡片仍显示名称/图标。

### D. 网关回退

- [ ] （可选）在网关 runtime 未完全就绪时访问频道页，卡片显示配置文件中的真实
      enabled 状态而非全部"已禁用"。
