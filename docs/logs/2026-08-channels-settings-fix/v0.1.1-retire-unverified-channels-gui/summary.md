# Summary — retire unverified channels from GUI

用户决策（2026-08-18）：把未经验证的频道从 GUI 明确拆掉 —— slack、whatsapp、
nextcloud_talk、mattermost、matrix、irc。后端代码不删，依赖与程序逻辑仅作
历史性保留。

## GUI 变更（本次交付）

- `channel-platforms.ts`：新增 `RETIRED_CHANNELS` 清单与 `isRetiredChannel()`；
  `CHANNEL_PLATFORMS` 移除 slack/whatsapp/irc/mattermost/nextcloud_talk 五个
  向导平台条目（matrix 本无向导条目）。
- `channel-wizard-fields.ts`：移除上述 5 个平台的凭据字段定义。
- `channel-icons.ts`：移除 6 个退役平台的图标 / 显示名 / 描述条目及不再使用
  的导入；删除 4 个品牌图标组件（SlackIcon / WhatsAppIcon / MattermostIcon /
  NextcloudTalkIcon）。
- `ChannelCardView.vue`：卡片列表过滤退役频道；仅剩退役频道时显示空状态。
- `ChannelsSettings.vue`：列表视图侧边栏改用过滤后的 `visibleDraftChannels`；
  自动选择跳过退役频道；`normalizeWizardCredentials` 移除 irc `channels_str`
  映射与 `use_tls` 字符串转换。
- `SettingsView.stories.ts`：mock 移除 whatsapp/slack。
- 测试更新：`ChannelCardView.test.ts`（6 例，含退役过滤与仅退役→空状态）、
  `ChannelsSettings.test.ts`（5 例：feishu 切换/合并/预填、email bool 规范化、
  侧边栏隐藏退役频道）。

## 明确保留（历史性保留，不删）

- `agent-diva-channels` crate 全部适配器代码与依赖。
- `agent-diva-core` 配置 schema 中 6 个通道的结构定义。
- `agent-diva-manager` 网关的更新路由 / 开关宏（含 `cce52a4e` 新增分支）。
- CLI `channel_statuses` 现有覆盖。

结果：频道页仅显示 7 个在维通道（telegram、discord、feishu、dingtalk、
email、qq、neuro-link）；即使配置文件中退役通道 enabled=true 也不展示。
