## Release / Deployment Notes

> 当前版本 `v0.0.1-ca-dist-gui-installer` 主要是**研发基线**：补齐 GUI 安装器、Windows Service 封装与相关 WBS/CI/QA 契约。尚未绑定特定的对外 Release tag，可作为后续正式版本（如 `v0.2.x`）的输入。

### Release Type

- **Type**: Internal engineering baseline
- **Scope**:
  - GUI 安装包构建流程与资源准备脚本；
  - Windows Service 封装和 CLI/GUI 服务管理命令；
  - CI 构建矩阵与 QA smoke/回归 WBS 的输入输出关系。

### Deployment Method (建议路径)

1. **本地验证（开发者 / Agent）**
   - 在 Windows/macOS/Linux 上执行：
     - `cargo build -p agent-diva-cli --release`
     - `cargo build -p agent-diva-service --release`（Windows）
     - `python scripts/ci/prepare_gui_bundle.py --gui-root agent-diva-gui --target-os <windows|macos|linux>`
     - `cd agent-diva-gui && pnpm install --frozen-lockfile && pnpm tauri build -- --target <对应平台 target>`
   - 使用 WBS 中的 `WP-DIST-GUI-01/02/03/04` 与 `CA-QA-SMOKE-DESKTOP` 对照 smoke 步骤执行手工验证。

2. **CI 构建路径（推荐）**
   - 通过 `.github/workflows/ci.yml` 的 `gui-build` job 自动生成三平台 GUI artifacts：
     - `agent-diva-gui-windows-<arch>-<sha>`
     - `agent-diva-gui-macos-<arch>-<sha>`
     - `agent-diva-gui-linux-<arch>-<sha>`
   - 从 CI artifacts 下载对应平台安装包，在专用 VM 环境中执行 WBS 中的 smoke/QA 步骤。

3. **对外 Release（后续版本）**
   - 建议在后续迭代（如 `v0.0.2` 或 `v0.2.x`）中：
     - 完成 `CA-CI-ARTIFACTS` 定义的 Release workflow（`release-artifacts.yml`）；
     - 将 `dist/gui/**` 与 `dist/headless/**` 上传到 GitHub Releases；
     - 在 Release body 中引用本迭代的 `summary.md` / `verification.md` 关键结论。

### Rollback Considerations

- 本次改动主要集中在：
  - 新增 crate：`agent-diva-service`；
  - CLI service 子命令与 GUI Tauri commands；
  - CI `gui-build` job 中的构建与资源预处理步骤；
  - 文档与 WBS 更新。
- 若后续发现问题，需要临时“回退”这条链路，可以采用以下方式：
  - 在 `.github/workflows/ci.yml` 中临时禁用 `gui-build` job 中的 `prepare_gui_bundle` 步骤与 `agent-diva-service` 构建；
  - 在 GUI 侧临时隐藏 General 设置页中的服务管理面板（前端级别改动，不影响安装器）；
  - 保留 `agent-diva-service` crate 代码，但在对外 Release 前不把其二进制打入安装包。

### Known Limitations

- `agent-diva-service` 与 CLI `service` 子命令目前只实现 Windows 平台；
- Tauri hooks 仅在 NSIS 安装器路径中启用，MSI 路径仍依赖后续 WiX 配置；
- GUI 服务管理面板目前只支持本地 Windows Service 管理，不包含 Linux systemd / macOS launchd 集成（这部分由 Headless WBS 负责）。

