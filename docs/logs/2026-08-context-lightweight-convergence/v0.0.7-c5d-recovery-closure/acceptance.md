# Acceptance

1. 普通 turn 的 provider context 报告只暴露三个顶层预算 region。
2. checkpoint 存在时，其正文 token 会计入同一 context budget，而不会被当作普通 history。
3. Recall 超限被丢弃前，working memory/当前用户保持可见。
4. provider overflow 触发 reactive compact 时，完成工具组可折叠，未完成组和当前执行组保持完整。
5. reactive/provider/quality 失败后，旧 checkpoint、durable index 和原始消息均不前进或覆盖。
6. 重启、reset/delete 后只从 canonical checkpoint 与完整 transcript 恢复，不读取旧压缩状态。
