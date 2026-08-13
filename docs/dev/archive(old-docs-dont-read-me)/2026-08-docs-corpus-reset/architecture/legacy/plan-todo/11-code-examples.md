# 关键代码示例（设计草案）

以下是适配 Diva 的伪代码，不可直接复制到生产环境。

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanModeState { Exploring, Drafting, AwaitingApproval, Executing, Verifying, Closed }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCapability { Inspect, PlanningRecord, WorkItem, WorkspaceWrite, Execute, External }

pub fn allows(state: PlanModeState, cap: ToolCapability) -> bool {
    matches!((state, cap),
        (PlanModeState::Exploring | PlanModeState::Drafting, ToolCapability::Inspect | ToolCapability::PlanningRecord)
        | (PlanModeState::Executing, ToolCapability::Inspect | ToolCapability::WorkItem | ToolCapability::WorkspaceWrite | ToolCapability::Execute)
        | (PlanModeState::Verifying, ToolCapability::Inspect | ToolCapability::WorkItem | ToolCapability::Execute)
    )
}
```

```rust
pub async fn approve(plan: &mut Plan, expected_revision: i32, todo_policy: TodoPolicy) -> Result<(), PlanningError> {
    if plan.phase != PlanPhase::AwaitingApproval { return Err(PlanningError::InvalidTransition); }
    ensure_revision(plan, expected_revision)?;
    ensure_complete(plan)?;
    // 在同一 store 事务中写入 ApprovalReceipt，并在 policy 要求时生成执行 TODO。
    plan.phase = PlanPhase::Execute;
    Ok(())
}
```

现有 agent loop 已在工具调用前拒绝计划态不允许的工具，[loop_turn.rs](agent-diva-agent/src/agent_loop/loop_turn.rs:904)；改造应把 `is_plan_mode_allowed_tool` 替换为 capability 查表，而不是复制另一套白名单。
