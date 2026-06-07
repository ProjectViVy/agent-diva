//! Planning tools for agent-diva.
//!
//! Provides read-only tools that expose plan and todo state to the agent,
//! plus lifecycle tools for plan creation, approval, and phase transitions.

use agent_diva_core::planning::events::TodoEvent;
use agent_diva_core::planning::ids::{PlanId, TodoId};
use agent_diva_core::planning::model::{
    Plan, PlanPhase, PlanStatus, TodoItem, TodoList, TodoPriority, TodoStatus,
};
use agent_diva_core::planning::render::render_todo_md;
use agent_diva_core::planning::store::PlanningStore;
use agent_diva_core::Error;
use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

/// Return the current TodoList as formatted markdown text.
///
/// Reads the active plan's todos from the store and renders them
/// using [`render_todo_md`]. Returns an error when no plan is active.
pub async fn todo_show(store: &dyn PlanningStore) -> Result<String, Error> {
    let plan_id = store.get_active_plan().await?;
    let todo_list = store.get_todos(&plan_id).await?;
    let plan = store.get_plan(&plan_id).await?;

    // Build a TodoList with header context for rendering
    let list = TodoList {
        plan_id: plan_id.clone(),
        revision: todo_list.revision,
        items: todo_list.items,
        created_at: todo_list.created_at,
        updated_at: todo_list.updated_at,
    };

    let rendered = render_todo_md(&list);

    // Prepend plan context
    let mut output = format!("# Todo List — {}\n\n", plan.title);
    output.push_str(&rendered);

    Ok(output)
}

// ---------------------------------------------------------------------------
// todo_write: full-replace tool
// ---------------------------------------------------------------------------

/// JSON deserialization struct for `todo_write` input items.
#[derive(Debug, Deserialize)]
struct TodoWriteItem {
    title: String,
    #[serde(default = "default_status")]
    status: String,
    #[serde(default = "default_priority")]
    priority: String,
    #[serde(default)]
    detail: Option<String>,
    #[serde(default)]
    block_reason: Option<String>,
    #[serde(default)]
    evidence_ref: Option<String>,
}

fn default_status() -> String {
    "pending".to_string()
}

fn default_priority() -> String {
    "normal".to_string()
}

fn parse_todo_status(s: &str) -> Result<TodoStatus, Error> {
    match s {
        "pending" => Ok(TodoStatus::Pending),
        "in_progress" => Ok(TodoStatus::InProgress),
        "blocked" => Ok(TodoStatus::Blocked),
        "completed" => Ok(TodoStatus::Completed),
        "canceled" => Ok(TodoStatus::Canceled),
        _ => Err(Error::Validation(format!("Invalid todo status: {}", s))),
    }
}

fn parse_todo_priority(s: &str) -> Result<TodoPriority, Error> {
    match s {
        "low" => Ok(TodoPriority::Low),
        "normal" => Ok(TodoPriority::Normal),
        "high" => Ok(TodoPriority::High),
        _ => Err(Error::Validation(format!(
            "Invalid todo priority: {}",
            s
        ))),
    }
}

