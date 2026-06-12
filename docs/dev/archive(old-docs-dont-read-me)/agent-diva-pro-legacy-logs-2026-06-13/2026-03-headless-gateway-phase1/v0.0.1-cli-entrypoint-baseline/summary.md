## Summary

- **Iteration**: `2026-03-headless-gateway-phase1`
- **Version**: `v0.0.1-cli-entrypoint-baseline`
- **Scope**: 第一阶段仅聚焦 `CA-HL-CLI-GATEWAY`，将网关 Headless 入口统一为 `agent-diva gateway run`，并与现有实现、WBS 与总览文档对齐。

### What Changed

- **CLI 入口收敛**
  - 在 `agent-diva-cli` 中将 `Commands::Gateway` 从单一枚举值调整为带子命令的结构：

```12:54:agent-diva-cli/src/main.rs
#[derive(Subcommand)]
#[command(rename_all = "kebab-case")]
enum Commands {
    /// Initialize agent-diva configuration
    Onboard,
    /// Run and manage the agent gateway
    Gateway {
        #[command(subcommand)]
        command: Option<GatewayCommands>,
    },
    // ...
}
```

- 新增 `GatewayCommands::Run`，并在 `main` 的分发逻辑中：
  - 将无子命令（`agent-diva gateway`）兼容映射到 `Run`；
  - 将 `Run` 分支直接路由到现有的 `run_gateway(&config_loader)`，不修改原有网关启动/关闭流程。

- **文档与 WBS 对齐**
  - 在 `docs/app-building/wbs-headless-service-mode.md` 中更新 `CA-HL-CLI-GATEWAY` 下的实现示例，使之与当前 CLI 代码一致（含 `Option<GatewayCmd>` 兼容逻辑）。
  - 在 `docs/app-building/README.md` 中新增“阶段建议”小节，明确 Phase 1 优先落地 `CA-HL-CLI-GATEWAY`，先统一 `agent-diva gateway run`，再推进服务化与分发。
  - 在顶层 `README.md` 与 `README.zh-CN.md` 中：
    - 将“启动网关”示例统一为 `agent-diva gateway run`；
    - 将 cron 说明改为以 `gateway run` 为主，同时注明 `agent-diva gateway` 作为兼容别名仍然有效。
  - 在 `docs/migration.md` 中同步变更：
    - 将从 Python 到 Rust 的 CLI 映射更新为 `agent-diva serve -> agent-diva gateway run`；
    - 说明 Rust 版 cron 在 `agent-diva gateway run` 运行期间自动触发。
  - 在 `docs/windows-standalone-app-solution.md` 中，将 GUI 模式 A 启动网关的建议命令更新为 `agent-diva gateway run`。

### Why It Matters

- 第一阶段在不触碰核心业务/架构的前提下，给 Headless 网关提供了：
  - **单一、稳定的入口契约**：`agent-diva gateway run`；
  - **兼容旧用法的平滑过渡**：仍接受 `agent-diva gateway`；
  - **与后续 WBS 的契约对齐**：systemd / launchd / Windows Service、GUI 控制面和分发文档都可以直接依赖该入口。
- 这为后续 `CA-HL-LNX-SYSTEMD`、`CA-HL-WIN-SERVICE`、`CA-DIST-CLI-PACKAGE` 与 `CA-CI-MATRIX` 提供了共同的“可运行 + 可文档化”的起点。

