# Acceptance

## Product Acceptance Steps

> 本文档从“产品/交付视角”描述如何对 `v0.0.2-gui-bundle-foundation` 进行接受性检查。  
> 当前阶段用户明确表示将手动完成安装与功能测试，以下步骤作为检查清单使用。

1. **文档与配置对齐**
   - 打开 `docs/app-building/README.md`：
     - 确认文档中已将 `CA-GUI-BUNDLE` 描述为 GUI 打包与安装器的关键控制账户；
     - 确认其指向的 WBS 文档包含了 `WP-GUI-BUNDLE-00/01/02/...` 等细化工作包。
   - 打开 `docs/app-building/wbs-gui-cross-platform-app.md`：
     - 在 `CA-GUI-BUNDLE` 小节，确认已经明确了：
       - 构建前置校验（环境/锁文件）；
       - Tauri 打包配置（targets、icon、resources、Windows hooks）；
       - 图标生成与版本号统一策略；
       - 与 CI/QA 的输入输出映射。
   - 打开 `docs/app-building/wbs-distribution-and-installers.md` 与 `docs/windows-standalone-app-solution.md`：
     - 确认其中对 `productName`、`identifier`、`bundle.targets`、`bundle.resources` 的描述与当前 `agent-diva-gui/src-tauri/tauri.conf.json` 完全一致。

2. **打包与产物检查（Windows 环境，本迭代已完成一次，用户可按需重放）**
   - 在仓库根目录：
     - 运行 `cargo build -p agent-diva-cli --release`，确保 CLI release 二进制可生成；
   - 在 `agent-diva-gui` 目录：
     - 运行 `pnpm install --frozen-lockfile`，确认依赖安装无错误；
     - 运行 `pnpm tauri icon src-tauri/icons/icon-source.svg --output src-tauri/icons`，确认多平台图标生成成功；
     - 运行 `pnpm bundle:prepare`，确认 `src-tauri/resources/bin/windows/agent-diva.exe` 与 manifest 文件存在；
     - 运行 `pnpm tauri build --target x86_64-pc-windows-msvc`，确认完成 NSIS/MSI 安装包构建且无致命错误。

3. **安装包存在性检查（无需安装，仅看文件）**
   - 在 `target/x86_64-pc-windows-msvc/release/bundle/` 下确认存在：
     - `nsis/Agent Diva_0.1.0_x64-setup.exe`
     - `msi/Agent Diva_0.1.0_x64_en-US.msi`
   - 可选：记录安装包大小和时间戳，用于后续版本对比。

4. **GUI 与服务管理入口的“表层”检查（由用户后续手动执行）**
   - 从上述任一安装包安装 GUI（**本迭代不强制执行，仅建议**）；
   - 启动 GUI，进入设置/通用设置中的服务管理面板：
     - 确认面板能显示运行平台（Windows）与当前运行模式（是否打包）；
     - 确认安装/启动/停止/卸载等按钮与文案与 WBS 中定义的行为一致（功能细节可在后续阶段手动验证）。

## Acceptance Result

- **从代码与配置视角**：
  - `agent-diva-gui` 已具备在 Windows 上完成一次 Tauri GUI 安装包打包的能力；
  - 图标、bundle 配置、资源目录与服务管理 commands 与文档保持一致；
  - CLI `service` 子命令与 GUI 侧服务管理桥接在接口层契约对齐（`status --json`、`install --auto-start`、`start/stop/uninstall`）。

- **从产品交付视角（本迭代结论）**：
  - 当前版本可以被视为“Windows GUI 打包与服务管理桥接的基础版本（foundation）”，适合作为后续：
    - CI 发布（`CA-CI-ARTIFACTS`）、
    - 安装器增强（`CA-DIST-GUI-INSTALLER`）、
    - GUI/服务 smoke 测试（`CA-QA-SMOKE-DESKTOP` / `CA-QA-SMOKE-HEADLESS`）
    的上游输入。
  - GUI 行为与安装体验的系统化测试按用户要求留待后续手动/自动化阶段完成，本迭代不将其作为“必须通过”的 gate。
