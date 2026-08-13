# 验收

自动验收标准：

1. 手动触发立即返回 pending/queued，Manager 请求不等待反思完成。
2. worker 开始后 attempt 增加，阶段按顺序落盘，成功或失败具有明确终态。
3. 中断运行在新 Manager 进程启动后被重新派发；当前活动运行不会重复派发。
4. publishing 阶段重放只保留一个确定性 proposal，返回相同 proposal ID。
5. 同 ID 但内容不同的已存 proposal 被判定为冲突，不覆盖原内容。
6. 取消和 deadline 超时不会更新成功 checkpoint。
7. 旧版不完整运行以 `legacy_incomplete` 终止，不静默跳过或推测恢复。

本切片不要求人工桌面验收。真实桌面只在 E0–E7 自动闭环与发布门全部完成后，
按 G2D+ 一次性执行。
