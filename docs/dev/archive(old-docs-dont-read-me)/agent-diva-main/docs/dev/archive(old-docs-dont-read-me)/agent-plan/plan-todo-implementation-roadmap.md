# Plan + TodoList Technical Implementation Roadmap

## Purpose

This document turns the Plan Mode and TodoList architecture into a practical
implementation route. The target capability is rolling planning:

```text
user goal -> plan -> generated todolist -> serial execution -> rolling update
```

Kanban, durable multi-worker dispatch, profile workers, and autonomous evolution
are intentionally deferred.

## Design Options

### Option A: File-Backed JSON Store

Store plan and TodoList state in:

```text
.agent-diva/plans/<plan-id>/state.json
.agent-diva/plans/<plan-id>/todo.json
.agent-diva/plans/<plan-id>/events.jsonl
```

Pros:

- Easiest to implement.
- Easy to inspect and edit manually.
- Matches the existing Plan Mode architecture document.

Cons:

- Atomic updates and recovery are harder.
- Concurrent writes become fragile later.
- Querying active, blocked, or stale items is awkward.

Use only if the goal is a very fast prototype.

### Option B: SQLite Store With Markdown Projections

Use SQLite as the canonical state store, while still rendering human-readable
artifacts under `.agent-diva/plans/<plan-id>/`:

```text
.agent-diva/planning.db
.agent-diva/plans/<plan-id>/plan.md
.agent-diva/plans/<plan-id>/todo.md
.agent-diva/plans/<plan-id>/verification.md
.agent-diva/plans/<plan-id>/evidence/
```

Pros:

- Stronger recovery and transaction semantics.
- Natural path to future Kanban.
- Easy to query current active plan and TodoItems.
- Avoids reworking persistence when moving from TodoList to task board.

Cons:

- Slightly more upfront schema work.
- Requires migrations and store tests.

Recommended.

### Option C: Pure Session-State TodoList

Keep TodoList in session memory and inject it into context.

Pros:

- Minimal implementation.
- Good for UI experiments.

Cons:

- Weak resume behavior.
- Poor evidence trail.
- Hard to connect with Plan Mode approval and verification.

Not recommended except for throwaway experiments.

## Recommendation

Use **Option B: SQLite Store With Markdown Projections**.

The current workspace already has `sqlx` with SQLite enabled. This gives a
lightweight durable substrate now and keeps the later Kanban upgrade path clean.
Markdown remains useful for human review, but typed SQLite state owns runtime
decisions.

## Crate And Module Placement

### `agent-diva-core::planning`

Own shared domain types and store contracts:

```text
agent-diva-core/src/planning/
  mod.rs
  ids.rs
  model.rs
  events.rs
  store.rs
  render.rs
```

Responsibilities:

- `Plan`, `PlanStep`, `TodoList`, `TodoItem`.
- `PlanPhase`, `PlanStatus`, `TodoStatus`.
- `PlanEvent`, `TodoEvent`.
- Store traits and SQLite implementation.
- Markdown projection rendering helpers.

Rationale: planning state is cross-cutting domain state. It should not live in
`agent-diva-agent`, because manager, CLI, GUI, and future services need the same
types.

### `agent-diva-agent::planning`

Own runtime orchestration:

```text
agent-diva-agent/src/planning/
  mod.rs
  orchestrator.rs
  todo_planner.rs
  context.rs
  verifier.rs
```

Responsibilities:

- Generate plan drafts.
- Generate TodoList from Plan.
- Execute TodoItems through the existing agent loop/tool registry.
- Record evidence.
- Decide when to revise TodoList.

### `agent-diva-tools::planning`

Expose model-facing tools:

```text
agent-diva-tools/src/planning.rs
```

Initial tools:

- `plan_show`
- `todo_show`
- `todo_update`
- `todo_start`
- `todo_complete`
- `todo_block`

Keep `plan_update` out of the first tool set unless the product wants the model
to directly mutate Plan. A safer first version lets `PlanOrchestrator` own Plan
changes and gives the model TodoList-level control.

### `agent-diva-manager`

