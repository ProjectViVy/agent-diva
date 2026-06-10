//! Planning tools that require access to the [`PlanOrchestrator`].
//!
//! These tools live in the agent crate (not agent-diva-tools) because they
//! depend on the orchestrator, which is an agent-level concept.

use agent_diva_core::planning::model::PlanPhase;
use agent_diva_core::planning::render::render_plan_md;
use agent_diva_core::planning::store::PlanningStore;
use agent_diva_tooling::{Result as ToolResult, Tool, ToolError};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::orchestrator::PlanOrchestrator;

fn core_err(e: agent_diva_core::Error) -> ToolError {
    ToolError::ExecutionFailed(e.to_string())
}

fn parse_plan_phase(s: &str) -> Result<PlanPhase, ToolError> {
    match s {
        "explore" | "Explore" => Ok(PlanPhase::Explore),
        "plan" | "Plan" => Ok(PlanPhase::Plan),
        "awaiting_approval" | "AwaitingApproval" => Ok(PlanPhase::AwaitingApproval),
        "execute" | "Execute" => Ok(PlanPhase::Execute),
        "verify" | "Verify" => Ok(PlanPhase::Verify),
        "completed" | "Completed" => Ok(PlanPhase::Completed),
        "failed" | "Failed" => Ok(PlanPhase::Failed),
        "partial" | "Partial" => Ok(PlanPhase::Partial),
        _ => Err(ToolError::InvalidParams(format!(
            "Invalid phase: '{}'. Valid: explore, plan, awaiting_approval, execute, verify, completed, failed, partial",
            s
        ))),
    }
}

// ---------------------------------------------------------------------------
// PlanApproveTool
// ---------------------------------------------------------------------------

/// `plan_approve` — marks a plan as approved, allowing transition to Execute.
pub struct PlanApproveTool {
    orchestrator: Arc<Mutex<PlanOrchestrator>>,
    store: Arc<dyn PlanningStore>,
}

impl PlanApproveTool {
    pub fn new(orchestrator: Arc<Mutex<PlanOrchestrator>>, store: Arc<dyn PlanningStore>) -> Self {
        Self {
            orchestrator,
            store,
        }
    }
}

#[async_trait]
impl Tool for PlanApproveTool {
    fn name(&self) -> &str {
        "plan_approve"
    }

    fn description(&self) -> &str {
        "Approve the active plan, allowing it to transition to the Execute phase."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _args: Value) -> ToolResult<String> {
        let plan_id = self.store.get_active_plan().await.map_err(core_err)?;
        let plan = self.store.get_plan(&plan_id).await.map_err(core_err)?;

        let mut orch = self.orchestrator.lock().await;
        orch.approve(&plan_id);

        Ok(format!(
            "Plan '{}' (id: {}) approved. You may now call plan_transition to advance to Execute phase.",
            plan.title, plan_id
        ))
    }
}

// ---------------------------------------------------------------------------
// PlanTransitionTool
// ---------------------------------------------------------------------------

/// `plan_transition` — transitions the active plan to a new phase.
pub struct PlanTransitionTool {
    orchestrator: Arc<Mutex<PlanOrchestrator>>,
    store: Arc<dyn PlanningStore>,
}

impl PlanTransitionTool {
    pub fn new(orchestrator: Arc<Mutex<PlanOrchestrator>>, store: Arc<dyn PlanningStore>) -> Self {
        Self {
            orchestrator,
            store,
        }
    }
}

#[async_trait]
impl Tool for PlanTransitionTool {
    fn name(&self) -> &str {
        "plan_transition"
    }

    fn description(&self) -> &str {
        "Transition the active plan to a new lifecycle phase. Valid transitions: explore→plan→awaiting_approval→execute→verify→completed/failed/partial."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "phase": {
                    "type": "string",
                    "description": "Target phase: explore, plan, awaiting_approval, execute, verify, completed, failed, partial"
                }
            },
            "required": ["phase"]
        })
    }

    async fn execute(&self, args: Value) -> ToolResult<String> {
        let phase_str = args
            .get("phase")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParams("Missing 'phase'".into()))?;

        let new_phase = parse_plan_phase(phase_str)?;
        let plan_id = self.store.get_active_plan().await.map_err(core_err)?;

        let mut orch = self.orchestrator.lock().await;
        let plan = orch
            .transition_to(self.store.as_ref(), &plan_id, new_phase)
            .await
            .map_err(core_err)?;

        Ok(format!(
            "Plan '{}' transitioned to phase: {} (status: {})",
            plan.title, plan.phase, plan.status
        ))
    }
}

// ---------------------------------------------------------------------------
// PlanShowTool
// ---------------------------------------------------------------------------

/// `plan_show` — read-only tool that returns the current plan with its steps.
pub struct PlanShowTool {
    store: Arc<dyn PlanningStore>,
}

impl PlanShowTool {
    pub fn new(store: Arc<dyn PlanningStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for PlanShowTool {
    fn name(&self) -> &str {
        "plan_show"
    }

    fn description(&self) -> &str {
        "Show the current active plan details including title, goal, phase, strategy, and steps."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _args: Value) -> ToolResult<String> {
        let plan_id = self.store.get_active_plan().await.map_err(core_err)?;
        let plan = self.store.get_plan(&plan_id).await.map_err(core_err)?;
        let steps = self.store.get_steps(&plan_id).await.map_err(core_err)?;
        Ok(render_plan_md(&plan, &steps))
    }
}
