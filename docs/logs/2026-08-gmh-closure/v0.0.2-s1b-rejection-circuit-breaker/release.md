# Release

本 slice 为库级原语 + agent 内部接线，无独立发布物、无外部服务依赖、无 schema 变更。

## 部署方式

随 `agent-diva-core` / `agent-diva-agent` 常规 crate 发布即可。新配置键默认值极大，
已设默认值，无需运维干预；显式配置 `rejection_circuit_window_secs` /
`rejection_circuit_threshold` 时按 JSON security 文件覆盖。

## 说明

- 无停机、无迁移、无 feature flag 切换。
- 默认行为与发布前完全一致（阈值 50 在正常使用中不会触发）。