# 验收步骤

1. 在仓库根目录执行 `just diva-gate`。
2. 等待网关进入 ready 状态。
3. 在同一终端连续按两次 Ctrl+C。
4. 使用 `Get-Process agent-diva,cargo -ErrorAction SilentlyContinue` 确认手动网关相关进程已退出。
5. 执行 `cargo build -p agent-diva-cli`，确认 `target\\debug\\agent-diva.exe` 不再因旧进程占用而无法替换。
6. 使用 `pnpm tauri dev` 启动 GUI，确认 Debug GUI 仍只启动前端，并连接由 `just diva-gate` 启动的后端。
