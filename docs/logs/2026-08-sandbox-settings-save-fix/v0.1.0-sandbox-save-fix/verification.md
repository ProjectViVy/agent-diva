# verification：沙箱设置保存修复

## 已执行的自动化验证（2026-08-05）

| 命令 | 结果 |
| --- | --- |
| `cd agent-diva-gui && npm test` | 通过：58 个测试文件 / 451 个用例全绿（含新增 3 个保存回归用例） |
| `cd agent-diva-gui && npm run build`（vue-tsc --noEmit + vite build） | 通过，无类型错误 |

新增回归用例（`SandboxSettingsSection.test.ts`）：

1. 切换模式下拉后保存，断言 `saveSandboxConfig` 收到的 payload 全部为
   snake_case 枚举值（与 Rust `#[serde(rename_all = "snake_case")]` 契约一致）。
2. 清空 timeout 输入框后保存，payload 的 `timeout_seconds` 回退为 60，不再产生
   非数值导致反序列化失败。
3. 保存失败时 toast 显示后端真实错误（断言包含 `unknown variant` 字样）。

说明：`just gui-automated-check` 中的 `cargo check --manifest-path
agent-diva-gui/src-tauri/Cargo.toml` 未运行——本次未改动任何 Rust 代码，
Tauri 命令与 schema 保持原状，无需重复编译验证。

## 延期项：真实桌面冒烟

本会话无法驱动真实 Tauri 桌面窗口，GUI 实机冒烟延后至人工验收
（步骤见 `acceptance.md`），并已在 `TODOLIST.md` 记录为 open 项。
