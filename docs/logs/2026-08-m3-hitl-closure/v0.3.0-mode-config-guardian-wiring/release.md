# Release

本 slice 让 Guardian 接入生产 shell 路径，是行为变更。

## 部署方式

随 `agent-diva-sandbox` / `agent-diva-tools` 常规 crate 发布。GUI 用户切换三模式即生效。

## 说明

- 无停机、无数据库迁移。
- 风险（按计划缓解）：Guardian 仅 `approval_policy != Never` 时附加；`Never` bypass
  路径逐字节不变；S2 契约测试先行落地。
- 自动学习（信任模式 create_rule 落盘）在 S4 实现，本 slice 仅返回学习标记。