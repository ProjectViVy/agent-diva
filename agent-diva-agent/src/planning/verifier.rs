//! Plan verification and finalization.
//!
//! [`PlanVerifier`] inspects the todo items for a plan and derives a
//! [`VerificationVerdict`], then stores the verdict and emits
//! [`PlanEvent`]s.  The companion [`finalize`](PlanVerifier::finalize)
//! step maps the verdict to a terminal plan phase/status and clears the
//! active-plan slot so the next pending plan can take over.

use chrono::Utc;

use agent_diva_core::planning::events::PlanEvent;
use agent_diva_core::planning::ids::PlanId;
use agent_diva_core::planning::model::{
    Plan, PlanPhase, PlanStatus, TodoStatus, VerificationVerdict,
};
use agent_diva_core::planning::store::PlanningStore;
use agent_diva_core::Error;

/// Stateless helper for computing verification verdicts and finalizing plans.
pub struct PlanVerifier;

impl PlanVerifier {
    /// Inspect all non-canceled todos for `plan_id` and produce a
    /// [`VerificationVerdict`].
    ///
    /// The verdict is stored on the plan and a
    /// [`PlanEvent::VerificationRecorded`] event is appended before
    /// the result is returned.
    pub async fn verify(
        store: &dyn PlanningStore,
        plan_id: &PlanId,
    ) -> Result<VerificationVerdict, Error> {
        let todo_list = store.get_todos(plan_id).await?;

        let mut total = 0usize;
        let mut completed = 0usize;
        let mut blocked = 0usize;
        let mut pending = 0usize;
        let mut in_progress = 0usize;

        for todo in &todo_list.items {
            if todo.status == TodoStatus::Canceled {
                continue;
            }
            total += 1;
            match todo.status {
                TodoStatus::Completed => completed += 1,
                TodoStatus::Blocked => blocked += 1,
                TodoStatus::Pending => pending += 1,
                TodoStatus::InProgress => in_progress += 1,
                TodoStatus::Canceled => unreachable!(),
            }
        }

        let verdict = if total == 0 || completed == total {
            // All non-canceled items are completed (or no items at all).
            VerificationVerdict::Pass
        } else if pending > 0 || in_progress > 0 {
            // The verify-gate should have prevented this, but be safe.
            VerificationVerdict::Fail
        } else if blocked > 0 {
            VerificationVerdict::Partial
        } else {
            VerificationVerdict::Fail
        };

        // Persist the verdict on the plan.
        let mut plan = store.get_plan(plan_id).await?;
        plan.verification_verdict = Some(verdict.clone());
        plan.updated_at = Utc::now();
        store.update_plan(&plan).await?;

        // Append the event.
        store
            .append_event(
                plan_id,
                &PlanEvent::VerificationRecorded {
                    plan_id: plan_id.clone(),
                    verdict: verdict.clone(),
                },
            )
            .await?;

        Ok(verdict)
    }

    /// Finalize a plan based on its stored [`VerificationVerdict`].
    ///
    /// Maps the verdict to a terminal phase/status pair:
    ///
    /// | Verdict  | Phase       | Status      |
    /// |----------|-------------|-------------|
    /// | Pass     | Completed   | Completed   |
    /// | Fail     | Failed      | Failed      |
    /// | Partial  | Partial     | Partial     |
    /// | None     | *(error)*   | —           |
    ///
    /// After updating the plan, the method tries to clear the active-plan
    /// slot by promoting the next non-terminal plan (if any).
    pub async fn finalize(
        store: &dyn PlanningStore,
        plan_id: &PlanId,
    ) -> Result<Plan, Error> {
        let mut plan = store.get_plan(plan_id).await?;

        let (new_phase, new_status, terminal_event) = match &plan.verification_verdict {
            Some(VerificationVerdict::Pass) => (
                PlanPhase::Completed,
                PlanStatus::Completed,
                PlanEvent::Completed {
                    plan_id: plan_id.clone(),
                },
            ),
            Some(VerificationVerdict::Fail) => (
                PlanPhase::Failed,
                PlanStatus::Failed,
                PlanEvent::Failed {
                    plan_id: plan_id.clone(),
                    reason: "Verification failed".to_string(),
                },
            ),
            Some(VerificationVerdict::Partial) => (
                PlanPhase::Partial,
                PlanStatus::Partial,
                PlanEvent::Partial {
                    plan_id: plan_id.clone(),
                },
            ),
            None => {
                return Err(Error::Validation(
                    "Cannot finalize a plan that has not been verified".to_string(),
                ));
            }
        };

        plan.phase = new_phase;
        plan.status = new_status;
        plan.updated_at = Utc::now();
        store.update_plan(&plan).await?;

        // Emit terminal event.
        store.append_event(plan_id, &terminal_event).await?;

        // Try to promote the next non-terminal plan to active.
        Self::promote_next_active_plan(store).await?;

        Ok(plan)
    }

    /// Walk the plan list and set the first non-terminal plan as active.
    ///
    /// Terminal phases are `Completed`, `Failed`, and `Partial`.
    /// If no non-terminal plan exists the active slot is left as-is.
    async fn promote_next_active_plan(store: &dyn PlanningStore) -> Result<(), Error> {
        let plans = store.list_plans().await?;
        for p in &plans {
            if !Self::is_terminal_phase(&p.phase) {
                store.set_active_plan(&p.id).await?;
                return Ok(());
            }
        }
        Ok(())
    }

