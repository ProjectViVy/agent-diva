# Release

本更新仍仅存在于隔离 `feat/channel-epic` worktree，未合入 `dev`，未推送。Production
bootstrap 现在会安装 typed AgentLoop runtime；如果 control lane 已关闭，Gateway 返回
`service_unavailable`，不回退到旧 bus。

回滚为回退本更新提交；C3a/b/d 的协议与 journal 文件仍可独立保留。C3e 事件 hub、C4
GUI migration 与 C6 clean-break 继续按原子切换顺序执行。

