# E1A 在线 Experience Journal

AgentLoop 在唯一工具执行边界之后写入 AutoDream Experience Journal。记录只包含
workspace/session/trace/action 关联、工具类别、结果类别、稳定摘要、时间与确定性
digest，不保存参数、完整工具输出、Memory 原文或 secret。

Journal 使用 workspace-local JSONL、文件锁、确定性幂等 ID 和物理 retention。
读取时拒绝损坏、外 workspace、错误 schema 和重复记录。AutoDream 输入收集现在优先
使用 `experience_journal`，再使用 session、Laputa 索引和 compaction 辅助证据。

本切片不改变 Memory authority，也不生成或批准新类型提案。
