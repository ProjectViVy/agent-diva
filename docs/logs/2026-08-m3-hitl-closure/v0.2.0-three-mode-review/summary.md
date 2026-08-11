# Summary

M3 HITL 收尾 S2：让「智能/谨慎/信任」三模式在 Guardian 层产生真正有差异的审批行为
（消除 G1/G2/G4）。本 slice 的 Guardian 逻辑在 Guardian 接入生产前不生效（S3 接线），
因此无生产行为变更。

## 变更

- `agent-diva-sandbox/src/guardian.rs::DefaultGuardianReviewer::review`：
  - `can_skip → Defer` 短路收紧为**仅 `Never`** 触发（三生产模式不得因策略返回 Skip
    而跳过风险预判——G3 根因）。
  - `OnFailure`（智能）不再盲 Defer：known-safe/read-only 自动放行（受 config 门控），
    危险必问、未知询问。
  - 拆分 `OnRequest | UnlessTrusted`：`OnRequest`（谨慎）= known-safe/read-only 自动
    （strict 全关则一切皆问）+ 危险询问 + 未知询问；`UnlessTrusted`（信任）=
    known-safe/read-only 自动 + 危险询问 + **未知自动放行** `auto_approve(false,
    config.enable_auto_learning)`。

## 对计划的调整

- **未改动 `exec_policy.rs::get_approval_requirement` 的 `Decision::Allow` 无匹配分支**：
  该分支不可达（空策略 evaluate 结果为 `Prompt`，非 `Allow`），拆分会产生不可测死代码。
  "信任放行未知" 由 Guardian 层独自承担，语义一致。
- **orchestrator fallback `check_approval` 拆分推迟到 S3**：拆分会使信任模式在
  Guardian 接线前（S2 单独落地时）对危险命令也直接放行，属临时回归；S3 接线 Guardian
  后一并处理更安全。

## Impact

- 三模式行为差异在 Guardian 层落地（谨慎全问 / 智能只读放行+危险未知询问 /
  信任未知放行+危险询问+自动学习）。
- 生产行为不变（Guardian 仍未接线，S3 接线）。