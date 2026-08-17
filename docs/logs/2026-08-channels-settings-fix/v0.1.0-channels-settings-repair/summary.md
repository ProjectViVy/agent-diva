# Summary — v0.1.0 channels-settings-repair

## 背景

GUI 频道设置页完全不可用：所有卡片显示"已禁用"、无频道名称与品牌图标；详情视图
对 telegram/discord 以外的通道只显示"配置项暂未完全支持 UI 编辑"；卡片开关切换
不持久化。

## 根因

1. 数据契约错配：`get_channels` 返回原始 `ChannelsConfig`（value 即配置对象），
   `ChannelCardView.vue` 却按 `{name, enabled, config}` 包装对象传给 `ChannelCard`，
   `channel.name` 恒为 undefined。
2. `handleCardToggle` 只触发 `saveCurrentChannel()`，后者只保存 `selectedChannel`，
   切换非选中卡片不持久化。
3. 网关 `get_channels_handler` 在 runtime oneshot 失败时返回 `ChannelsConfig::default()`
   （全部 enabled=false），误导为"全部已禁用"。
4. `apply_channel_update` 缺 neuro-link/irc/mattermost/nextcloud_talk 分支，更新被静默丢弃。
5. 向导编辑 initial-data 把配置平铺在顶层，而 `ChannelWizardModal` 只读
   `initialData.credentials`，编辑时字段全空；向导完成为整体替换，会重置非向导字段。
6. matrix 未登记进 `channel-icons.ts`。

## 变更

### GUI（agent-diva-gui）

- `ChannelCardView.vue`：从 entries 组装规范化 `{name, enabled, config}`，修复名称/图标/启用态。
- `ChannelsSettings.vue`：抽出 `persistChannel(name, config)`，卡片开关直接持久化被点通道；
  详情视图对 12 个向导支持平台提供"通过配置向导编辑"入口；向导 initial-data 改为
  `credentials` 包装；`handleWizardComplete` 合并既有配置再保存；新增
  `normalizeWizardCredentials`（select 字符串转 bool、irc `channels_str` → `channels`）；
  `openWizard` 清理残留编辑态。
- `ChannelWizardModal.vue`：平台网格改遍历 `CHANNEL_PLATFORMS`（matrix 无凭据字段，不进向导）。
- `channel-icons.ts`：登记 matrix（Lucide Boxes 兜底图标 + 显示名 + 描述）。
- `locales/zh.ts`、`locales/en.ts`：新增 `editViaWizard` / `editViaWizardHint`（zh 另加 `platformMatrix`）。
- 新增测试：`ChannelCard.test.ts`（5）、`ChannelCardView.test.ts`（4）、`ChannelsSettings.test.ts`（4）。

### 网关（agent-diva-manager）

- `runtime_control.rs`：`apply_channel_update` 补齐 neuro-link（wire 名映射 `neuro_link`
  字段）、irc、mattermost、nextcloud_talk 分支；未知通道由静默 warn 改为 `bail!`；
  `impl_channel_toggle!` 扩展到 13 类型；新增 2 个单元测试。
- `handlers.rs`：`get_channels_handler` runtime 失败时回退用 `ConfigLoader::with_dir(config_dir)`
  读取磁盘配置，不再返回全禁用的默认值。

## 影响范围

- GUI 设置 → 频道页（卡片视图、列表详情、配置向导）。
- 网关 `/api/channels` GET/POST 行为（失败回退、四通道更新接受）。
- 不涉及 agent/provider/memory/persona 路径。

## 明确不做（已记录 TODOLIST）

- `cli_runtime.rs channel_statuses` 补齐 4 通道状态（CHANNELS-STATUS-COVERAGE）。
- 向导连接测试与卡片删除后端接入（CHANNELS-WIZARD-TEST-DELETE）。
