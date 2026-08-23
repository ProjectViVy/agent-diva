# v0.4.0 GUI/Manager 工作区一致性：总结

> 状态：骨架（Wave D 完成后补全）

## 交付范围

- Manager 状态端点返回 canonical workspace、来源、AGENTS 状态（存在/截断/digest/预算），
  正文不入普通日志。
- GUI `WorkspaceContext` store 作为唯一快照来源；移除 `GeneralSettings.vue` /
  `ProvidersSettings.vue` 重复的 `statusReport.config.workspace` 回显与重复
  `getConfig_status()` 请求。
- `NormalMode` Topbar 只读 `WorkspaceChip`；`SettingsView` 新增 Workspace 页面
  （canonical root、来源、AGENTS 摘要抽屉：来源/预算/digest/截断，只读不可编辑）。
- 目录选择走"停止 → 保存 → 重建 Gateway → 恢复会话"单一切换流程；流式回答/Plan
  执行/待审批时阻止切换；失败保留旧上下文并提供恢复动作；过期 inspect/status 响应丢弃。
- 设计依据：`docs/research/workspace-agents-diva-adaptation-2026-08/gui-workspace-agents-design.md`。
