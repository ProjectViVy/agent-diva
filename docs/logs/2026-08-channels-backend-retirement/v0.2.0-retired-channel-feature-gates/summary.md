# v0.2.0 退役通道后端拆除（Cargo feature 门控）— 总结

## 背景

继 GUI 下架退役通道（`56802e12`，v0.1.1）之后，用户决策（2026-08-18）：
对 6 个未验证通道在后端**明确拆除**，但保留 channel 核心代码——做成可配置
增强：默认编译时不编译这些通道的适配器与注入逻辑，源码留在仓库，可随时按
feature 重新启用。

退役集合：`slack`、`whatsapp`、`matrix`、`irc`、`mattermost`、`nextcloud_talk`。

## 变更内容

拆除完全收敛在 `agent-diva-channels` crate 内，下游零改动：

1. **Cargo.toml**：新增 `[features]`，`default = []`；6 个 opt-in feature
   `channel-slack`（= `dep:slack-morphism`）、`channel-whatsapp`、
   `channel-matrix`、`channel-irc`、`channel-mattermost`、
   `channel-nextcloud-talk`。`slack-morphism` 转 `optional = true`。
   `whatsapp_bridge_integration` 测试以 `[[test]] required-features`
   挂到 `channel-whatsapp`。
2. **src/lib.rs**：6 个 `pub mod` 与 6 个 `pub use XxxHandler` 各加
   `#[cfg(feature = "channel-*")]`，顶部注明退役决策与恢复方式。
3. **src/manager.rs**：四处注入点逐臂门控——
   - 顶部 6 个 handler `use` 导入；
   - `channel_validation` 6 个 match 臂（兜底 `_ => return None`）；
   - `configured_channel_names` 名单 6 个条目；
   - `build_updated_handler` 6 个 match 臂（兜底 `_ => None`）；
   - `initialize()` 6 个启动块。

   每处附 `// RETIRED 2026-08-18：默认拆除，feature channel-* 恢复` 注释。
4. **回归测试**：`configured_channel_names_excludes_retired_channels_by_default`
   ——6 个退役通道全部 `enabled=true` 且字段齐全时不得进入可路由名单，
   telegram 齐全为阳性对照；各通道断言带 `#[cfg(not(feature))]` 以便
   feature 恢复后测试仍可编译。

## 影响范围

- `agent-diva-manager` 的 `task_runtime.rs` 通过 `configured_channel_names`
  路由：退役通道自动从可路由名单消失（即拆除目标）。
- 配置层（core `ChannelsConfig`、manager `apply_channel_update`/开关宏、
  CLI `channel_statuses`）保持历史性兼容，未改动。
- `slack-morphism` 依赖默认不再参与编译。

## 明确不做

- 不删 6 个通道源文件与依赖声明（仅转 optional）。
- 不动 core 配置 schema 与 CLI 状态输出。
- 不做运行时开关（编译期 feature 即配置点）。
