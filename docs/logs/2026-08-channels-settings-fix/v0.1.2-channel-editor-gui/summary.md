# Summary — v0.1.2 channel editor GUI

设置 → 频道配置在卡片模式下点编辑会切到空白列表页；列表详情只有
telegram/discord 手写表单，其余通道只提示走向导或改 YAML；向导打开后也不回填。

## 根因

1. `handleCardEdit` 只切 `viewMode = 'list'`，不打开向导。
2. 列表详情对非 telegram/discord 通道只渲染 `editViaWizardHint` 或
   `providers.unsupportedUI`。
3. `ChannelWizardModal` 不在 `open` 上升沿读取 `initialData`，编辑态永远空表单。
4. 列表详情没用 `.channels-content` padding，侧栏写死 33% 宽，编辑布局被 clip。

## 变更

- 新增 `ChannelEditorForm.vue`：按字段表渲染 7 个 GUI 通道（boolean / string-list /
  基础+高级分组）。
- `channel-wizard-fields.ts` 对齐 `ChannelsConfig` schema（含 Email 行为字段、
  Discord 高级项、Neuro-Link 默认端口 9100）。
- 卡片编辑：留在卡片模式并打开向导；编辑态跳过选平台并预填凭据。
- 列表详情：每个可见通道都挂同一套表单，去掉 YAML /「请去向导」空壳。
- 侧栏改为 220px，详情使用 `.channels-content` + 分组布局。
- 向导保存仍 merge 既有配置；编辑不强制把通道改成 enabled。

## 影响范围

- 仅 `agent-diva-gui` 频道设置页、locales、频道布局 CSS。
- 不改网关 / `update_channel` 契约，不恢复退役通道。
