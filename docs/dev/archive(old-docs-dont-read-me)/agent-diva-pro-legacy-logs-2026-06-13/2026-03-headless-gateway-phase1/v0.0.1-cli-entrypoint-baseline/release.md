## Release

### Release Type

- **内部迭代 / 文档+CLI 契约发布**（无新二进制分发包，仅更新工作方式与说明文档）。

### Deployment / Adoption

- 本次变更无需单独部署步骤，随正常编译即可生效：
  - 开发与 CI 环境中，`cargo run -p agent-diva-cli -- gateway run` 即可使用新入口；
  - 未来若打包 CLI 或 GUI 安装包，均可直接依赖该入口，不需额外开关。
- 对现有用户的行为影响：
  - 旧用法 `agent-diva gateway` 仍然有效（被兼容映射到 `run`），不会立刻破坏已有脚本；
  - 推荐在后续文档与脚本中逐步替换为 `agent-diva gateway run`，使语义更明确。

### Follow-up Release Suggestions

- 等到后续阶段完成以下能力时，可考虑统一对外发布一个“Headless + Service + 分发”版本：
  - `CA-HL-LNX-SYSTEMD`：提供标准 systemd unit 与安装脚本；
  - `CA-HL-WIN-SERVICE` / `CA-HL-MAC-LAUNCHD`：完成 Windows/macOS 服务化封装；
  - `CA-DIST-CLI-PACKAGE`：提供跨平台 CLI/服务分发包；
  - `CA-CI-MATRIX` + `CA-CI-SMOKE-HEADLESS`：在 CI 中持续验证 `gateway run` 服务形态。
- 届时在 Release Note 中可以直接引用本迭代作为“Headless 入口基线”，说明后续特性构建在同一入口上。