    fn is_terminal_phase(phase: &PlanPhase) -> bool {
        matches!(
            phase,
            PlanPhase::Completed | PlanPhase::Failed | PlanPhase::Partial
        )
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

    fn make_plan(id: &PlanId, title: &str, phase: PlanPhase, status: PlanStatus) -> Plan {
        let now = Utc::now();
        Plan {
            id: id.clone(),
            title: title.to_string(),
            goal: format!("Goal for {}", title),
            phase,
            status,
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
        store.set_active_plan(&plan.id).await.unwrap();
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

    // -----------------------------------------------------------------------
    // verify tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_verify_all_completed_returns_pass() {
        let store = make_store().await;
        let plan_id = PlanId::new();
        let plan = make_plan(&plan_id, "All Done", PlanPhase::Verify, PlanStatus::InProgress);
        create_plan_in_store(&store, &plan).await;

        add_todo(&store, &plan_id, TodoStatus::Completed).await;
        add_todo(&store, &plan_id, TodoStatus::Completed).await;

        let verdict = PlanVerifier::verify(&store, &plan_id).await.unwrap();
        assert_eq!(verdict, VerificationVerdict::Pass);

        // Verdict stored on plan.
        let plan = store.get_plan(&plan_id).await.unwrap();
        assert_eq!(plan.verification_verdict, Some(VerificationVerdict::Pass));

        // Event was appended.
        let events = store.get_events(&plan_id).await.unwrap();
        assert!(events.iter().any(|e| matches!(
            e,
            PlanEvent::VerificationRecorded {
                verdict: VerificationVerdict::Pass,
                ..
            }
        )));
    }

    #[tokio::test]
    async fn test_verify_blocked_returns_partial() {
        let store = make_store().await;
        let plan_id = PlanId::new();
        let plan = make_plan(&plan_id, "Blocked", PlanPhase::Verify, PlanStatus::InProgress);
        create_plan_in_store(&store, &plan).await;

        add_todo(&store, &plan_id, TodoStatus::Completed).await;
        add_todo(&store, &plan_id, TodoStatus::Blocked).await;

        let verdict = PlanVerifier::verify(&store, &plan_id).await.unwrap();
        assert_eq!(verdict, VerificationVerdict::Partial);

        let plan = store.get_plan(&plan_id).await.unwrap();
        assert_eq!(plan.verification_verdict, Some(VerificationVerdict::Partial));
    }

    #[tokio::test]
    async fn test_verify_mixed_returns_fail() {
        let store = make_store().await;
        let plan_id = PlanId::new();
        let plan = make_plan(&plan_id, "Mixed", PlanPhase::Verify, PlanStatus::InProgress);
        create_plan_in_store(&store, &plan).await;

        // Canceled items are ignored; remaining are pending → Fail.
        add_todo(&store, &plan_id, TodoStatus::Completed).await;
        add_todo(&store, &plan_id, TodoStatus::Pending).await;
        add_todo(&store, &plan_id, TodoStatus::Canceled).await;

        let verdict = PlanVerifier::verify(&store, &plan_id).await.unwrap();
        assert_eq!(verdict, VerificationVerdict::Fail);
    }

    // -----------------------------------------------------------------------
    // finalize tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_finalize_with_pass_verdict() {
        let store = make_store().await;
        let plan_id = PlanId::new();
        let plan = make_plan(&plan_id, "Pass Plan", PlanPhase::Verify, PlanStatus::InProgress);
        create_plan_in_store(&store, &plan).await;

        // Run verify to set the verdict.
        add_todo(&store, &plan_id, TodoStatus::Completed).await;
        PlanVerifier::verify(&store, &plan_id).await.unwrap();

        let finalized = PlanVerifier::finalize(&store, &plan_id).await.unwrap();
        assert_eq!(finalized.phase, PlanPhase::Completed);
        assert_eq!(finalized.status, PlanStatus::Completed);

        // Terminal event was emitted.
        let events = store.get_events(&plan_id).await.unwrap();
        assert!(events
            .iter()
            .any(|e| matches!(e, PlanEvent::Completed { .. })));
    }

    #[tokio::test]
    async fn test_finalize_with_fail_verdict() {
        let store = make_store().await;
        let plan_id = PlanId::new();
        let plan = make_plan(&plan_id, "Fail Plan", PlanPhase::Verify, PlanStatus::InProgress);
        create_plan_in_store(&store, &plan).await;

        // Add a pending todo so verify yields Fail.
        add_todo(&store, &plan_id, TodoStatus::Completed).await;
        add_todo(&store, &plan_id, TodoStatus::Pending).await;
        PlanVerifier::verify(&store, &plan_id).await.unwrap();

        let finalized = PlanVerifier::finalize(&store, &plan_id).await.unwrap();
        assert_eq!(finalized.phase, PlanPhase::Failed);
        assert_eq!(finalized.status, PlanStatus::Failed);

        let events = store.get_events(&plan_id).await.unwrap();
        assert!(events
            .iter()
            .any(|e| matches!(e, PlanEvent::Failed { .. })));
    }

    #[tokio::test]
    async fn test_finalize_clears_active_plan() {
        let store = make_store().await;

        // Plan A is about to be finalized.
        let plan_a_id = PlanId::new();
        let plan_a = make_plan(&plan_a_id, "Plan A", PlanPhase::Verify, PlanStatus::InProgress);
        create_plan_in_store(&store, &plan_a).await;

        // Plan B is still in Explore — it should become active after A finalizes.
        let plan_b_id = PlanId::new();
        let plan_b = make_plan(&plan_b_id, "Plan B", PlanPhase::Explore, PlanStatus::Pending);
        store.create_plan(&plan_b).await.unwrap();

        // Verify + finalize plan A.
        add_todo(&store, &plan_a_id, TodoStatus::Completed).await;
        PlanVerifier::verify(&store, &plan_a_id).await.unwrap();
        PlanVerifier::finalize(&store, &plan_a_id).await.unwrap();

        // Active plan should now be plan B.
        let active = store.get_active_plan().await.unwrap();
        assert_eq!(active, plan_b_id);
    }
}
