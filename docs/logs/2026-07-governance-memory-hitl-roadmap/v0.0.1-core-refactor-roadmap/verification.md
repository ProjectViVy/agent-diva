# Verification

## 验证范围

- 检查计划是否覆盖治理、Memory、Human-in-the-loop 三条主线及其交叉依赖。
- 检查是否绑定现有 `agent-diva-core`、`agent-diva-agent`、`agent-diva-laputa`、`agent-diva-sandbox`、`agent-diva-manager`、`agent-diva-gui` 边界。
- 检查是否包含失败关闭、审批内容哈希绑定、重启恢复、审计、迁移回滚、真实 smoke 和性能门槛。
- 检查 Markdown 格式与局部 diff。

## 结果

规划活动、阶段 Gate、里程碑、DoD 和兼容迁移要求均已写入 `TODOLIST.md`。本次为文档变更，不运行 Rust/GUI 测试；使用 `git diff --check` 作为最低验证。
