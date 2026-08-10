# 上下文管理施工决策修订摘要

本修订把 C1-0 评审中提出的七项施工建议升级为 C1–C5 的冻结决策，消除 provider
wire role、会话快照、artifact 安全、动态挂载和缓存告警语义上的实现时歧义。

## 冻结内容

1. 动态 fragment 由 capability-aware serializer 映射 wire role，未知 provider 使用
   fail-safe user-context envelope。
2. C1 必须复用已实现的 C1-0 最小类型；`prefix_hash` 归 P0-4。
3. Stable/SessionStable 使用会话快照与显式失效矩阵，compact 不清 stable snapshot。
4. Tool schema 排序与 provider cache-control 分为 C1b/C1d 两个原子提交。
5. C3 artifact store 受隔离、retention、容量、脱敏、key、恢复和配对安全门约束。
6. C4 mount 在同一 turn 下一次 provider call 生效，并持续受策略过滤。
7. 缓存观测按 expected/policy/undeclared/suspected/deletion 分类并使用连续趋势。

本次仅修订文档与项目排期，不修改运行时代码。
