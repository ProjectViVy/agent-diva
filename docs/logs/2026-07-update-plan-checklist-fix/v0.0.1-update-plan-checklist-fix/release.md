# 发布说明

本次变更作为一个聚焦的 Rust/GUI 兼容性修复提交交付，不涉及数据库迁移或配置迁移。

发布后无需人工迁移：旧客户端或历史载荷中的 `Pending`、`InProgress`、`Completed` 仍可反序列化；新事件统一输出 snake_case 状态。

如需回滚，应整体回滚本次提交。回滚会恢复旧事件顺序和 GUI 定位逻辑，因此会重新引入 `update_plan` 后 typing/assistant 行无法收口的风险。
