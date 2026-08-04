# summary：修复 GUI 沙箱设置"保存设置"失败

版本：v0.1.0-sandbox-save-fix ｜ 日期：2026-08-05 ｜ 分支：agent-diva-pro

## 问题

GUI"沙箱设置"页点击"保存设置"失败，仅显示通用"保存失败"toast。

根因：前后端枚举序列化格式不匹配。

- 后端 `agent-diva-core/src/config/schema.rs` 的 `SandboxMode`/`AskForApproval`
  使用 `#[serde(rename_all = "snake_case")]`（`read_only`、`workspace_write`、
  `danger_full_access`、`on_failure`、`on_request`、`unless_trusted`）。
- 前端 TS 类型与下拉选项使用 kebab-case（`read-only`、`workspace-write` 等）。
- Tauri 命令 `save_sandbox_config`（`agent-diva-gui/src-tauri/src/commands.rs:6314`）
  反序列化时报 `unknown variant`，保存必然失败；前端 catch 吞掉真实错误。

次要问题：timeout 输入框清空时产生非数值（`null`/空串），`u64` 反序列化失败；
保存失败 toast 不含后端真实错误信息。

## 变更

仅 `agent-diva-gui` 前端（Rust 后端不改，snake_case 与存量 config.json 一致）：

1. `src/api/desktop.ts`：`SandboxConfig` 的 `mode`/`approval_policy` 联合类型改 snake_case。
2. `src/components/settings/SandboxSettingsSection.vue`：
   - 下拉选项数组与三处默认值改 snake_case；
   - `saveConfig` 提交前清洗 `timeout_seconds`（非有限值或 <1 回退 60，上限 600）；
   - 保存失败 toast 附带后端真实错误信息。
3. `src/locales/zh.ts`、`src/locales/en.ts`：modes/policies i18n key 改 snake_case（文案不变）。
4. `src/components/settings/SandboxSettingsSection.test.ts`：mock 数据改 snake_case；
   新增 3 个回归用例（snake_case payload、清空 timeout 回退 60、失败 toast 显示真实错误）。

## 影响范围

- GUI 沙箱设置页的加载回显、下拉标签、保存流程。
- 不影响 gateway/CLI 的配置读取路径（schema 未变）。