/// Full-replace write of the active plan's TodoList.
///
/// Parses `items_json` as a JSON array of todo entries, validates each one,
/// deletes all existing todos for the active plan, creates new ones, appends
/// a `TodoEvent::Revised` event, and returns the rendered markdown.
pub async fn todo_write(store: &dyn PlanningStore, items_json: &str) -> Result<String, Error> {
    // Parse input JSON
    let write_items: Vec<TodoWriteItem> = serde_json::from_str(items_json)?;

    // Validate all items upfront
    for item in &write_items {
        if item.title.trim().is_empty() {
            return Err(Error::Validation("Todo title cannot be empty".to_string()));
        }
        parse_todo_status(&item.status)?;
        parse_todo_priority(&item.priority)?;
        if item.status == "blocked" && item.block_reason.is_none() {
            return Err(Error::Validation(
                "Blocked items must have block_reason".to_string(),
            ));
        }
    }

    // Get the active plan
    let plan_id = store.get_active_plan().await?;

    // Get current revision before deleting
    let current_list = store.get_todos(&plan_id).await?;
    let new_revision = current_list.revision + 1;

    // Delete all existing todos
    store.delete_todos(&plan_id).await?;

    // Create new todo items
    let now = Utc::now();
    for item in &write_items {
        let todo = TodoItem {
            id: TodoId::new(),
            plan_step_id: None,
            title: item.title.clone(),
            detail: item.detail.clone(),
            status: parse_todo_status(&item.status)?,
            priority: parse_todo_priority(&item.priority)?,
            evidence_ref: item.evidence_ref.clone(),
            block_reason: item.block_reason.clone(),
            updated_at: now,
        };
        store.create_todo(&plan_id, &todo).await?;
    }

    // Append Revised event
    store
        .append_todo_event(
            &plan_id,
            &TodoEvent::Revised {
                plan_id: plan_id.clone(),
                revision: new_revision,
            },
        )
        .await?;

    // Get updated todos and render
    let updated_list = store.get_todos(&plan_id).await?;
    let plan = store.get_plan(&plan_id).await?;

    let list = TodoList {
        plan_id: plan_id.clone(),
        revision: updated_list.revision,
        items: updated_list.items,
        created_at: updated_list.created_at,
        updated_at: updated_list.updated_at,
    };

    let rendered = render_todo_md(&list);

    let mut output = format!("# Todo List — {}\n\n", plan.title);
    output.push_str(&rendered);

    Ok(output)
}

// ---------------------------------------------------------------------------
// Tool trait implementations
// ---------------------------------------------------------------------------

fn core_err_to_tool(e: Error) -> crate::base::ToolError {
    crate::base::ToolError::ExecutionFailed(e.to_string())
}

/// `todo_show` — read-only tool that returns the current TodoList as markdown.
pub struct TodoShowTool {
    store: Arc<dyn PlanningStore>,
}

impl TodoShowTool {
    pub fn new(store: Arc<dyn PlanningStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl crate::base::Tool for TodoShowTool {
    fn name(&self) -> &str {
        "todo_show"
    }

    fn description(&self) -> &str {
        "Show the current TodoList for the active plan as formatted markdown."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _args: Value) -> crate::base::Result<String> {
        todo_show(self.store.as_ref()).await.map_err(core_err_to_tool)
    }
}

/// `todo_write` — full-replace tool that creates/revises the entire TodoList.
pub struct TodoWriteTool {
    store: Arc<dyn PlanningStore>,
}

impl TodoWriteTool {
    pub fn new(store: Arc<dyn PlanningStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl crate::base::Tool for TodoWriteTool {
    fn name(&self) -> &str {
        "todo_write"
    }

    fn description(&self) -> &str {
        "Create or revise the entire TodoList for the active plan. Pass a JSON array of items with fields: title (required), status, priority, detail, block_reason, evidence_ref."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "items": {
                    "type": "array",
                    "description": "Full list of todo items to replace the current list.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "title": { "type": "string" },
                            "status": { "type": "string", "enum": ["pending", "in_progress", "blocked", "completed", "canceled"] },
                            "priority": { "type": "string", "enum": ["low", "normal", "high"] },
                            "detail": { "type": "string" },
                            "block_reason": { "type": "string" },
                            "evidence_ref": { "type": "string" }
                        },
                        "required": ["title"]
                    }
                }
            },
            "required": ["items"]
        })
    }

    async fn execute(&self, args: Value) -> crate::base::Result<String> {
        let items = args
            .get("items")
            .and_then(|v| serde_json::to_string(v).ok())
            .ok_or_else(|| crate::base::ToolError::InvalidParams("Missing 'items' field".into()))?;
        todo_write(self.store.as_ref(), &items)
            .await
            .map_err(core_err_to_tool)
    }
}

/// `plan_create` — creates a new plan and makes it the active plan.
pub struct PlanCreateTool {
    store: Arc<dyn PlanningStore>,
}

