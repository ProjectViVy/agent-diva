# Oil Frontend 细化总结

本次按 `oil-frontend` 规则补充 Workspace/AGENTS GUI 方案，仍未修改生产代码。

新增约束：

- `WorkspaceContext` 是唯一业务快照，Rust/Gateway 为 root/source/AGENTS/runtime identity 权威；
- Topbar、Settings 和 AGENTS 抽屉共享同一状态，不允许 `GeneralSettings` 重复请求并各自缓存；
- 目录选择是候选预览流程，不是长期文本表单；一次“切换并重启”完成真实提交；
- inspect/apply/rescan 有明确 loading、refreshing、candidate、processing、error 状态，过期响应不能覆盖新候选；
- 弹层、确认弹窗、抽屉和主滚动容器各自负责边界；
- 组件落点、API 类型、数据流和组件/数据流/smoke 验收矩阵已明确。

详细文档：

`docs/research/workspace-agents-diva-adaptation-2026-08/gui-workspace-agents-design.md`
