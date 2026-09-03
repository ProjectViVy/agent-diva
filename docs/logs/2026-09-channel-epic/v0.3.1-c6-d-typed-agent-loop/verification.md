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
- `just test` 中 agent targets：433 unit tests，以及 compaction（3）、compaction integration（5）、
  image multimodal（2）、world claim routing（1）integration tests 通过；包含 dispatcher
  same-session FIFO、cross-session concurrency、queue full、wait timeout、stop、request-scoped
  stop、reset、failed-worker recovery，以及 typed trust/admission/correlation 回归。
- `cargo test -p agent-diva-channels`：131 library tests、runtime TCK 12、shared TCK 10 通过；
  QQ live harness 1 ignored。
- `cargo test -p agent-diva-manager`：131 tests 通过、1 ignored；attachment admission、Cron
  Fabric ingress、typed egress/correlation 通过；Manager autodream/marketplace integration
  也通过。
- `cargo test -p agent-diva-tools`：125 tests 通过；message tool typed attachment/media case
  通过。
- `cargo test -p agent-diva-cli --test update_plan_e2e`：CLI Manager HTTP/SSE typed envelope
  round-trip 通过。
- `cargo test -p agent-diva-core --test channel_fabric_tck --test channel_protocol_tck --test security_integration`：8 + 5 + 5 通过。
- `cargo test -p agent-diva-channels --test channel_adapter_runtime_tck --test channel_adapter_shared_tck`：12 + 10 通过。

最终审查修复后的重复门禁（HEAD `4234b30c`）再次执行并通过：

- `just fmt-check`：通过。
- `just check`：通过；仅有 `imap-proto v0.10.2` future-incompatibility warning。
- `just channel-clean-break-check`：self-test 与 active-source scan 均通过。
- `just test`：全 workspace 通过；最终运行观察到 agent 433 个 unit tests、channels 131
  个 library tests、manager 131 个 tests（1 ignored）、tools 125 个 tests，以及所有集成、
  GUI Rust tests 和 doc tests 均无失败；QQ live credential-gated case 仍 ignored。
- `cargo test -p agent-diva-core supervised::store`：37 通过，包含旧 schema 无 `context` 列的
  additive migration 与 typed route roundtrip 回归。
- `cargo test -p agent-diva-agent subagent`：34 通过；`cargo test -p agent-diva-manager
  runtime_chat_resolves_typed_attachments_before_admission`：1 通过；
  `cargo test -p agent-diva-tools enqueue_background_task`：7 通过。
- Core channel/protocol/security TCK：8 + 5 + 5 通过；channel adapter runtime/shared TCK：
  12 + 10 通过。
- `cargo run -p agent-diva-cli -- --help`：通过，输出完整 usage/commands，无 secret 或外网。

## Supervised run schema migration

`supervised_runs.context` 是向后兼容方向的 additive SQLite schema migration：启动时使用
`PRAGMA table_info(supervised_runs)` 检测列，不存在时才执行 `ALTER TABLE ... ADD COLUMN
context TEXT`。旧列和旧行不被重写；旧行的 `context` 仍为 `NULL`，因此需要 typed route 的
Subagent handler 会明确 fail-closed。新记录的 `SupervisedRunContext` JSON 序列化失败会传播为
存储错误，不会静默写入空字符串；tags 序列化也遵循同一错误策略。

回滚影响：旧二进制通常会忽略 SQLite 中新增的 nullable 列，已有旧数据可继续读取；但旧
运行时不具备 typed route 的 Subagent result correlation 能力，不能把回滚后的行为当成 C6-D
语义等价。迁移不会自动删除列，也不会恢复已删除的旧 DTO/queue API；恢复生产旧版本前必须
暂停依赖 typed context 的排队任务并按版本策略处理其 `NULL` context。

## Clean Break 静态证据

`just channel-clean-break-check` 覆盖 Cargo/just/CI/scripts 以及 core、agent、manager、tools、
tooling、channels、CLI、E2E、service、migration、GUI active roots，并跳过 `.git`、`target`、
依赖构建输出、历史/archive 文档和 checker 自身。随后使用同一排除规则执行递归 `rg` proof，
旧 DTO/API、旧 MessageBus、旧 sender/receiver、旧 callback 与旧 execution metadata token
均为零命中；手工命令输出 `active forbidden-token rg: zero hits`。历史 archive 中保留的迁移
证据不属于 active source，未被删除或机械改写。

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
