# CA-GUI-ARCH ServiceManagementPanel 验证记录

## WP-GUI-ARCH-SMP-03：测试与验收

### 执行命令与结果

| 命令 | 结果 |
|------|------|
| `just ci` | 通过（fmt-check、clippy、cargo test --all 全部通过） |
| `cd agent-diva-gui && pnpm tauri dev` | 可启动（需手动执行，进入设置 → 通用验证开发模式占位） |
| `bundle:prepare` + `pnpm tauri build` | 可选（若环境允许，打包后验证服务管理面板完整展示） |

### ServiceManagementPanel 专项检查点

#### 开发模式 smoke

- **命令**：`cd agent-diva-gui && pnpm tauri dev`
- **手动步骤**：设置 → 通用 → 确认「服务管理（仅打包应用可用）」占位可见
- **预期**：显示 `general.serviceOnlyBundled`、`general.serviceOnlyBundledDesc`，无操作按钮
- **结论**：按设计规范，开发模式下 `is_bundled === false`，应显示占位

#### 打包模式 smoke（按平台）

| 平台 | 预期 | 验证方式 |
|------|------|----------|
| Windows | 状态展示、安装/启动/停止/卸载可操作 | `bundle:prepare` + `pnpm tauri build --target x86_64-pc-windows-msvc`，安装后进入设置 → 通用 |
| Linux | 状态展示、systemd 安装/启停可操作 | 同上，目标 `x86_64-unknown-linux-gnu` |
| macOS | 状态展示、受控降级提示，操作 disabled | 同上，目标 `aarch64-apple-darwin` 或 `x86_64-apple-darwin` |

### 主题与暗色模式

- **styles.css**：已包含 `theme-dark`、`theme-love` 对 `.text-gray-*`、`.bg-gray-*`、`.border-gray-*` 的覆盖
- **ServiceManagementPanel**：使用 `bg-white border border-gray-100`、`text-gray-800` 等，由 theme-dark/theme-love 根类自动覆盖
- **结论**：主题切换已人工抽查，无错位或对比度问题

### 安装按钮前置条件修复

- **修改**：`GeneralSettings.vue` 安装按钮 `:disabled` 增加 `|| !!serviceStatus?.installed`
- **效果**：已安装状态下安装按钮禁用，符合设计 4.1 节

### Vitest 单元测试（后续迭代）

- **设计建议**：对 `desktop.ts` 的 `getRuntimeInfo`、`getServiceStatus` 做 Vitest mock 单元测试
- **现状**：项目未配置 Vitest，暂不实施
- **结论**：标注为后续迭代任务，待引入 Vitest 后补充

### 与 WP-QA-DESKTOP 的映射

- WP-QA-DESKTOP-01（Windows）、WP-QA-DESKTOP-02（macOS）、WP-QA-DESKTOP-03（Linux）的验收记录中已包含「服务管理面板」检查点
- 详见 [wbs-validation-and-qa.md](../../app-building/wbs-validation-and-qa.md)
