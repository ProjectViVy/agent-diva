# TodoList Runtime Architecture

## Background And Goal

This document extends the Plan Mode architecture with a lightweight TodoList
runtime. The goal is to support rolling execution planning before introducing a
durable multi-agent Kanban system.

The core decision is:

```text
Plan owns the goal and strategy.
TodoList is the execution projection generated from the Plan.
```

TodoList should not become a second planning system. It is the short-horizon
working queue that the agent updates while executing a plan. Kanban-style
multi-worker dispatch, durable queues, profile workers, and task claiming are
intentionally deferred until the autonomous-evolution loop is mature.

## Relationship To Plan Mode

The existing Plan Mode flow is:

```text
Explore -> Plan -> AwaitingApproval -> Execute -> Verify -> Completed
                                                       -> Failed
                                                       -> Partial
```

With TodoList, the execution path becomes:

```text
Explore -> Plan -> GenerateTodoList -> AwaitingApproval -> ExecuteTodos -> Verify
```

`AwaitingApproval` remains part of strict Plan Mode. In lighter rolling-planning
usage, the runtime may generate and update TodoList without forcing a full
approval gate on every revision. The approval gate still applies when the user
explicitly enters Plan Mode or when future policy marks an operation as risky.

## Non Goals

- No Kanban board in this stage.
- No multiple active TodoLists per plan.
- No parallel TodoItem execution.
- No worker claiming, dispatcher loop, or profile assignment.
- No dependency DAG for TodoItem in the MVP.
- No autonomous task creation outside an active plan or user request.
- No use of Markdown parsing as the source of truth.

## Conceptual Model

### Plan

Plan is the strategic layer:

- user goal
- current understanding
- assumptions
- constraints
- risks
- plan steps
- expected outputs
- verification strategy

### TodoList

TodoList is the execution layer:

- concrete next actions
- current progress
- blocked items
- evidence links
- short notes needed for recovery

TodoList is regenerated or revised as the plan evolves. The agent should prefer
small, immediately actionable TodoItems over broad strategy statements.

## Directory Protocol

TodoList state lives under the existing plan directory:

```text
.agent-diva/
  plans/
    active.json
    <plan-id>/
      input.md
      exploration_findings.md
      plan.md
      todo.json
      todo.md
      state.json
      events.jsonl
      verification.md
      evidence/
        todo-001.md
        todo-002.md
```

File responsibilities:

- `todo.json`: canonical typed TodoList state.
- `todo.md`: human-readable projection for review and status display.
- `evidence/todo-*.md`: execution evidence for completed TodoItems.

`todo.json` is the source of truth. `todo.md` is a projection and must never be
parsed to recover state.

## Data Model Draft

```rust
pub struct TodoList {
    pub plan_id: PlanId,
    pub revision: u64,
    pub items: Vec<TodoItem>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct TodoItem {
    pub id: TodoId,
    pub plan_step_id: Option<String>,
    pub title: String,
    pub detail: Option<String>,
    pub status: TodoStatus,
    pub priority: TodoPriority,
    pub evidence_ref: Option<String>,
    pub block_reason: Option<String>,
    pub updated_at: DateTime<Utc>,
}

pub enum TodoStatus {
    Pending,
    InProgress,
    Blocked,
    Completed,
    Canceled,
}

pub enum TodoPriority {
    Low,
    Normal,
    High,
}

pub enum TodoEvent {
    TodoListGenerated { plan_id: PlanId, revision: u64 },
    TodoListRevised { plan_id: PlanId, revision: u64 },
    TodoStarted { todo_id: TodoId },
    TodoBlocked { todo_id: TodoId },
    TodoCompleted { todo_id: TodoId },
    TodoCanceled { todo_id: TodoId },
}
```

`plan_step_id` is optional because some execution tasks are operational details
that do not deserve their own PlanStep. For example, "inspect Cargo.toml" may
support a broader PlanStep such as "identify integration points".

## Component Boundaries

### PlanOrchestrator

Owns the plan lifecycle and decides when a TodoList should be generated,
revised, or considered complete. It remains the only component allowed to move a
plan across major phases.

### TodoListStore

Owns `todo.json`, writes `todo.md`, appends Todo events to `events.jsonl`, and
performs conservative recovery checks.

### TodoPlanner

Turns an approved or active Plan into TodoItems. It should be model-assisted but
validated by typed rules before being persisted.

### AgentLoop

Executes the current TodoItem through existing tools. It may report progress and
evidence, but it should not independently create a new plan lifecycle.

### ContextBuilder

Injects the active plan summary and active TodoList into the system context. The
injection should be compact and should prioritize:

1. current plan goal
2. current strategy
3. active TodoItem
4. pending and blocked TodoItems
5. last revision timestamp

### MemoryProvider

May receive completed plan and Todo summaries after verification. It does not own
active TodoList state.

## Runtime Rules

- A TodoList must belong to exactly one Plan.
- A Plan may have zero or one active TodoList in the MVP.
- At most one TodoItem may be `InProgress`.
- `Completed` TodoItems should include `evidence_ref` when execution involved
  tool use or file changes.
- `Blocked` TodoItems must include `block_reason`.
- Revisions must be monotonic.
- TodoList recovery must prefer `todo.json` plus `events.jsonl`.
- TodoList updates should be append-observable through `TodoEvent`.

## Tool Surface Draft

The MVP tool surface should stay small:

- `todo_show`: show active TodoList.
- `todo_update`: create or revise TodoItems for the current Plan.
- `todo_start`: mark one TodoItem as in progress.
- `todo_complete`: mark one TodoItem complete with evidence.
- `todo_block`: mark one TodoItem blocked with a reason.

For model ergonomics, `todo_update` may support batch replacement, but the
runtime must preserve revision history through events.

## PlanStep Versus TodoItem

| Layer | Purpose | Example |
| --- | --- | --- |
| PlanStep | Strategic phase with rationale and expected output | "Design the TodoList state model and recovery rules." |
| TodoItem | Immediate executable action | "Read the existing Plan Mode architecture document." |

PlanStep answers "why and what". TodoItem answers "what is the next concrete
thing to do".

## Future Kanban Upgrade Path

TodoList deliberately avoids dispatcher and worker semantics. Later, Kanban can
promote selected TodoItems into durable multi-agent tasks when any of these are
true:

- work should survive process restarts independently of the current session
- multiple named workers are needed
- human comments and unblock flow are required
- workspace isolation is required
- task audit must outlive a single plan

This keeps the current stage lightweight while preserving a clean migration path
from `Plan -> TodoList -> Kanban`.

## Implementation Roadmap

### Phase 0: Documents And Types

- Land this architecture document.
- Add TodoList/TodoItem/TodoEvent domain types.
- Add serialization tests.

### Phase 1: Store And Context Injection

- Implement `TodoListStore`.
- Add active TodoList context injection.
- Render `todo.md` from typed state.

### Phase 2: Tool Surface

- Add Todo tools through `ToolAssembly`.
- Enforce one active `InProgress` item.
- Record evidence references for completed items.

### Phase 3: Plan Integration

- Generate TodoList from Plan draft or approved Plan.
- Execute TodoItems through PlanOrchestrator.
- Include Todo status in `plan status`.

## Acceptance Criteria

- TodoList is explicitly modeled as a Plan execution projection.
- Todo state is typed and durable under `.agent-diva/plans/<plan-id>/`.
- Markdown is human-facing only.
- The MVP remains serial and avoids Kanban/dispatcher semantics.
- Active Plan and TodoList can be injected into context after session resume or
  compaction.