impl PlanCreateTool {
    pub fn new(store: Arc<dyn PlanningStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl crate::base::Tool for PlanCreateTool {
    fn name(&self) -> &str {
        "plan_create"
    }

    fn description(&self) -> &str {
        "Create a new plan and set it as the active plan. Provide a title and goal."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "title": { "type": "string", "description": "Short plan title" },
                "goal": { "type": "string", "description": "What the plan aims to achieve" }
            },
            "required": ["title", "goal"]
        })
    }

    async fn execute(&self, args: Value) -> crate::base::Result<String> {
        let title = args
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::base::ToolError::InvalidParams("Missing 'title'".into()))?
            .to_string();
        let goal = args
            .get("goal")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::base::ToolError::InvalidParams("Missing 'goal'".into()))?
            .to_string();

        if title.trim().is_empty() || goal.trim().is_empty() {
            return Err(crate::base::ToolError::InvalidParams(
                "title and goal cannot be empty".into(),
            ));
        }

        let now = Utc::now();
        let plan = Plan {
            id: PlanId::new(),
            title,
            goal,
            phase: PlanPhase::Explore,
            status: PlanStatus::Pending,
            strategy: None,
            assumptions: vec![],
            risks: vec![],
            open_questions: vec![],
            verification_verdict: None,
            created_at: now,
            updated_at: now,
        };

        let id = plan.id.clone();
        let title_out = plan.title.clone();

        self.store.create_plan(&plan).await.map_err(core_err_to_tool)?;
        self.store
            .set_active_plan(&id)
            .await
            .map_err(core_err_to_tool)?;

        Ok(format!(
            "Plan created: {} (id: {})\nPhase: Explore\nUse todo_write to add todo items, then plan_transition to advance through phases.",
            title_out, id
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::planning::ids::PlanId;

    async fn test_store() -> agent_diva_core::planning::store::SqlitePlanningStore {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .unwrap();
        agent_diva_core::planning::store::SqlitePlanningStore::new(pool)
            .await
            .unwrap()
    }

    fn make_plan(id: &str, title: &str) -> Plan {
        let now = Utc::now();
        Plan {
            id: PlanId(id.to_string()),
            title: title.to_string(),
            goal: "Test goal".to_string(),
            phase: PlanPhase::Execute,
            status: PlanStatus::InProgress,
            strategy: None,
            assumptions: vec![],
            risks: vec![],
            open_questions: vec![],
            verification_verdict: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn make_todo(id: &str, title: &str, status: TodoStatus, priority: TodoPriority) -> TodoItem {
        TodoItem {
            id: TodoId(id.to_string()),
            plan_step_id: None,
            title: title.to_string(),
            detail: None,
            status,
            priority,
            evidence_ref: None,
            block_reason: None,
            updated_at: Utc::now(),
        }
    }

    // -----------------------------------------------------------------------
    // todo_show tests (existing)
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_todo_show_returns_formatted() {
        let store = test_store().await;
        store
            .create_plan(&make_plan("p1", "Ship Widget"))
            .await
            .unwrap();
        store
            .set_active_plan(&PlanId("p1".to_string()))
            .await
            .unwrap();

        store
            .create_todo(
                &PlanId("p1".to_string()),
                &make_todo("t1", "Write code", TodoStatus::InProgress, TodoPriority::High),
            )
            .await
            .unwrap();
        store
            .create_todo(
                &PlanId("p1".to_string()),
                &make_todo("t2", "Write tests", TodoStatus::Pending, TodoPriority::Normal),
            )
            .await
            .unwrap();
        store
            .create_todo(
                &PlanId("p1".to_string()),
                &make_todo("t3", "Done task", TodoStatus::Completed, TodoPriority::Normal),
            )
            .await
            .unwrap();

        let result = todo_show(&store).await.unwrap();

        // Check header
        assert!(result.contains("# Todo List — Ship Widget"));

        // Check status groups
        assert!(result.contains("## In Progress"));
        assert!(result.contains("## Pending"));
        assert!(result.contains("## Completed"));

        // Check items
        assert!(result.contains("- [ ] Write code **[HIGH]**"));
        assert!(result.contains("- [ ] Write tests"));
        assert!(result.contains("- [x] Done task"));
    }

    #[tokio::test]
    async fn test_todo_show_no_active_plan() {
        let store = test_store().await;
        let result = todo_show(&store).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("No active plan"),
            "Expected 'No active plan' error, got: {}",
            err_msg
        );
    }

    // -----------------------------------------------------------------------
    // todo_write tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_todo_write_basic() {
        let store = test_store().await;
        store
            .create_plan(&make_plan("p1", "Build Feature"))
            .await
            .unwrap();
        store
            .set_active_plan(&PlanId("p1".to_string()))
            .await
            .unwrap();

        let items_json = r#"[
            {"title": "Design API", "status": "in_progress", "priority": "high"},
            {"title": "Implement backend", "status": "pending", "priority": "normal"},
            {"title": "Write docs", "status": "pending", "priority": "low", "detail": "README and API docs"}
        ]"#;

        let result = todo_write(&store, items_json).await.unwrap();

        // Check header is present
        assert!(result.contains("# Todo List — Build Feature"));

        // Verify items are in the store
        let plan_id = store.get_active_plan().await.unwrap();
        let todos = store.get_todos(&plan_id).await.unwrap();
        assert_eq!(todos.items.len(), 3);

        // Verify rendered output
        assert!(result.contains("Design API"));
        assert!(result.contains("Implement backend"));
        assert!(result.contains("Write docs"));
    }

    #[tokio::test]
    async fn test_todo_write_replaces_existing() {
        let store = test_store().await;
        store
            .create_plan(&make_plan("p1", "Project"))
            .await
            .unwrap();
        store
            .set_active_plan(&PlanId("p1".to_string()))
            .await
            .unwrap();

        // Create initial todos
        store
            .create_todo(
                &PlanId("p1".to_string()),
                &make_todo("old1", "Old task 1", TodoStatus::Pending, TodoPriority::Normal),
            )
            .await
            .unwrap();
        store
            .create_todo(
                &PlanId("p1".to_string()),
                &make_todo("old2", "Old task 2", TodoStatus::Pending, TodoPriority::Normal),
            )
            .await
            .unwrap();

        // Verify initial state
        let plan_id = store.get_active_plan().await.unwrap();
        let before = store.get_todos(&plan_id).await.unwrap();
        assert_eq!(before.items.len(), 2);

        // Replace with new set
        let items_json = r#"[
            {"title": "New task A", "status": "pending", "priority": "high"},
            {"title": "New task B", "status": "pending", "priority": "normal"}
        ]"#;

        todo_write(&store, items_json).await.unwrap();

        // Verify only new items exist
        let after = store.get_todos(&plan_id).await.unwrap();
        assert_eq!(after.items.len(), 2);

        let titles: Vec<&str> = after.items.iter().map(|t| t.title.as_str()).collect();
        assert!(titles.contains(&"New task A"));
        assert!(titles.contains(&"New task B"));
        assert!(!titles.contains(&"Old task 1"));
        assert!(!titles.contains(&"Old task 2"));
    }

    #[tokio::test]
    async fn test_todo_write_no_active_plan() {
        let store = test_store().await;

        let items_json = r#"[{"title": "Task"}]"#;
        let result = todo_write(&store, items_json).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("No active plan"),
            "Expected 'No active plan' error, got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_todo_write_validates_empty_title() {
        let store = test_store().await;
        store
            .create_plan(&make_plan("p1", "Project"))
            .await
            .unwrap();
        store
            .set_active_plan(&PlanId("p1".to_string()))
            .await
            .unwrap();

        let items_json = r#"[{"title": "", "status": "pending"}]"#;
        let result = todo_write(&store, items_json).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Todo title cannot be empty"),
            "Expected empty title error, got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_todo_write_validates_blocked_needs_reason() {
        let store = test_store().await;
        store
            .create_plan(&make_plan("p1", "Project"))
            .await
            .unwrap();
        store
            .set_active_plan(&PlanId("p1".to_string()))
            .await
            .unwrap();

        // Blocked without block_reason
        let items_json =
            r#"[{"title": "Blocked task", "status": "blocked", "priority": "high"}]"#;
        let result = todo_write(&store, items_json).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Blocked items must have block_reason"),
            "Expected block_reason error, got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_todo_write_validates_invalid_status() {
        let store = test_store().await;
        store
            .create_plan(&make_plan("p1", "Project"))
            .await
            .unwrap();
        store
            .set_active_plan(&PlanId("p1".to_string()))
            .await
            .unwrap();

        let items_json = r#"[{"title": "Bad status", "status": "invalid_status"}]"#;
        let result = todo_write(&store, items_json).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Invalid todo status"),
            "Expected invalid status error, got: {}",
            err_msg
        );
    }
}
