//! Planning hooks for the agent loop.
//!
//! Provides thin wrappers that can be called from the agent loop's
//! turn initialization and lifecycle events to integrate with the
//! planning subsystem.

use agent_diva_core::planning::ids::PlanId;
use agent_diva_core::planning::store::PlanningStore;
use agent_diva_core::Error;

use super::context::PlanContextBuilder;

/// HOOK-1: Inject planning system message before the agent loop iteration starts.
///
/// Returns `Ok(Some(block))` when an active plan exists, `Ok(None)` otherwise.
pub async fn inject_plan_context(
    store: &dyn PlanningStore,
) -> Result<Option<String>, Error> {
    PlanContextBuilder::build_context(store).await
}

/// HOOK-2: Check whether a tool name belongs to the planning subsystem.
///
/// Returns `true` if `tool_name` is `"todo_write"` or `"todo_show"`.
pub fn is_planning_tool_call(tool_name: &str) -> bool {
    matches!(tool_name, "todo_write" | "todo_show")
}

/// HOOK-3: Called after a planning tool call completes successfully.
///
/// Currently a no-op stub; the actual store sync will be wired in when the
/// event-sourcing layer is implemented.
pub async fn on_planning_tool_complete(
    _store: &dyn PlanningStore,
    _plan_id: &PlanId,
) -> Result<(), Error> {
    // TODO: flush dirty writes from tool execution
    Ok(())
}

/// HOOK-4: Called when the session is about to be persisted.
///
/// Currently a no-op stub; will flush any dirty planning state.
pub async fn on_session_save(
    _store: &dyn PlanningStore,
) -> Result<(), Error> {
    // TODO: flush dirty planning writes
    Ok(())
}

/// HOOK-5: Check whether a message content string is planning-related.
///
/// Returns `true` if `content` starts with `"## Active Plan:"` or contains
/// `"You have pending TodoList items"`.
pub fn is_planning_message(content: &str) -> bool {
    content.starts_with("## Active Plan:")
        || content.contains("You have pending TodoList items")
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

    // -----------------------------------------------------------------------
    // HOOK-1 (existing)
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_inject_returns_none_when_no_plan() {
        let store = test_store().await;
        let result = inject_plan_context(&store).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_inject_returns_context_when_plan_active() {
        let store = test_store().await;
        let now = Utc::now();
        let plan = Plan {
            id: PlanId("p1".to_string()),
            title: "Test".to_string(),
            goal: "Goal".to_string(),
            phase: PlanPhase::Execute,
            status: PlanStatus::InProgress,
            strategy: None,
            assumptions: vec![],
            risks: vec![],
            open_questions: vec![],
            verification_verdict: None,
            created_at: now,
            updated_at: now,
        };
        store.create_plan(&plan).await.unwrap();
        store
            .set_active_plan(&PlanId("p1".to_string()))
            .await
            .unwrap();

        let result = inject_plan_context(&store).await.unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains("## Active Plan: Test"));
    }

    // -----------------------------------------------------------------------
    // HOOK-2: is_planning_tool_call
    // -----------------------------------------------------------------------

    #[test]
    fn test_is_planning_tool_call_todo_write() {
        assert!(is_planning_tool_call("todo_write"));
    }

    #[test]
    fn test_is_planning_tool_call_todo_show() {
        assert!(is_planning_tool_call("todo_show"));
    }

    #[test]
    fn test_is_planning_tool_call_non_planning() {
        assert!(!is_planning_tool_call("shell_exec"));
        assert!(!is_planning_tool_call("read_file"));
        assert!(!is_planning_tool_call(""));
    }

    // -----------------------------------------------------------------------
    // HOOK-3 / HOOK-4: no-op stubs compile and return Ok
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_on_planning_tool_complete_returns_ok() {
        let store = test_store().await;
        let plan_id = PlanId("p1".to_string());
        on_planning_tool_complete(&store, &plan_id)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_on_session_save_returns_ok() {
        let store = test_store().await;
        on_session_save(&store).await.unwrap();
    }

    // -----------------------------------------------------------------------
    // HOOK-5: is_planning_message
    // -----------------------------------------------------------------------

    #[test]
    fn test_is_planning_message_active_plan() {
        let content = "## Active Plan: Ship Feature X\nGoal: ...\n...";
        assert!(is_planning_message(content));
    }

    #[test]
    fn test_is_planning_message_nag() {
        let content = "You have pending TodoList items. Pick up the next one now.";
        assert!(is_planning_message(content));
    }

    #[test]
    fn test_is_planning_message_regular() {
        assert!(!is_planning_message("Here is the answer to your question."));
        assert!(!is_planning_message(""));
    }

    // -----------------------------------------------------------------------
    // Story 3.4: Integration — NagTracker full cycle
    // -----------------------------------------------------------------------

    use super::super::nag::NagTracker;

    #[tokio::test]
    async fn test_nag_integration_full_cycle() {
        // 1. Create a store with an active plan that has pending todos.
        let store = test_store().await;
        let now = Utc::now();
        let plan = Plan {
            id: PlanId("p1".to_string()),
            title: "Ship Feature X".to_string(),
            goal: "Get to production".to_string(),
            phase: PlanPhase::Execute,
            status: PlanStatus::InProgress,
            strategy: None,
            assumptions: vec![],
            risks: vec![],
            open_questions: vec![],
            verification_verdict: None,
            created_at: now,
            updated_at: now,
        };
        store.create_plan(&plan).await.unwrap();
        store
            .set_active_plan(&PlanId("p1".to_string()))
            .await
            .unwrap();

        store
            .create_todo(
                &PlanId("p1".to_string()),
                &TodoItem {
                    id: TodoId("t1".to_string()),
                    plan_step_id: None,
                    title: "Write tests".to_string(),
                    detail: None,
                    status: TodoStatus::Pending,
                    priority: TodoPriority::Normal,
                    evidence_ref: None,
                    block_reason: None,
                    updated_at: now,
                },
            )
            .await
            .unwrap();

        // 2. Simulate 3 turns without planning calls — nag should fire.
        let mut tracker = NagTracker::new();
        for _ in 0..3 {
            assert!(!tracker.should_nag());
            tracker.record_turn(false);
        }
        assert!(tracker.should_nag());
        assert_eq!(
            tracker.nag_message(),
            "You have pending TodoList items. Pick up the next one now."
        );

        // 3. Simulate a planning call — nag resets.
        tracker.record_turn(true);
        assert!(!tracker.should_nag());

        // 4. Confirm the store still has the active plan and context.
        let ctx = inject_plan_context(&store).await.unwrap();
        assert!(ctx.is_some());
        assert!(ctx.unwrap().contains("## Active Plan: Ship Feature X"));
    }
}
