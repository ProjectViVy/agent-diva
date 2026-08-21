# 用户验收标准

1. Topbar、Workspace 设置页和 AGENTS 抽屉使用同一个已提交 `WorkspaceContext`，不会出现路径或状态不一致。
2. 选择目录后先显示候选预览；取消、inspect 失败或 apply 失败都不改变当前 workspace。
3. apply 只有一个主提交动作；processing 时确认面板不关闭，成功后才刷新 root 与 session。
4. AGENTS 的 `injected/truncated/missing/empty/unreadable` 与页面的 loading/refreshing/processing/error 互斥显示。
5. 过期 inspect/status 响应不会覆盖最新候选或当前快照。
6. 主页面只有一个滚动容器；Popover、Dialog、Drawer 不产生根页面横向滚动或无边界嵌套滚动。
7. `GeneralSettings` 不再维护 workspace 的第二个请求和第二套可写状态。
