# 验收

自动验收标准：

1. 新候选只能形成 PendingReview proposal。
2. allow/deny 必须进入 Governance Ledger，普通 transition API 不得绕过。
3. edit 后 request ID 与 digest 更新，旧 allowed receipt 立即无效。
4. decision 后崩溃可由 retry 只补做 proposal transition。
5. deny 后 suppression 文件不含候选原文，重启后仍生效。
6. 同内容下一 run 不再生成 proposal，并记录 `suppressed`。
7. 显著变化后的内容可以重新提案。
8. suppression 超过 90 天失效，物理条目最多 1000。
9. 损坏 suppression 状态阻断候选发布，不静默忽略。

人工桌面验收继续推迟到 E7 后的 G2D+。
