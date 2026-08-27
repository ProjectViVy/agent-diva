# Acceptance

- 启动 GUI 后，Topbar 显示当前工作区名称；点击后可查看完整 canonical root、来源和
  `AGENTS.md` 是否存在、摘要路径、digest、字符数及截断状态。
- Workspace Popover 和 Workspace Settings 均只读；刷新时显示 loading/refreshing 状态，
  失败时保留上一份有效快照并呈现错误，不会让旧响应覆盖较新的 workspace generation。
- 设置 Dashboard 可进入专用“工作区”页面；General Settings 不再重复渲染 workspace 路径。
- 当前阶段不存在路径编辑、热切换或隐式迁移入口，后续切换流程必须经过 WS-02/WS-03。
