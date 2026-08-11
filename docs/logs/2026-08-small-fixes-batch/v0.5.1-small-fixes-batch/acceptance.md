# v0.5.1-small-fixes-batch Acceptance

## 验收步骤

1. **gateway 端口配置生效**
   - 在 `~/.agent-diva/config.json` 设置 `"gateway": { "port": 3999 }`。
   - 运行 `just run -- gateway`（或 `cargo run -p agent-diva-cli -- gateway`）。
   - 观察点：HTTP API 监听在 3999（`curl http://127.0.0.1:3999/health` 或
     端口探测），而不是 3000。
2. **未配置端口行为不变**
   - 移除 `gateway.port` 配置后重启 gateway，应仍监听 3000。
3. **example 编译**
   - `cargo clippy -p agent-diva-providers --all-targets -- -D warnings` 通过。

## 自动化证据

- `cargo test -p agent-diva-cli gateway_runtime_config`：2/2 通过
  （`gateway_runtime_config_uses_configured_port`、
  `gateway_runtime_config_keeps_default_port_when_unconfigured`）。
