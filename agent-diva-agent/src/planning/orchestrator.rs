//! Plan lifecycle orchestrator.
//!
//! Manages phase transitions through a well-defined state machine,
//! enforcing approval gates and todo-completion gates where required.

use std::collections::HashSet;

use agent_diva_core::planning::events::PlanEvent;
use agent_diva_core::planning::ids::PlanId;
use agent_diva_core::planning::model::{Plan, PlanPhase, PlanStatus, TodoStatus};
use agent_diva_core::planning::store::PlanningStore;
use agent_diva_core::Error;
use chrono::Utc;

/// In-memory orchestrator that drives plans through their lifecycle phases.
pub struct PlanOrchestrator {
    approved_plans: HashSet<PlanId>,
}

impl PlanOrchestrator {
    /// Create a new orchestrator with no approved plans.
    pub fn new() -> Self {
        Self {
            approved_plans: HashSet::new(),
        }
    }

    /// Mark a plan as approved, allowing it to transition to `Execute`.
    pub fn approve(&mut self, plan_id: &PlanId) {
        self.approved_plans.insert(plan_id.clone());
    }

    /// Check whether a plan has been approved.
    pub fn is_approved(&self, plan_id: &PlanId) -> bool {
        self.approved_plans.contains(plan_id)
    }

    /// Transition a plan to a new phase.
    ///
    /// Validates that the transition is legal, checks any required gates,
    /// updates the plan in the store, and emits lifecycle events.
    pub async fn transition_to(
        &mut self,
        store: &dyn PlanningStore,
        plan_id: &PlanId,
        new_phase: PlanPhase,
    ) -> Result<Plan, Error> {
        // 1. Get current plan
        let mut plan = store.get_plan(plan_id).await?;

        let current_phase = plan.phase.clone();

        // 2. Validate transition
        if !Self::is_valid_transition(&current_phase, &new_phase) {
            return Err(Error::Validation(format!(
                "Invalid transition: {} → {}",
                current_phase, new_phase
            )));
        }

        // 3. Check gates
        if new_phase == PlanPhase::Execute {
            self.check_execute_gate(plan_id).await?;
        }
        if new_phase == PlanPhase::Verify {
            self.check_verify_gate(store, plan_id).await?;
        }

        // 4. Determine new status
        let old_status = plan.status.clone();
        let new_status = match &new_phase {
            PlanPhase::Execute | PlanPhase::Verify => PlanStatus::InProgress,
            PlanPhase::Completed => PlanStatus::Completed,
            PlanPhase::Failed => PlanStatus::Failed,
            PlanPhase::Partial => PlanStatus::Partial,
            _ => plan.status.clone(), // Explore, Plan, AwaitingApproval keep current status
        };

        // 5. Update plan
        plan.phase = new_phase.clone();
        plan.status = new_status.clone();
        plan.updated_at = Utc::now();
        store.update_plan(&plan).await?;

        // 6. Append PhaseTransition event
        store
            .append_event(
                plan_id,
                &PlanEvent::PhaseTransition {
                    plan_id: plan_id.clone(),
                    from: current_phase,
                    to: new_phase,
                },
            )
            .await?;

        // 7. Append StatusChanged event if status changed
        if old_status != new_status {
            store
                .append_event(
                    plan_id,
                    &PlanEvent::StatusChanged {
                        plan_id: plan_id.clone(),
                        from: old_status,
                        to: new_status,
                    },
                )
                .await?;
        }

        Ok(plan)
    }

    /// Check whether a phase transition is valid.
    pub fn is_valid_transition(from: &PlanPhase, to: &PlanPhase) -> bool {
        // Any phase → Failed is always valid (emergency bail-out)
        if *to == PlanPhase::Failed {
            return true;
        }

        matches!(
            (from, to),
            (PlanPhase::Explore, PlanPhase::Plan)
                | (PlanPhase::Plan, PlanPhase::AwaitingApproval)
                | (PlanPhase::AwaitingApproval, PlanPhase::Execute)
                | (PlanPhase::Execute, PlanPhase::Verify)
                | (PlanPhase::Verify, PlanPhase::Completed)
                | (PlanPhase::Verify, PlanPhase::Failed)
                | (PlanPhase::Verify, PlanPhase::Partial)
        )
    }

    /// Gate: transitioning to Execute requires prior approval.
    async fn check_execute_gate(&self, plan_id: &PlanId) -> Result<(), Error> {
        if !self.is_approved(plan_id) {
            return Err(Error::Validation(
                "Plan requires approval before execution".to_string(),
            ));
        }
        Ok(())
    }

    /// Gate: transitioning to Verify requires all non-canceled todos to be
    /// Completed or Blocked.
    async fn check_verify_gate(
        &self,
        store: &dyn PlanningStore,
        plan_id: &PlanId,
    ) -> Result<(), Error> {
        let todo_list = store.get_todos(plan_id).await?;
        for todo in &todo_list.items {
            if todo.status == TodoStatus::Pending || todo.status == TodoStatus::InProgress {
                return Err(Error::Validation(
                    "All todos must be completed, blocked, or canceled before verification"
                        .to_string(),
                ));
            }
        }
        Ok(())
    }
}

