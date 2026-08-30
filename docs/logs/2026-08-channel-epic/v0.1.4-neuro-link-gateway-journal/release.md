# Release

本迭代仅提交在隔离 worktree `feat/channel-epic`，未合入 `dev`，未推送远端，未改变
现有 GUI 实时链路和旧 Manager SSE。正式发布前仍需完成 C3c 的 AgentLoop/Fabric
production runtime wiring，并在 C6 统一执行 clean-break 原子切换。

回滚方式：回退本迭代提交；`super-channel-events.db` 是独立 projection 文件，可在确认
没有需要恢复的 Neuro-Link cursor 后删除，不影响 Session、BML 或 Laputa authority。

