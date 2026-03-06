# CA-GUI-CMDS 完成摘要

## 版本

v0.0.1-ca-gui-cmds-complete

## 变更范围

本次迭代完成 CA-GUI-CMDS（Tauri commands 与网关通信）的验证、缺口修复与验收闭环。

### 主要变更

1. **WP-CMDS-VERIFY**：验证现有 commands 完整性
   - 确认 lib.rs 中 CA-GUI-CMDS 相关 13 个 commands 已注册
   - 确认 desktop.ts 导出与 commands 一一对应
   - 确认 GeneralSettings.vue、ConsoleView.vue 调用正确
   - 执行 `just fmt-check`、`just check`、`just test` 均通过

2. **WP-CMDS-MACOS-RES**：macOS launchd 资源入包
   - 在 `scripts/ci/prepare_gui_bundle.py` 中新增 `target_os == "macos"` 分支
   - 将 `contrib/launchd` 复制到 `resources/launchd/`，与 Linux systemd 逻辑对称
   - 更新 `build_manifest` 的 `service_templates`，按 target_os 条件写入 `linux_systemd` 与 `macos_launchd`

3. **WP-CMDS-SMOKE**：桌面 smoke 验证
   - 开发模式：`pnpm tauri dev` 可启动
   - 中控台与设置页可访问，gateway 启停、配置、日志、服务状态 commands 可用

4. **WP-CMDS-DOC**：文档与优先级同步
   - 在 `docs/app-building/优先级.md` 中为 CA-GUI-CMDS 增加【已完成】标记

5. **WP-CMDS-LOG**：迭代日志
   - 创建本迭代目录及 summary.md、verification.md、acceptance.md

### 影响范围

- **修改文件**：`scripts/ci/prepare_gui_bundle.py`、`docs/app-building/优先级.md`
- **新增文件**：`docs/logs/2026-03-ca-gui-cmds/v0.0.1-ca-gui-cmds-complete/*.md`
- **未修改**：agent-diva-core、agent-diva-agent、agent-diva-providers、agent-diva-channels、agent-diva-tools、agent-diva-manager、agent-diva-cli 核心逻辑
