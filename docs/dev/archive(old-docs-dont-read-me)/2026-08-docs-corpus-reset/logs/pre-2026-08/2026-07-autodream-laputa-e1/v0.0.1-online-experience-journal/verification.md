# 验证

- Core 契约测试覆盖确定性幂等、跨 workspace 拒绝、损坏行拒绝及物理 retention。
- AutoDream 输入测试覆盖 Experience Journal 优先级、digest/ID provenance 与无
  authority 写入。
- Agent tool-step 聚焦测试覆盖 executor admission 回归。
- `cargo test -p agent-diva-core experience --lib`：3 项通过。
- `cargo test -p agent-diva-autodream`：全部通过；新增 inputs 聚焦测试 5 项通过。
- `cargo test -p agent-diva-agent tool_step --lib`：3 项通过。
- `cargo clippy -p agent-diva-core -p agent-diva-agent -p agent-diva-autodream -- -D warnings`：
  通过。
- `just fmt-check`：通过。
- `just check`：通过。

`just test` 的全工作区门仍受 E0 已记录的运行中桌面二进制锁与 GUI test PDB
链接限制阻断；本切片没有把聚焦测试冒充全门。真实桌面测试按决策延后至 G2D+。
