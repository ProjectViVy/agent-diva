# C1c Workspace Memory Epoch Verification

## 定向验证

- `cargo test -p agent-diva-laputa external_authority_refresh_updates_stale_provider_once -- --nocapture`：通过。
- `cargo test -p agent-diva-manager typed_commit_queues_workspace_memory_refresh_command -- --nocapture`：通过。
- `cargo check -p agent-diva-core -p agent-diva-laputa -p agent-diva-agent -p agent-diva-manager`：通过。
- `just check`：通过，workspace Clippy warnings denied。
- `cargo fmt --all -- --check`：通过（先执行标准格式化收敛两处排版差异）。

## 覆盖点

- 外部 Provider 写入后旧 Provider 能刷新 startup projection。
- 相同 authority revision 不重复递增 startup revision。
- Manager handler 能发送 workspace-scoped Runtime Control command。
- Typed authority 提交后刷新通知失败不会回滚已提交数据；重启 Provider 可从最新 BML revision 初始化。

## 工作区门禁

- `just fmt-check`：通过。
- `just test`：通过（`cargo test --all`，workspace tests 与 doctests 全绿）。
- `just ci`：通过；包含格式、workspace Clippy、全量测试、health benchmark、feature
  gates、BML boundary guard 与 Laputa clean-break gate。
- `cargo run -p agent-diva-cli -- --help`：通过，帮助页正常输出并退出 0。
