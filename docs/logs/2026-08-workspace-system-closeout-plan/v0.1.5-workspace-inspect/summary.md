# WS-02：工作区候选预检

本阶段为 GUI 工作区设置接入目录选择与候选 draft：

- Tauri 提供原生目录选择、canonicalize、目录可读性、workspace ID 和 `AGENTS.md` 状态预检。
- 候选路径与当前 `WorkspaceContext` 分离；预检不会写入配置，也不会改变当前 runtime 或会话集合。
- Settings 页显示候选 canonical root、workspace ID、可读性和 AGENTS 摘要，并保留取消/失败状态。
- 连续预检使用 generation 保护，只接受最新选择的响应；较晚返回的旧响应不能覆盖新候选。

同时保留了后续 WS-03 原子事务所需的 runtime 生命周期状态入口；本阶段的确认页尚未把候选
标记为已切换。
