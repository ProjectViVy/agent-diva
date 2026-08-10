# 验证

- 检查 README、C0 总论和 C1 专章均可追踪七项冻结决策。
- 检查 C1 PR 拆分明确 tool 排序先于且独立于 cache-control。
- 检查快照矩阵覆盖 Frozen Core、AGENTS.md、Skills、Mask、L1、Compact、Session end。
- 检查 C3 安全契约覆盖隔离、key、容量、retention、脱敏、恢复、错误与 tool-call 配对。
- 检查 C4 明确 same-turn 下一次 provider call 生效与 policy 重检。
- 检查缓存告警不再以单次 `cache_read` 下降或 declared break 作为 warn 条件。
- `git diff --check`：通过。

本次没有代码或用户可见行为变化，因此 Rust 构建、测试与 GUI/CLI smoke 不适用。