impl Default for PlanOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use agent_diva_core::planning::ids::{PlanId, TodoId};
    use agent_diva_core::planning::model::{
        Plan, PlanPhase, PlanStatus, TodoItem, TodoPriority, TodoStatus,
    };
    use agent_diva_core::planning::store::SqlitePlanningStore;
    use chrono::Utc;
    use sqlx::SqlitePool;

    async fn make_store() -> SqlitePlanningStore {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        SqlitePlanningStore::new(pool).await.unwrap()
    }

    fn make_plan(id: &PlanId, title: &str) -> Plan {
        let now = Utc::now();
        Plan {
            id: id.clone(),
            title: title.to_string(),
            goal: format!("Goal for {}", title),
            phase: PlanPhase::Explore,
            status: PlanStatus::Pending,
            strategy: None,
            assumptions: vec![],
            risks: vec![],
            open_questions: vec![],
            verification_verdict: None,
            created_at: now,
            updated_at: now,
        }
    }

    async fn create_plan_in_store(store: &SqlitePlanningStore, plan: &Plan) {
        store.create_plan(plan).await.unwrap();
    }

    async fn add_todo(
        store: &SqlitePlanningStore,
        plan_id: &PlanId,
        status: TodoStatus,
    ) {
        let todo = TodoItem {
            id: TodoId::new(),
            plan_step_id: None,
            title: "Test todo".to_string(),
            detail: None,
            status,
            priority: TodoPriority::Normal,
            evidence_ref: None,
            block_reason: None,
            updated_at: Utc::now(),
        };
        store.create_todo(plan_id, &todo).await.unwrap();
    }

    #[tokio::test]
    async fn test_new_plan_starts_in_explore() {
        let store = make_store().await;
        let plan_id = PlanId::new();
        let plan = make_plan(&plan_id, "Test");
        create_plan_in_store(&store, &plan).await;

        let mut orch = PlanOrchestrator::new();
        let updated = orch
            .transition_to(&store, &plan_id, PlanPhase::Plan)
            .await
            .unwrap();

        assert_eq!(updated.phase, PlanPhase::Plan);
    }

    #[tokio::test]
    async fn test_valid_transition_chain() {
        let store = make_store().await;
        let plan_id = PlanId::new();
        let plan = make_plan(&plan_id, "Lifecycle Test");
        create_plan_in_store(&store, &plan).await;

        let mut orch = PlanOrchestrator::new();

        // Explore → Plan
        let p = orch
            .transition_to(&store, &plan_id, PlanPhase::Plan)
            .await
            .unwrap();
        assert_eq!(p.phase, PlanPhase::Plan);

        // Plan → AwaitingApproval
        let p = orch
            .transition_to(&store, &plan_id, PlanPhase::AwaitingApproval)
            .await
            .unwrap();
        assert_eq!(p.phase, PlanPhase::AwaitingApproval);

        // Approve
        orch.approve(&plan_id);

        // AwaitingApproval → Execute
        let p = orch
            .transition_to(&store, &plan_id, PlanPhase::Execute)
            .await
            .unwrap();
        assert_eq!(p.phase, PlanPhase::Execute);
        assert_eq!(p.status, PlanStatus::InProgress);

        // Add completed todos so Verify gate passes
        add_todo(&store, &plan_id, TodoStatus::Completed).await;

        // Execute → Verify
        let p = orch
            .transition_to(&store, &plan_id, PlanPhase::Verify)
            .await
            .unwrap();
        assert_eq!(p.phase, PlanPhase::Verify);

        // Verify → Completed
        let p = orch
            .transition_to(&store, &plan_id, PlanPhase::Completed)
            .await
            .unwrap();
        assert_eq!(p.phase, PlanPhase::Completed);
        assert_eq!(p.status, PlanStatus::Completed);
    }

    #[tokio::test]
    async fn test_invalid_transition_rejected() {
        let store = make_store().await;
        let plan_id = PlanId::new();
        let plan = make_plan(&plan_id, "Invalid Transition");
        create_plan_in_store(&store, &plan).await;

        let mut orch = PlanOrchestrator::new();
        let result = orch
            .transition_to(&store, &plan_id, PlanPhase::Execute)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Validation(msg) => assert!(msg.contains("Invalid transition")),
            other => panic!("Expected Validation error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_execute_requires_approval() {
        let store = make_store().await;
        let plan_id = PlanId::new();
        let plan = make_plan(&plan_id, "Needs Approval");
        create_plan_in_store(&store, &plan).await;

        let mut orch = PlanOrchestrator::new();

        // Explore → Plan → AwaitingApproval
        orch.transition_to(&store, &plan_id, PlanPhase::Plan)
            .await
            .unwrap();
        orch.transition_to(&store, &plan_id, PlanPhase::AwaitingApproval)
            .await
            .unwrap();

        // Try Execute without approval
        let result = orch
            .transition_to(&store, &plan_id, PlanPhase::Execute)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Validation(msg) => assert!(msg.contains("approval")),
            other => panic!("Expected Validation error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_verify_requires_all_todos_complete() {
        let store = make_store().await;
        let plan_id = PlanId::new();
        let plan = make_plan(&plan_id, "Todos Pending");
        create_plan_in_store(&store, &plan).await;

        // Progress to Execute
        let mut orch = PlanOrchestrator::new();
        orch.transition_to(&store, &plan_id, PlanPhase::Plan)
            .await
            .unwrap();
        orch.transition_to(&store, &plan_id, PlanPhase::AwaitingApproval)
            .await
            .unwrap();
        orch.approve(&plan_id);
        orch.transition_to(&store, &plan_id, PlanPhase::Execute)
            .await
            .unwrap();

        // Add a pending todo
        add_todo(&store, &plan_id, TodoStatus::Pending).await;

        // Try Verify
        let result = orch
            .transition_to(&store, &plan_id, PlanPhase::Verify)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Validation(msg) => assert!(msg.contains("completed, blocked, or canceled")),
            other => panic!("Expected Validation error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_verify_passes_with_all_completed() {
        let store = make_store().await;
        let plan_id = PlanId::new();
        let plan = make_plan(&plan_id, "All Done");
        create_plan_in_store(&store, &plan).await;

        let mut orch = PlanOrchestrator::new();
        orch.transition_to(&store, &plan_id, PlanPhase::Plan)
            .await
            .unwrap();
        orch.transition_to(&store, &plan_id, PlanPhase::AwaitingApproval)
            .await
            .unwrap();
        orch.approve(&plan_id);
        orch.transition_to(&store, &plan_id, PlanPhase::Execute)
            .await
            .unwrap();

        // All completed
        add_todo(&store, &plan_id, TodoStatus::Completed).await;
        add_todo(&store, &plan_id, TodoStatus::Completed).await;

        let result = orch
            .transition_to(&store, &plan_id, PlanPhase::Verify)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_verify_passes_with_blocked_todos() {
        let store = make_store().await;
        let plan_id = PlanId::new();
        let plan = make_plan(&plan_id, "Blocked Is OK");
        create_plan_in_store(&store, &plan).await;

        let mut orch = PlanOrchestrator::new();
        orch.transition_to(&store, &plan_id, PlanPhase::Plan)
            .await
            .unwrap();
        orch.transition_to(&store, &plan_id, PlanPhase::AwaitingApproval)
            .await
            .unwrap();
        orch.approve(&plan_id);
        orch.transition_to(&store, &plan_id, PlanPhase::Execute)
            .await
            .unwrap();

        // Mix of completed and blocked
        add_todo(&store, &plan_id, TodoStatus::Completed).await;
        add_todo(&store, &plan_id, TodoStatus::Blocked).await;

        let result = orch
            .transition_to(&store, &plan_id, PlanPhase::Verify)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_emergency_fail_from_any_phase() {
        let store = make_store().await;
        let plan_id = PlanId::new();
        let plan = make_plan(&plan_id, "Emergency");
        create_plan_in_store(&store, &plan).await;

        let mut orch = PlanOrchestrator::new();

        // Directly fail from Explore
        let p = orch
            .transition_to(&store, &plan_id, PlanPhase::Failed)
            .await
            .unwrap();
        assert_eq!(p.phase, PlanPhase::Failed);
        assert_eq!(p.status, PlanStatus::Failed);
    }

    #[tokio::test]
    async fn test_terminal_states_reject_transitions() {
        let store = make_store().await;
        let plan_id = PlanId::new();
        let plan = make_plan(&plan_id, "Terminal");
        create_plan_in_store(&store, &plan).await;

        let mut orch = PlanOrchestrator::new();

        // Explore → Plan → AwaitingApproval → Execute → Verify → Completed
        orch.transition_to(&store, &plan_id, PlanPhase::Plan)
            .await
            .unwrap();
        orch.transition_to(&store, &plan_id, PlanPhase::AwaitingApproval)
            .await
            .unwrap();
        orch.approve(&plan_id);
        orch.transition_to(&store, &plan_id, PlanPhase::Execute)
            .await
            .unwrap();
        add_todo(&store, &plan_id, TodoStatus::Completed).await;
        orch.transition_to(&store, &plan_id, PlanPhase::Verify)
            .await
            .unwrap();
        orch.transition_to(&store, &plan_id, PlanPhase::Completed)
            .await
            .unwrap();

        // Try transitioning from Completed → anything
        let result = orch
            .transition_to(&store, &plan_id, PlanPhase::Plan)
            .await;
        assert!(result.is_err());
    }
}
