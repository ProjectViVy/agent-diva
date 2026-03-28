# Acceptance

## 验收步骤

1. 打开各 crate 的 `Cargo.toml`，确认 `version = "0.4.0"`。
2. 检查主项目 crate 的 `authors` 字段，确认统一为 `mastwet (projectViVY Team, undefine foundation)`。
3. 检查 `agent-diva-cli/src/main.rs`，确认 CLI 版本展示为 `0.4.0`。
4. 检查 GUI `agent-diva-gui/package.json`、`agent-diva-gui/src-tauri/Cargo.toml`、About 页面，确认版本与作者展示已同步。
5. 检查 `justfile`、`scripts/package-linux.sh`、`scripts/package-macos.sh`、`docs/packaging.md`，确认打包版本号为 `0.4.0`。

## 当前验收状态

- 元数据修改已完成。
- 运行态 smoke 仍需在解除 `agent-diva.exe` 文件占用后复验。
