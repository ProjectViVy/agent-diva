# 验收

自动验收标准：

1. 生产 AutoDream 使用配置解析出的 provider 与原始 model ID，且 tool choice 禁用。
2. 外发 Reflection JSON 不含既有 Memory 原文或检测到的 PII/API key。
3. provider 返回候选必须引用输入中的完整 evidence ref，不能只猜 ID。
4. artifact 保存非固定 proposal type 和完整 candidate metadata。
5. gate 对证据、重复、矛盾、scope、容量、敏感度、注入与敏感内容逐项给出稳定码。
6. gate rejection 日志无候选内容、Memory 原文或 provider payload。
7. provider 不可用不降级为模板候选；无候选不伪装成“生成一个提案”。
8. accepted candidate 只进入 PendingReview proposal，不写 typed Memory。

人工桌面验收继续推迟到 E7 自动闭环完成后的 G2D+。
