# Verification

通过：

- `cargo fmt --all -- --check`
- `cargo test --manifest-path agent-diva-gui/src-tauri/Cargo.toml gateway_lifecycle --lib`
  （1 passed）
- `cargo test --manifest-path agent-diva-gui/src-tauri/Cargo.toml external_gateway_mode --lib`
  （1 passed）
- `cargo test --manifest-path agent-diva-gui/src-tauri/Cargo.toml workspace_switch_tests --lib`
  （3 passed）
- `cargo check --manifest-path agent-diva-gui/src-tauri/Cargo.toml`
- `npm test`（70 files / 502 tests passed）
- `npm run build`（Vue typecheck 与 Vite production build passed）
- PowerShell parser 检查 `scripts/make-diva.ps1`
- `just --dry-run start` 与 `just --dry-run make-diva`
- `git diff --check`

真实开发版 smoke：清除 `AGENT_DIVA_EXTERNAL_GATEWAY` 后运行 `npm run tauri dev`，Tauri
成功启动内嵌 Gateway 并写入端口 `23516`；请求 `GET /api/workspace` 返回 HTTP 200。测试 GUI
与 Vite 进程随后已停止。

smoke 发现未配置 workspace 时根目录继承 Tauri 进程 CWD；此独立缺口已记录为
`WS-GUI-DEFAULT-ROOT-STABILITY`，不扩入本次生命周期修复。
