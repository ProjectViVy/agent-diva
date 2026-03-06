# CA-GUI-CMDS 验证记录

## WP-CMDS-VERIFY：验证现有 commands 完整性

### 执行命令与结果

| 命令 | 结果 |
|------|------|
| `just fmt-check` | 通过 |
| `just check` | 通过（cargo clippy --all -- -D warnings） |
| `just test` | 通过（所有 crate 测试通过） |
| `pnpm tauri dev` | 可启动（agent-diva-gui 目录下执行） |

### commands 核对

- **lib.rs**：CA-GUI-CMDS 相关 13 个 commands 已注册
  - get_runtime_info, get_service_status, install_service, uninstall_service, start_service, stop_service
  - get_gateway_process_status, start_gateway, stop_gateway
  - load_config, save_config, tail_logs, check_health
- **desktop.ts**：导出与 commands 一一对应
- **GeneralSettings.vue**：服务管理面板调用 getRuntimeInfo、getServiceStatus、installService、uninstallService、startService、stopService
- **ConsoleView.vue**：gateway 启停、配置、日志调用 startGateway、stopGateway、getGatewayProcessStatus、loadRawConfig、saveRawConfig、tailLogs

## WP-CMDS-MACOS-RES：macOS launchd 资源入包

### 代码变更

- `scripts/ci/prepare_gui_bundle.py`：在 `target_os == "macos"` 时复制 `contrib/launchd` 到 `resources/launchd/`
- `build_manifest`：`service_templates` 按 target_os 条件写入

### 验证

- 在 Windows 上执行 `python scripts/ci/prepare_gui_bundle.py --gui-root agent-diva-gui`：成功，manifest 中 linux_systemd、macos_launchd 为 null（符合预期）
- 在 macOS 上执行 `bundle:prepare` 后，`resources/launchd/` 应存在 install.sh、uninstall.sh、plist（需 macOS 环境验证）

## WP-CMDS-SMOKE：桌面 smoke 验证

### 开发模式

- 命令：`cd agent-diva-gui && pnpm tauri dev`
- 结果：GUI 可启动，主窗口加载正常
- 观察点：中控台、设置页可访问；gateway 启停、配置、日志、服务状态 commands 可用（开发模式下服务管理显示“仅打包应用可用”）

### 打包模式

- 当前平台（Windows）：`bundle:prepare` + `pnpm tauri build` 可执行
- 安装后 smoke：需在目标平台执行 WP-QA-DESKTOP-01/02/03 相关步骤（人工或 CI 环境）
