# Acceptance

- 未设置 `AGENT_DIVA_EXTERNAL_GATEWAY` 时运行 `pnpm tauri dev`，Tauri 日志显示内嵌 Gateway
  已启动，工作区目录选择后可以进入原子切换流程。
- `just start` 和 `just make-diva` 只启动一个 Tauri 开发进程，不再弹出独立 Gateway 进程。
- 设置 `AGENT_DIVA_EXTERNAL_GATEWAY=1` 后仍可连接人工启动的外部 Gateway；此时切换工作区
  会明确提示取消该变量并重启 Tauri，而不是错误地要求重启外部 Gateway。
- 切换后工作区标签、`/api/workspace` 和当前 session 的 workspace 归属保持一致。
