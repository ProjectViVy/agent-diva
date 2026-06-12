## Acceptance

### Product / Architecture Acceptance Checklist

1. **Headless 入口契约**
   - [x] `agent-diva-cli` 暴露统一的 Headless 网关入口：`agent-diva gateway run`。
   - [x] 旧命令 `agent-diva gateway` 仍然可用，并在 CLI 代码中明确被映射到 `Run`，形成兼容层。
2. **文档与 WBS 一致性**
   - [x] `docs/app-building/wbs-headless-service-mode.md` 中 `CA-HL-CLI-GATEWAY` 的实现示例与实际 CLI 代码一致（包括 `Option<GatewayCmd>` 的兼容逻辑与示例命令）。
   - [x] `docs/app-building/README.md` 中已明确 Phase 1 先落地 `CA-HL-CLI-GATEWAY`，后续才进入 systemd / launchd / Windows Service 与分发。
   - [x] 顶层 `README.md` / `README.zh-CN.md` 的“启动网关”和 cron 说明统一为 `agent-diva gateway run`，并说明兼容旧命令。
   - [x] `docs/migration.md` 的 CLI 映射与 cron 说明已与上述约定保持一致。
   - [x] `docs/windows-standalone-app-solution.md` 中 Windows 独立 App 的推荐网关启动命令已更新为 `agent-diva gateway run`。
3. **运行与验证**
   - [x] `just check`（clippy）通过，未引入新的 warning 或 error。
   - [x] `just test`（`cargo test --all`）通过，全仓库测试绿灯。
   - [x] 使用 `cargo run -p agent-diva-cli -- gateway run` 完成一次 smoke，确认：
     - Gateway 正常启动；
     - API Server 监听 `127.0.0.1:3000`；
     - Channel/Agent Loop/Manager 正常初始化。
   - [ ] `just fmt-check`：目前因其他 crate 既有格式差异失败（与本次改动无关），在本迭代中仅记录，不强行修复。

### Acceptance Result

- 本迭代按照“最小侵入 + 能力可达”的原则，完成了 `CA-HL-CLI-GATEWAY` 的第一阶段目标：
  - 统一了 Headless 入口的命令契约；
  - 确保实现、文档与后续 WBS 链条之间没有概念偏差；
  - 验证了实际运行行为在现有代码基线上是健康的。
- 结论：**接受** 作为 Headless 构建路线的 Phase 1 基线，可在此基础上继续推进服务化（systemd/launchd/Windows Service）、分发与 CI/QA 相关 CA。

