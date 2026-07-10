# 网关退出残留修复

## 变更

- 为手动 `gateway run` 的后台任务加入有界关闭等待。
- 优雅关闭超时后继续执行 abort 和有限等待，避免单个模块卡住整个进程。
- 保持 `pnpm tauri dev` Debug 模式不启动内嵌后端。

## 影响

Windows 开发模式下，`just diva-gate` 收到 Ctrl+C 后可以从卡住的 shutdown 阶段退出，减少 `agent-diva.exe` 和 Cargo 残留导致的文件占用。