Expose control-plane APIs:

- `GET /planning/status`
- `GET /planning/plans/{id}`
- `GET /planning/plans/{id}/todo`
- `POST /planning/plans/{id}/approve`
- `POST /planning/plans/{id}/cancel`

CLI commands can call the same manager API later.

## Data Model V1

### Tables

```sql
CREATE TABLE plans (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  goal TEXT NOT NULL,
  phase TEXT NOT NULL,
  status TEXT NOT NULL,
  strategy TEXT,
  assumptions_json TEXT NOT NULL DEFAULT '[]',
  risks_json TEXT NOT NULL DEFAULT '[]',
  open_questions_json TEXT NOT NULL DEFAULT '[]',
  verification_verdict TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE plan_steps (
  id TEXT PRIMARY KEY,
  plan_id TEXT NOT NULL,
  ordinal INTEGER NOT NULL,
  title TEXT NOT NULL,
  rationale TEXT,
  expected_output TEXT,
  status TEXT NOT NULL,
  evidence_ref TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(plan_id) REFERENCES plans(id)
);

CREATE TABLE todo_items (
  id TEXT PRIMARY KEY,
  plan_id TEXT NOT NULL,
  plan_step_id TEXT,
  ordinal INTEGER NOT NULL,
  title TEXT NOT NULL,
  detail TEXT,
  status TEXT NOT NULL,
  priority TEXT NOT NULL,
  evidence_ref TEXT,
  block_reason TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(plan_id) REFERENCES plans(id),
  FOREIGN KEY(plan_step_id) REFERENCES plan_steps(id)
);

CREATE TABLE planning_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  plan_id TEXT NOT NULL,
  event_type TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY(plan_id) REFERENCES plans(id)
);

CREATE TABLE active_plan (
  singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
  plan_id TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(plan_id) REFERENCES plans(id)
);
```

### Runtime Invariants

- Only one active plan in MVP.
- Only one `todo_items.status = 'in_progress'` per plan.
- TodoItems are ordered by `ordinal`.
- A TodoItem cannot be completed without evidence when it performed tool or file
  mutations.
- A blocked TodoItem must have `block_reason`.
- Markdown projections are regenerated from typed state.

## Execution Flow

### 1. Start Plan

Entry point:

```text
plan start "<user goal>"
```

Runtime:

1. Create `plans` row in `Explore`.
2. Write `input.md` projection.
3. Run bounded exploration through the existing agent loop.
4. Save `exploration_findings.md`.
5. Draft Plan and PlanSteps.
6. Generate TodoItems from PlanSteps.
7. Render `plan.md` and `todo.md`.
8. Move to `AwaitingApproval` if strict Plan Mode is enabled.

### 2. Approve Plan

Runtime:

1. Validate there is an active plan.
2. Validate `plan.md`, `todo.md`, and typed TodoItems exist.
3. Transition to `Execute`.
4. Start the first pending TodoItem.

### 3. Execute TodoItem

Runtime:

1. Mark TodoItem `InProgress`.
2. Inject active Plan and TodoList into context.
3. Let existing `AgentLoop` execute the concrete item.
4. Write `evidence/todo-<id>.md`.
5. Mark TodoItem `Completed`, `Blocked`, or `Canceled`.
6. If execution changes the strategy, revise TodoList and increment revision.

### 4. Rolling Todo Update

TodoList may change when:

- exploration discovers a missing prerequisite
- a TodoItem becomes blocked
- verification fails
- user changes the goal
- execution makes a TodoItem obsolete

Every revision must append a `TodoListRevised` event and regenerate `todo.md`.

### 5. Verify

After all non-canceled TodoItems complete:

1. Run verification command or record manual verification.
2. Write `verification.md`.
3. Store `PASS`, `FAIL`, or `PARTIAL`.
4. Transition Plan to terminal status.

## Context Injection

Add a compact block after memory/soul context and before generic tool guidance:

