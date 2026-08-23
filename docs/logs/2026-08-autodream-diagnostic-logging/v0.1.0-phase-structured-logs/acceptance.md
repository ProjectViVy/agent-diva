# Acceptance

用户原话要求关掉 `AUTODREAM-DIAGNOSTIC-LOGGING`。关项条件是补齐阶段级
结构化日志，而不是改产品表或恢复已删除的 MemoryPatch 路径。

验收点：

1. 一次 S3 成功 run 能按 `run_id` 读到 orient/gather/consolidate/propose
   阶段事件，以及输入摘要。
2. Skill 候选闸门拒绝带稳定 `gate_code`；成功发出的 Skill 审查请求带
   `proposal_id`。
3. 输入缺失失败带 `failure_code=input_unavailable`。
4. 未写 REDLINE/DREAM/用户偏好，未恢复 MemoryPatch/SopCreate/Governance，
   未把 STM 改成提案。
5. 根 `TODOLIST.md` 已移出该条并写入完成归档。
