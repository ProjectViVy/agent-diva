# Acceptance

- [x] Exploring/Drafting 允许 Inspect + PlanningRecord，拒绝 exec/write。
- [x] AwaitingApproval 仅 Inspect。
- [x] Executing 允许 exec/write/external，拒绝 Unknown（由 core 矩阵覆盖）。
- [x] Verifying 允许 Inspect/PlanningRecord/Execute，拒绝 write。
- [x] Closed 全拒绝。
- [x] 非法迁移返回 `PlanningPolicyError::InvalidTransition`。
- [x] 执行 seam 二次拒绝，denied 工具不进入 registry executor。
- [x] 未重建旧 permission mode。
