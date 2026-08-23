# v0.4.0 GUI/Manager 工作区一致性：验收步骤

> 状态：骨架（Wave D 完成后补全）

1. 启动 GUI，确认 Topbar WorkspaceChip 显示当前 canonical workspace 与来源。
2. 打开 Settings → Workspace，确认展示 root、来源、AGENTS 状态（存在/截断/digest）。
3. 选择新目录并提交，确认走"停止 → 保存 → 重建 → 恢复"流程且会话恢复。
4. 在流式回答/待审批期间尝试切换，确认被阻止。
5. 模拟重建失败，确认保留旧 workspace 上下文且有恢复动作。
6. 确认 AGENTS 摘要只读，GUI 不提供编辑入口；外部 workspace 不被写入模板。
