//! Todo generation from plan steps.
//!
//! Maps [`PlanStep`] items into actionable [`TodoItem`] entries.

use agent_diva_core::planning::ids::TodoId;
use agent_diva_core::planning::model::{PlanStatus, PlanStep, TodoItem, TodoPriority, TodoStatus};
use chrono::Utc;

/// Stateless planner that converts plan steps into todo items.
pub struct TodoPlanner;

impl TodoPlanner {
    /// Generate a [`TodoItem`] for each [`PlanStep`].
    ///
    /// - `plan_step_id` is set to the step's id.
    /// - `detail` falls back from `rationale` to `expected_output`.
    /// - Status is mapped: Pending→Pending, InProgress→InProgress,
    ///   Blocked→Blocked, Completed→Completed, Failed→Canceled.
    /// - Priority defaults to `Normal`.
    pub fn generate_todos(steps: &[PlanStep]) -> Vec<TodoItem> {
        let now = Utc::now();
        steps
            .iter()
            .map(|step| TodoItem {
                id: TodoId::new(),
                plan_step_id: Some(step.id.clone()),
                title: step.title.clone(),
                detail: step
                    .rationale
                    .clone()
                    .or_else(|| step.expected_output.clone()),
                status: Self::map_status(&step.status),
                priority: TodoPriority::Normal,
                evidence_ref: step.evidence_ref.clone(),
                block_reason: None,
                updated_at: now,
            })
            .collect()
    }

    fn map_status(step_status: &PlanStatus) -> TodoStatus {
        match step_status {
            PlanStatus::Pending => TodoStatus::Pending,
            PlanStatus::InProgress => TodoStatus::InProgress,
            PlanStatus::Blocked => TodoStatus::Blocked,
            PlanStatus::Completed => TodoStatus::Completed,
            PlanStatus::Failed => TodoStatus::Canceled,
            PlanStatus::Partial | PlanStatus::Canceled => TodoStatus::Pending,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::planning::ids::PlanId;
    use agent_diva_core::planning::model::PlanStep;

    fn make_step(id: &str, plan_id: &PlanId, ordinal: i32, title: &str, status: PlanStatus) -> PlanStep {
        let now = Utc::now();
        PlanStep {
            id: id.to_string(),
            plan_id: plan_id.clone(),
            ordinal,
            title: title.to_string(),
            rationale: Some(format!("Rationale for {}", title)),
            expected_output: Some(format!("Output for {}", title)),
            status,
            evidence_ref: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn test_generate_todos_from_steps() {
        let plan_id = PlanId::new();
        let steps = vec![
            make_step("s1", &plan_id, 0, "Step One", PlanStatus::Pending),
            make_step("s2", &plan_id, 1, "Step Two", PlanStatus::InProgress),
            make_step("s3", &plan_id, 2, "Step Three", PlanStatus::Completed),
        ];

        let todos = TodoPlanner::generate_todos(&steps);
        assert_eq!(todos.len(), 3);

        assert_eq!(todos[0].title, "Step One");
        assert_eq!(todos[0].status, TodoStatus::Pending);

        assert_eq!(todos[1].title, "Step Two");
        assert_eq!(todos[1].status, TodoStatus::InProgress);

        assert_eq!(todos[2].title, "Step Three");
        assert_eq!(todos[2].status, TodoStatus::Completed);
    }

    #[test]
    fn test_generate_todos_empty() {
        let todos = TodoPlanner::generate_todos(&[]);
        assert!(todos.is_empty());
    }

    #[test]
    fn test_generate_todos_preserves_step_id() {
        let plan_id = PlanId::new();
        let steps = vec![
            make_step("step-alpha", &plan_id, 0, "Alpha", PlanStatus::Pending),
            make_step("step-beta", &plan_id, 1, "Beta", PlanStatus::Blocked),
        ];

        let todos = TodoPlanner::generate_todos(&steps);
        assert_eq!(todos[0].plan_step_id.as_deref(), Some("step-alpha"));
        assert_eq!(todos[1].plan_step_id.as_deref(), Some("step-beta"));
    }
}
