# 调试模式默认内嵌 Gateway

修复了工作区切换在开发构建中必然失败的生命周期策略冲突：调试构建此前强制使用外部
Gateway，但原子切换只允许内嵌 Gateway，导致重启外部进程也无法恢复。

- debug 与 release 现在都默认由 Tauri 启动、持有并停止内嵌 Gateway。
- `just start` 和 `just make-diva` 不再额外启动外部 Gateway。
- 需要独立调试 Gateway 时可显式设置 `AGENT_DIVA_EXTERNAL_GATEWAY=1`；该兼容模式仍拒绝
  应用内原子切换，并给出可执行的恢复提示。
- 生命周期判定改为可独立测试的纯函数，覆盖默认值、falsey 值和 truthy 兼容开关。

本记录取代 `v0.1.6-workspace-atomic-switch` 中“debug 固定使用外部 Gateway”的历史约束。