```text
## Active Plan
Goal: ...
Phase: Execute
Strategy: ...

## Active TodoList
Current: [todo-003] Implement TodoListStore serialization tests
Pending:
- [todo-004] Add context injection renderer
- [todo-005] Add manager status endpoint
Blocked:
- [todo-002] Needs user approval for ...
```

Rules:

- Keep the block compact.
- Include only active, pending, and blocked items.
- Do not inject full evidence.
- Link evidence by stable artifact name when needed.

## Tool Integration

Add planning tools through `ToolAssembly` as a separate feature toggle inside
`BuiltInToolsConfig`.

Suggested toggle:

```rust
pub struct BuiltInToolsConfig {
    pub planning: bool,
    // existing fields...
}
```

Tool behavior:

- `todo_show`: read-only.
- `todo_update`: mutates TodoList revision.
- `todo_start`: enforces one active item.
- `todo_complete`: requires summary and optional evidence ref.
- `todo_block`: requires reason.

The first implementation should not let tools bypass `PlanOrchestrator` phase
gates. Tools should call a planning service boundary, not write the store
directly.

## Event Bus Integration

Add planning events to `AgentEvent` or introduce a nested planning event type:

```rust
PlanningEvent {
    PlanCreated,
    PlanDrafted,
    TodoListGenerated,
    TodoStarted,
    TodoCompleted,
    TodoBlocked,
    PlanVerificationRecorded,
}
```

Manager, CLI, and future GUI should observe this event stream. Persistence still
belongs to the planning store.

## Implementation Phases

### Phase 0: Types And Store

Deliverables:

- `agent-diva-core::planning` module.
- SQLite schema and migrations.
- Domain model serialization tests.
- Store CRUD tests with `tempfile`.
- Markdown projection renderer tests.

Acceptance:

- Create a plan with steps and TodoItems.
- Load it after process restart.
- Render deterministic `plan.md` and `todo.md`.

### Phase 1: Context And Read-Only Status

Deliverables:

- Active plan pointer.
- Context injection renderer.
- `plan status` / manager status read path.
- `todo_show` read-only tool.

Acceptance:

- Active plan and TodoList are visible in context.
- A resumed session can recover current Todo state.

### Phase 2: Todo Tool Mutations

Deliverables:

- `todo_update`, `todo_start`, `todo_complete`, `todo_block`.
- Revision events.
- One-in-progress invariant.
- Evidence artifact write path.

Acceptance:

- Agent can execute a 3-item TodoList serially.
- Blocked item records reason.
- Completed item records evidence.

### Phase 3: Plan-To-Todo Generation

Deliverables:

- Model-assisted TodoList generation from PlanStep.
- Validation and normalization layer.
- Strict Plan Mode approval gate.

Acceptance:

- `plan start` creates Plan + TodoList.
- User approval is required before strict execution.
- TodoList can be revised after a blocked item.

### Phase 4: Verification And Closure

Deliverables:

- Verification recording.
- PASS/FAIL/PARTIAL terminal transition.
- Completed Plan/Todo summary handoff to MemoryProvider.

Acceptance:

- Plan cannot complete without verification.
- Failed verification does not auto-promote to success.

## Risks And Controls

| Risk | Control |
| --- | --- |
| TodoList becomes a second planning system | Require every TodoList to have a PlanId and render it under a plan directory |
| Model mutates Plan too freely | Keep first tool surface Todo-focused; PlanOrchestrator owns Plan changes |
| Context injection gets too large | Compact renderer with caps and active/pending/blocked filtering |
| Store design overfits future Kanban | Keep schema serial and single-active-plan; avoid worker/claim fields now |
| Markdown drift | Always regenerate Markdown from typed state |
| Execution bypasses approval | PlanOrchestrator phase gates wrap tool mutations and execution entry points |

## First Concrete PR Slice

The smallest useful implementation PR should be:

1. Add `agent-diva-core::planning` domain types.
2. Add SQLite store initialization and CRUD for `plans`, `plan_steps`,
   `todo_items`, and `planning_events`.
3. Add deterministic `todo.md` renderer.
4. Add tests for create/load/render and one-active-todo invariant.

This slice produces no user-facing command yet, but it creates the durable base
for the next PR.
