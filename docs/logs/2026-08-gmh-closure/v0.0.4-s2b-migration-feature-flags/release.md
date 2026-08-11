# Release

本 slice 为迁移 CLI 的接口变更，无独立发布物。

## 部署方式

随 `agent-diva-migration` 常规 crate 发布。操作方从 `just migrate` 调用时需传入
`--features <name>` 才能执行对应 `Apply`。

## 说明

- 无停机、无数据库迁移。
- 行为变更：`Apply` 现在默认被 gate，需显式 `--features` 放行；这是有意为之的安全默认。
- 不引入 crate feature，不恢复 Mentle，不引入长期双写。