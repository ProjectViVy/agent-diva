# Release

本 slice 为库级能力新增与重构，无独立发布物、无 schema 变更。

## 部署方式

随 `agent-diva-sandbox` 常规 crate 发布。无运维干预。

## 说明

- 无停机、无迁移、无行为变更。
- 风险：`ToolOrchestrator` 构造器签名变化（`with_exec_policy`/`with_exec_policy_and_guardian`
  参数改 `Arc`），仅影响内部/test 调用方，无外部生产调用者。