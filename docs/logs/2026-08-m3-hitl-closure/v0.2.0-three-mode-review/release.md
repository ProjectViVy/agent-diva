# Release

本 slice 为 Guardian 层逻辑修正，无独立发布物。

## 部署方式

随 `agent-diva-sandbox` 常规 crate 发布。本 slice 逻辑在 Guardian 接入生产前不生效。

## 说明

- 无停机、无迁移。
- 风险：S3 接线后此逻辑对所有命令生效（爆炸半径）；已通过拆片（S3 独立接线 + 契约测试）
  与 `approval_policy != Never` 门控缓解。