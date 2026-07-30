# E2 可恢复 AutoDream 编排器

本切片把 AutoDream 从同步请求内执行改为可持久化、可重启恢复的后台编排：

- 运行记录保存阶段、attempt、deadline 与更新时间。
- 普通反思阶段依次落盘为 gathering、reflecting、validating、publishing 和终态。
- Manager 创建任务后立即返回 queued，由后台执行；启动时重新派发可恢复任务。
- stale/异进程锁对应的非终态运行重新排队，不再直接宣告失败。
- 提案 ID 与输入 evidence ID 确定化；publishing 重放复用既有 artifact/proposal，
  内容冲突则 fail closed。
- 无编排元数据的旧版未完成运行不猜测恢复，使用 `legacy_incomplete` 失败码终止。

影响范围为 AutoDream 运行记录、worker/output、Manager 调度与启动恢复。未改变
Laputa 审批规则，也未开放自动批准或 Memory 直写。
