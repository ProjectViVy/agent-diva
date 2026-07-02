# Epic 1 评审修复总结

## 范围

本轮迭代收口 Epic 1 代码评审补丁项，覆盖 Laputa 存储、提案 apply、rollback/events、Tauri 错误透传，以及 session save 耐久性。

## 变更

- Laputa 存储写入和 session save 在 atomic rename 后同步父目录。
- 收紧 stale lock recovery，带可解析 owner pid 的 lock 不再仅凭 mtime 删除。
- apply、rollback、proposal 状态写入统一使用 proposal 写锁。
- apply 失败补偿会清理 rollback、changelog、audit 已写 artifact。
- 显式 TBD 的 journal reflective authority 写入允许 raw bytes。
- rollback 增加当前内容校验、失败补偿，以及 30 天回滚边界。
- changelog diff 改为 unified-diff 形态。
- event persistence 改为加锁 append-only JSONL，并增加 replay / buffer_overflow 行为。
- Tauri Laputa 命令保留结构化错误 JSON。

## 影响

Epic 1 已在 sprint status 中标记为 done。剩余验证阻断是本轮范围外的预存 sandbox 编译问题。
