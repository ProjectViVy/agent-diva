//! Plan context builder for system-prompt injection.
//!
//! [`PlanContextBuilder`] reads the active plan's state from a
//! [`PlanningStore`] and produces a compact markdown block (≤ 800 chars)
//! suitable for injection into the agent's system message.

use agent_diva_core::planning::model::{TodoPriority, TodoStatus};
use agent_diva_core::planning::store::PlanningStore;
use agent_diva_core::Error;

/// Builds a compact plan context block (≤ 800 chars) for system prompt injection.
pub struct PlanContextBuilder;

impl PlanContextBuilder {
    /// Build a context block from the active plan's state.
    ///
    /// Returns `Ok(None)` if no active plan exists.
    pub async fn build_context(
        store: &dyn PlanningStore,
    ) -> Result<Option<String>, Error> {
        let plan_id = match store.get_active_plan().await {
            Ok(id) => id,
            Err(Error::Internal(msg)) if msg == "No active plan" => return Ok(None),
            Err(e) => return Err(e),
        };

        let plan = store.get_plan(&plan_id).await?;
        let todo_list = store.get_todos(&plan_id).await?;

        // Filter to only active/pending/blocked items
        let active_todos: Vec<_> = todo_list
            .items
            .iter()
            .filter(|t| {
                matches!(
                    t.status,
                    TodoStatus::InProgress | TodoStatus::Pending | TodoStatus::Blocked
                )
            })
            .collect();

        let mut block = String::new();
        block.push_str(&format!("## Active Plan: {}\n", plan.title));
        block.push_str(&format!("Goal: {}\n", plan.goal));
        block.push_str(&format!("Phase: {}\n", plan.phase));

        if let Some(strategy) = &plan.strategy {
            block.push_str(&format!("Strategy: {}\n", strategy));
        }

        for todo in &active_todos {
            let marker = match todo.status {
                TodoStatus::InProgress => "▶",
                TodoStatus::Pending => "○",
                TodoStatus::Blocked => "✗",
                _ => continue,
            };
            let priority = match todo.priority {
                TodoPriority::High => " [HIGH]",
                TodoPriority::Low => " [LOW]",
                _ => "",
            };
            block.push_str(&format!("{} {}{}", marker, todo.title, priority));
            if let Some(reason) = &todo.block_reason {
                block.push_str(&format!(" (blocked: {})", reason));
            }
            block.push('\n');
        }

        // Truncate to 800 chars
        if block.len() > 800 {
            block.truncate(797);
            block.push_str("...");
        }

        Ok(Some(block))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::planning::ids::{PlanId, TodoId};
    use agent_diva_core::planning::model::*;
    use chrono::Utc;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn test_store() -> agent_diva_core::planning::store::SqlitePlanningStore {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        agent_diva_core::planning::store::SqlitePlanningStore::new(pool)
            .await
            .unwrap()
    }

    fn make_plan(id: &str, title: &str, goal: &str) -> Plan {
        let now = Utc::now();
        Plan {
            id: PlanId(id.to_string()),
            title: title.to_string(),
            goal: goal.to_string(),
            phase: PlanPhase::Execute,
            status: PlanStatus::InProgress,
            strategy: Some("Iterative approach".to_string()),
            assumptions: vec![],
            risks: vec![],
            open_questions: vec![],
            verification_verdict: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn make_todo(
        id: &str,
        title: &str,
        status: TodoStatus,
        priority: TodoPriority,
        block_reason: Option<String>,
    ) -> TodoItem {
        TodoItem {
            id: TodoId(id.to_string()),
            plan_step_id: None,
            title: title.to_string(),
            detail: None,
            status,
            priority,
            evidence_ref: None,
            block_reason,
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_build_context_with_active_plan() {
        let store = test_store().await;
        let plan = make_plan("p1", "Ship Feature X", "Get feature X to production");
        store.create_plan(&plan).await.unwrap();
        store
            .set_active_plan(&PlanId("p1".to_string()))
            .await
            .unwrap();

        store
            .create_todo(
                &PlanId("p1".to_string()),
                &make_todo("t1", "Write tests", TodoStatus::InProgress, TodoPriority::High, None),
            )
            .await
            .unwrap();
        store
            .create_todo(
                &PlanId("p1".to_string()),
                &make_todo("t2", "Deploy to staging", TodoStatus::Pending, TodoPriority::Normal, None),
            )
            .await
            .unwrap();

        let ctx = PlanContextBuilder::build_context(&store).await.unwrap();
        assert!(ctx.is_some());

        let block = ctx.unwrap();
        assert!(block.contains("## Active Plan: Ship Feature X"));
        assert!(block.contains("Goal: Get feature X to production"));
        assert!(block.contains("Phase: Execute"));
        assert!(block.contains("Strategy: Iterative approach"));
        assert!(block.contains("▶ Write tests"));
        assert!(block.contains("[HIGH]"));
        assert!(block.contains("○ Deploy to staging"));
    }

    #[tokio::test]
    async fn test_build_context_no_active_plan() {
        let store = test_store().await;
        let ctx = PlanContextBuilder::build_context(&store).await.unwrap();
        assert!(ctx.is_none());
    }

    #[tokio::test]
    async fn test_build_context_truncates_to_800_chars() {
        let store = test_store().await;
        let plan = make_plan("p1", "Big Plan", "A very large plan with many todos");
        store.create_plan(&plan).await.unwrap();
        store
            .set_active_plan(&PlanId("p1".to_string()))
            .await
            .unwrap();

        // Create many todos to exceed 800 chars
        for i in 0..50 {
            store
                .create_todo(
                    &PlanId("p1".to_string()),
                    &make_todo(
                        &format!("t{}", i),
                        &format!("This is a fairly long todo item title number {} that takes up space", i),
                        TodoStatus::Pending,
                        TodoPriority::Normal,
                        None,
                    ),
                )
                .await
                .unwrap();
        }

        let ctx = PlanContextBuilder::build_context(&store)
            .await
            .unwrap()
            .unwrap();
        assert!(
            ctx.len() <= 800,
            "Context block was {} chars, expected ≤ 800",
            ctx.len()
        );
        assert!(ctx.ends_with("..."));
    }

    #[tokio::test]
    async fn test_build_context_excludes_completed() {
        let store = test_store().await;
        let plan = make_plan("p1", "Test Plan", "Test goal");
        store.create_plan(&plan).await.unwrap();
        store
            .set_active_plan(&PlanId("p1".to_string()))
            .await
            .unwrap();

        store
            .create_todo(
                &PlanId("p1".to_string()),
                &make_todo("t1", "Active task", TodoStatus::InProgress, TodoPriority::Normal, None),
            )
            .await
            .unwrap();
        store
            .create_todo(
                &PlanId("p1".to_string()),
                &make_todo("t2", "Completed task", TodoStatus::Completed, TodoPriority::Normal, None),
            )
            .await
            .unwrap();
        store
            .create_todo(
                &PlanId("p1".to_string()),
                &make_todo("t3", "Canceled task", TodoStatus::Canceled, TodoPriority::Normal, None),
            )
            .await
            .unwrap();
        store
            .create_todo(
                &PlanId("p1".to_string()),
                &make_todo(
                    "t4",
                    "Blocked task",
                    TodoStatus::Blocked,
                    TodoPriority::Low,
                    Some("waiting on API".to_string()),
                ),
            )
            .await
            .unwrap();

        let ctx = PlanContextBuilder::build_context(&store)
            .await
            .unwrap()
            .unwrap();

        // Active items should be present
        assert!(ctx.contains("▶ Active task"));
        assert!(ctx.contains("✗ Blocked task"));
        assert!(ctx.contains("(blocked: waiting on API)"));
        assert!(ctx.contains("[LOW]"));

        // Completed/canceled should NOT be present
        assert!(!ctx.contains("Completed task"));
        assert!(!ctx.contains("Canceled task"));
    }
}
