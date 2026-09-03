# C6-D verification

日期：2026-09-04

## 门禁结果

以下命令在隔离 branch/worktree 执行，结果均为通过，除明确标注项外没有失败重试被隐藏：

- `just fmt-check`：通过。
- `just check`：通过（`cargo clippy --all -- -D warnings`）；仅有 `imap-proto v0.10.2`
  future-incompatibility warning，不影响本次门禁。
- `just test`：通过，全 workspace 测试通过；QQ live credential-gated case 保持 ignored，
  没有真实凭据或网络依赖。
- `just channel-clean-break-check`：self-test 与 active-source scan 均通过，输出
  `Channel clean-break self-test passed` 和 `Channel clean-break verified`。
- `cargo run -p agent-diva-cli -- --help`：通过，CLI usage/commands 正常输出；未使用 secret
  或外网。

聚焦回归也已执行并通过：

- `cargo test -p agent-diva-core`：727 unit tests、channel Fabric TCK 8、protocol TCK 5、
  security integration 5 通过（doc tests 通过，预期 ignored 保持）。
- `cargo test -p agent-diva-agent --tests`：434 unit/integration tests 通过；包含 dispatcher
  same-session FIFO、cross-session concurrency、queue full、wait timeout、stop、request-scoped
  stop、reset、failed-worker recovery，以及 typed trust/admission/correlation 回归。
- `cargo test -p agent-diva-channels`：131 library tests、runtime TCK 12、shared TCK 10 通过；
  QQ live harness 1 ignored。
- `cargo test -p agent-diva-manager`：129 tests 通过、1 ignored；attachment admission、Cron
  Fabric ingress、typed egress/correlation 通过；Manager autodream/marketplace integration
  也通过。
- `cargo test -p agent-diva-tools`：125 tests 通过；message tool typed attachment/media case
  通过。
- `cargo test -p agent-diva-cli --test update_plan_e2e`：CLI Manager HTTP/SSE typed envelope
  round-trip 通过。
- `cargo test -p agent-diva-core --test channel_fabric_tck --test channel_protocol_tck --test security_integration`：8 + 5 + 5 通过。
- `cargo test -p agent-diva-channels --test channel_adapter_runtime_tck --test channel_adapter_shared_tck`：12 + 10 通过。

## Clean Break 静态证据

`just channel-clean-break-check` 覆盖 Cargo/just/CI/scripts 以及 core、agent、manager、tools、
tooling、channels、CLI、E2E、service、migration、GUI active roots，并跳过 `.git`、`target`、
依赖构建输出、历史/archive 文档和 checker 自身。随后使用同一排除规则执行递归 `rg` proof，
旧 DTO/API、旧 MessageBus、旧 sender/receiver、旧 callback 与旧 execution metadata token
均为零命中。历史 archive 中保留的迁移证据不属于 active source，未被删除或机械改写。

## Rust 1.80 与剩余门禁

执行：

`just msrv-probe check -p agent-diva-core -p agent-diva-agent -p agent-diva-manager -p agent-diva-channels -p agent-diva-tools -p agent-diva-cli -p agent-diva-e2e`

Rust `1.80.0` / Cargo `1.80.1` channel-scoped probe 在解析缓存的 `getrandom 0.4.3` manifest
时失败，因为该依赖要求 Cargo 1.85-era `edition2024` feature。该结果原样保留为 C6-E open，
没有宣称 MSRV 通过。真实平台 receipt、桌面断线恢复和全工作区 MSRV 仍待 C6-E。

GUI 产品文件未在 C6-D 修改；workspace `just test` 已包含 GUI Rust tests，因此没有额外
Tauri desktop gate 可宣称完成。

## 期间修复的测试/编译问题

验证期间发现并在独立聚焦提交中修复了 clippy 的 typed content formatting、typed session
worker/Manager command boxing，以及 CLI SSE test 对 `Box<ApiRequest>` 的迁移；还修复了 failed
worker recovery test 的 waiter transition observation。最终门禁结果以上述通过结果为准。
