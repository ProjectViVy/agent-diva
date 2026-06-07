---
stepsCompleted: [step-01-validate-prerequisites, step-02-design-epics, step-03-stories]
inputDocuments:
  - docs/dev/agent-plan/plan-todo-implementation-roadmap.md
  - docs/dev/agent-plan/todolist-runtime-architecture.md
  - docs/dev/agent-plan/plan-mode-architecture.md
  - docs/dev/agent-plan/planning-gui-design-supplement.md
  - docs/dev/agent-plan/pre-implementation-research.md
  - docs/dev/agent-plan/plan-todo-ui-scope-extract.md
  - docs/dev/agent-plan/phase-a-pre-session-truth-source-fix.md
---

# Agent Diva Pro — Plan+TodoList Epic Breakdown

## Overview

This document provides the complete epic and story breakdown for the Plan+TodoList feature on the `agent-diva-pro` branch. It transforms the 7 design documents into implementable stories organized by delivery order.

**Scope boundary:** Kanban, durable multi-worker dispatch, profile workers, and autonomous evolution are intentionally deferred. This MVP covers rolling single-plan execution: `user goal → plan → generated todolist → serial execution → rolling update`.

## Requirements Inventory

### Functional Requirements

FR1: The system SHALL provide domain types for Plan (id, title, goal, phase, status, strategy, assumptions, risks, open_questions, verification_verdict) in `agent-diva-core::planning`.
FR2: The system SHALL provide domain types for PlanStep (id, plan_id, ordinal, title, rationale, expected_output, status, evidence_ref).
FR3: The system SHALL provide domain types for TodoList (plan_id, revision, items) and TodoItem (id, plan_step_id, title, detail, status, priority, evidence_ref, block_reason).
FR4: The system SHALL provide enums: PlanPhase (Explore/Plan/AwaitingApproval/Execute/Verify/Completed/Failed/Partial), PlanStatus (Pending/InProgress/Blocked/Completed/Failed/Partial/Canceled), TodoStatus (Pending/InProgress/Blocked/Completed/Canceled), TodoPriority (Low/Normal/High).
FR5: The system SHALL persist plans, steps, todos, and events in SQLite with 5 tables (plans, plan_steps, todo_items, planning_events, active_plan) at `.agent-diva/planning.db`.
FR6: The system SHALL enforce a single active plan constraint via a singleton `active_plan` table.
FR7: The system SHALL render TodoList as deterministic markdown (`todo.md`) regenerated from typed state, never parsed back.
FR8: The system SHALL provide a `todo_show` read-only tool for the model to query current todo state.
FR9: The system SHALL provide a `todo_write` full-replace tool for the model to create/revise the entire TodoList in one call.
FR10: The system SHALL inject a compact Plan+TodoList context block (≤800 chars) into the agent system prompt after memory context and before tool guidance.
FR11: The system SHALL implement a NAG mechanism that injects a reminder after 3 consecutive model turns with pending todos but no planning tool calls.
FR12: The system SHALL implement PlanOrchestrator with a phase state machine (Explore→Plan→AwaitingApproval→Execute→Verify→Completed/Failed/Partial) and gate rules for transitions.
FR13: The system SHALL gate phase transitions: approval required before Execute; verification required before terminal state.
FR14: The system SHALL integrate 5 AgentLoop hooks: (1) inject plan/todo system message before iteration, (2) detect planning tool calls after LLM response, (3) detect state changes after tool execution, (4) sync planning store before session save, (5) protect plan/todo from consolidation compaction.
FR15: The system SHALL expose a REST API for plan CRUD: GET/POST/PUT/DELETE `/api/plans` via `agent-diva-manager`.
FR16: The system SHALL expose Tauri commands: `get_plans()`, `get_plan(planId)`, `get_active_plan()` for GUI access.
FR17: The system SHALL log planning events (PlanCreated, PlanDrafted, TodoListGenerated, TodoStarted, TodoCompleted, TodoBlocked, VerificationRecorded) to the `planning_events` table.

### Non-Functional Requirements

NFR1: Only one active plan at a time (enforced by singleton table and store logic).
NFR2: Only one `todo_items.status = 'in_progress'` per plan at any time.
NFR3: Blocked TodoItems MUST have a non-empty `block_reason`.
NFR4: Completed TodoItems that performed tool or file mutations MUST have an `evidence_ref`.
NFR5: Markdown projections are always regenerated from typed SQLite state — never parsed back as source of truth.
NFR6: SQLite database path: `.agent-diva/planning.db`. Migrations via `CREATE TABLE IF NOT EXISTS` for MVP.
NFR7: Context injection block MUST be ≤800 chars, including only active/pending/blocked items.
NFR8: All planning domain types live in `agent-diva-core::planning` for cross-crate reuse (core, agent, tools, manager, GUI).
NFR9: Planning tools registered via `BuiltInToolsConfig.planning: bool` toggle in `agent-diva-agent::tool_config`.
NFR10: PlanningService in manager is lazy-created (same pattern as SkillService/McpService).

### Additional Requirements (Architecture)

AR1: Module placement — `agent-diva-core::planning` (types + store + render), `agent-diva-agent::planning` (orchestrator + context + verifier), `agent-diva-tools::planning` (model-facing tools).
AR2: `sqlx` with SQLite already available as workspace dependency — no new dependency needed.
AR3: PR dependency chain: PR#1 → PR#2 → PR#3 → PR#4 → PR#6; PR#5 can parallelize after PR#1.
AR4: Phase A-PRE prerequisite (session durability fix, P0-4) is already completed on both main and pro branches.
AR5: `PlanningService` in manager follows the same lazy-init pattern as `SkillService` and `McpService`.
AR6: Tool behavior rules: `todo_write` is full-replace (Claude Code V1 style); `plan_update` is deferred — PlanOrchestrator owns Plan changes.
AR7: `agent-diva-sandbox` crate is NOT in current workspace — sandbox remediation is a separate effort.

### UX Design Requirements

UX-DR1: `DecisionCard.vue` renders `plan_create` tool output as approval card with steps, risk level, approve/reject buttons (403 lines). **ALREADY BUILT in pro.**
UX-DR2: `TodoCard.vue` renders `todo_write` tool output as interactive checklist with checkboxes, progress counter, mark-all-done, auto-collapse when all done (322 lines). **ALREADY BUILT in pro.**
UX-DR3: `ApprovalBanner.vue` renders `approval_request` as time-limited banner with countdown, risk badge, allow/reject buttons, keyboard shortcuts (279 lines). **ALREADY BUILT in pro.**
UX-DR4: Permission selector (cautious/smart/trusted) inline in `ChatView.vue`. **ALREADY BUILT in pro.**
UX-DR5: `PlanningView.vue` standalone dual-pane page (left Plan list + right detail). **NOT BUILT — Phase B.**
UX-DR6: `PlanStatusCard.vue` active plan overview with 4-grid status indicator. **NOT BUILT — Phase B.**
UX-DR7: `TodoListPanel.vue` progress panel grouped by status with `TodoItemRow.vue`. **NOT BUILT — Phase B.**
UX-DR8: Status color system (pending=gray, in_progress=pink, completed=green, blocked=amber, canceled=gray, failed=red). **ALREADY BUILT in pro.**
UX-DR9: i18n zh/en for all planning strings (`todoCard.*`, `card.*`, `approval.*`, `chat.planMode`). **ALREADY BUILT in pro.**
UX-DR10: Sidebar navigation adds `'planning'` section between existing items. **NOT BUILT — Phase B.**

### FR Coverage Map

| FR | Epic | Story |
|----|------|-------|
| FR1 | Epic 1 | 1.1 |
| FR2 | Epic 1 | 1.1 |
| FR3 | Epic 1 | 1.1 |
| FR4 | Epic 1 | 1.1 |
| FR5 | Epic 1 | 1.2 |
| FR6 | Epic 1 | 1.2 |
| FR7 | Epic 1 | 1.3 |
| FR8 | Epic 2 | 2.2 |
| FR9 | Epic 3 | 3.1 |
| FR10 | Epic 2 | 2.1 |
| FR11 | Epic 3 | 3.2 |
| FR12 | Epic 4 | 4.1 |
| FR13 | Epic 4 | 4.2 |
| FR14 | Epic 2-3 | 2.3, 3.3 |
| FR15 | Epic 5 | 5.1, 5.2 |
| FR16 | Epic 5 | 5.3 |
| FR17 | Epic 1 | 1.2 |

## Epic List

| Epic | Title | Depends On | PR# | Est. Lines |
|------|-------|-----------|-----|-----------|
| Epic 1 | Planning Domain Foundation | — | PR#1 | ~300 |
| Epic 2 | Plan Context & Read Path | Epic 1 | PR#2 | ~150 |
| Epic 3 | Todo Mutations & NAG | Epic 2 | PR#3 | ~200 |
| Epic 4 | Plan Lifecycle Management | Epic 3 | PR#4 | ~250 |
| Epic 5 | Planning HTTP API | Epic 1 | PR#5 | ~200 |
| Epic 6 | Verification & Closure | Epic 4 | PR#6 | ~150 |
| Epic 7 | GUI Planning View | Epic 4 | GUI Phase B | ~400 |

**Dependency graph:**
```
Epic 1 ──→ Epic 2 ──→ Epic 3 ──→ Epic 4 ──→ Epic 6
  │                                          
  └──→ Epic 5 (parallel after Epic 1)
                    Epic 4 ──→ Epic 7
```

---

## Epic 1: Planning Domain Foundation

**Goal:** Establish the durable planning substrate — domain types, SQLite store, and markdown projections — that all subsequent epics build on.

**Delivers:** FR1–FR7, FR17, NFR1, NFR5–NFR8, AR1–AR2

### Story 1.1: Domain Types

As a developer,
I want complete domain types for Plan, PlanStep, TodoList, and TodoItem with all status/priority enums,
So that all crates can share a single typed planning model without duplication.

**Acceptance Criteria:**

**Given** `agent-diva-core::planning` module exists with `ids.rs`, `model.rs`, `events.rs`
**When** I inspect the types
**Then** `PlanId(String)` and `TodoId(String)` newtypes exist in `ids.rs`
**And** `Plan` struct has fields: id, title, goal, phase, status, strategy, assumptions, risks, open_questions, verification_verdict, created_at, updated_at
**And** `PlanStep` struct has fields: id, plan_id, ordinal, title, rationale, expected_output, status, evidence_ref, created_at, updated_at
**And** `TodoItem` struct has fields: id, plan_step_id, title, detail, status, priority, evidence_ref, block_reason, updated_at
**And** `TodoList` struct has fields: plan_id, revision, items, created_at, updated_at
**And** `PlanPhase`, `PlanStatus`, `TodoStatus`, `TodoPriority`, `VerificationVerdict` enums are defined with all variants
**And** `PlanEvent` and `TodoEvent` enums cover all event types (Created, Drafted, Generated, Started, Completed, Blocked, etc.)
**And** all types derive `Debug, Clone, Serialize, Deserialize` and implement `PartialEq` where appropriate
**And** `mod.rs` re-exports all public types

### Story 1.2: SQLite Store

As a developer,
I want a `SqlitePlanningStore` that persists plans, steps, todos, and events with full CRUD,
So that planning state survives process restarts and can be queried efficiently.

**Acceptance Criteria:**

**Given** `store.rs` in `agent-diva-core::planning` defines a `PlanningStore` trait
**When** I examine the trait
**Then** it has methods: `create_plan`, `get_plan`, `update_plan`, `delete_plan`, `list_plans`
**And** methods: `create_step`, `get_steps`, `update_step`
**And** methods: `create_todo`, `get_todos`, `update_todo`, `delete_todos`
**And** methods: `append_event`, `get_events`
**And** methods: `set_active_plan`, `get_active_plan`

**Given** `SqlitePlanningStore` implements `PlanningStore` with `SqlitePool`
**When** I run `create_plan` with a valid Plan
**Then** the plan is persisted in the `plans` table
**And** `get_plan` returns the same Plan with all fields round-tripped
**And** `set_active_plan` writes to the singleton `active_plan` table
**And** calling `set_active_plan` twice replaces the previous active plan

**Given** the 5-table schema (plans, plan_steps, todo_items, planning_events, active_plan)
**When** the store initializes
**Then** all tables are created via `CREATE TABLE IF NOT EXISTS`
**And** foreign keys are enforced
**And** the singleton constraint on `active_plan` is enforced via `CHECK (singleton = 1)`

**Given** unit tests using `tempfile` for the SQLite database
**When** tests run
**Then** they cover: create/load plan, CRUD steps, CRUD todos, active plan singleton, event append/query
**And** all tests pass with `cargo test -p agent-diva-core planning`

### Story 1.3: Markdown Renderer

As a developer,
I want a deterministic `render_todo_md(TodoList) -> String` function,
So that human-readable todo.md projections are always in sync with typed state.

**Acceptance Criteria:**

**Given** `render.rs` in `agent-diva-core::planning`
**When** I call `render_todo_md` with a TodoList containing items in various statuses
**Then** the output is valid markdown with sections for each status group (In Progress, Pending, Blocked, Completed, Canceled)
**And** each item shows: checkbox, title, priority badge, block reason (if blocked), evidence ref (if completed)
**And** items are ordered by `ordinal`
**And** the output is deterministic — same input always produces same output

**Given** `render.rs` also provides `render_plan_md(Plan, &[PlanStep]) -> String`
**When** I call it with a Plan and its steps
**Then** the output shows: title, goal, phase, strategy, assumptions, risks, open questions, and numbered steps with status

**Given** unit tests
**When** tests run
**Then** they cover: empty list, mixed statuses, blocked items with reasons, completed items with evidence
**And** all tests pass

### Story 1.4: Integration Validation

As a developer,
I want an end-to-end validation of types → store → render,
So that I can confirm the foundation works as a coherent unit before building on it.

**Acceptance Criteria:**

**Given** an integration test in `agent-diva-core::planning`
**When** I create a Plan with 3 PlanSteps and 5 TodoItems via the store
**And** update some items to various statuses
**And** call `render_todo_md` and `render_plan_md`
**Then** the rendered markdown reflects the current state
**And** reloading from the store after closing and reopening the pool produces identical results
**And** the active plan singleton correctly points to the created plan

---

## Epic 2: Plan Context & Read Path

**Goal:** Make the current plan and todo state visible to the agent loop via context injection and a read-only tool, enabling the model to understand what it should be working on.

**Delivers:** FR8, FR10, FR14 (HOOK-1), NFR7, NFR9

### Story 2.1: Context Injection

As an agent,
I want the active plan and todo state injected into my system prompt,
So that I know the current goal, strategy, and pending items without the user repeating them.

**Acceptance Criteria:**

**Given** `context.rs` in `agent-diva-agent::planning`
**When** `ContextBuilder` is assembling the system prompt
**And** an active plan exists in the planning store
**Then** a compact block is inserted after the Memory section and before tool guidance
**And** the block contains: plan goal, current phase, strategy snippet, current in-progress todo, pending items, blocked items
**And** the block does not exceed 800 characters
**And** completed/canceled items are excluded from the context block

**Given** no active plan exists
**When** `ContextBuilder` assembles the system prompt
**Then** no planning block is injected
**And** the rest of the prompt is unaffected

### Story 2.2: todo_show Tool

As an agent,
I want a `todo_show` read-only tool,
So that I can query the current todo state on demand during execution.

**Acceptance Criteria:**

**Given** `planning.rs` in `agent-diva-tools`
**When** the model calls `todo_show` with no arguments
**Then** the tool returns the full current TodoList as formatted text (title, status, priority for each item)
**And** the tool is read-only — it does not modify any state

**Given** `BuiltInToolsConfig.planning = true`
**When** `ToolAssembly` builds the tool list
**Then** `todo_show` is registered and available to the model
**And** when `planning = false`, the tool is not registered

### Story 2.3: HOOK-1 — System Message Injection

As a developer,
I want the agent loop to inject a planning system message before the iteration loop starts,
So that the model receives plan context on every turn.

**Acceptance Criteria:**

**Given** `loop_turn.rs` in `agent-diva-agent`
**When** a turn begins and an active plan exists
**Then** a system message containing the planning context block is injected before the first iteration
**And** the message uses `role: system` with a clear "Active Plan" header
**And** the injection happens once per turn, not per iteration

---

## Epic 3: Todo Mutations & NAG

**Goal:** Give the model the ability to create and revise the TodoList, and keep it on track with a gentle NAG mechanism when it drifts from pending items.

**Delivers:** FR9, FR11, FR14 (HOOK-2, HOOK-3, HOOK-4, HOOK-5), AR6

### Story 3.1: todo_write Tool

As an agent,
I want a `todo_write` full-replace tool,
So that I can create or revise the entire TodoList in one call.

**Acceptance Criteria:**

**Given** `planning.rs` in `agent-diva-tools`
**When** the model calls `todo_write` with a JSON array of todo items
**Then** all existing todo items for the active plan are replaced with the new list
**And** each item is validated: title required, status must be valid, blocked items must have block_reason
**And** a `TodoListRevised` event is appended
**And** the `todo.md` projection is regenerated
**And** the TodoList revision counter increments

**Given** the tool is called without an active plan
**Then** the tool returns an error: "No active plan. Start a plan first."

### Story 3.2: NAG Mechanism

As an agent,
I want a NAG reminder after 3 consecutive turns with pending todos but no planning tool calls,
So that I don't drift away from the TodoList.

**Acceptance Criteria:**

**Given** the agent loop is executing with an active plan that has pending TodoItems
**When** 3 consecutive model turns complete without any `todo_write` or `todo_show` tool call
**Then** a system message is injected: "You have pending TodoList items. Pick up the next one now."
**And** the NAG counter resets after any planning tool call
**And** the NAG does not fire if all items are completed/blocked/canceled

### Story 3.3: HOOK-2/3/4/5 — Agent Loop Integration

As a developer,
I want 4 additional agent loop hooks for planning integration,
So that planning state stays consistent with execution.

**Acceptance Criteria:**

**Given** HOOK-2 in `loop_turn.rs`
**When** the model returns a tool call matching `todo_write` or `todo_show`
**Then** the hook detects it as a planning tool call and updates the NAG counter

**Given** HOOK-3
**When** a planning tool execution completes
**Then** the hook detects state changes and triggers store sync

**Given** HOOK-4
**When** the session is about to be saved
**Then** the planning store is synced (dirty writes flushed, projections regenerated)

**Given** HOOK-5
**When** context consolidation/compaction runs
**Then** planning system messages are protected from removal

### Story 3.4: Mutation Integration Tests

As a developer,
I want integration tests for todo mutations and NAG,
So that the full write path is validated.

**Acceptance Criteria:**

**Given** an integration test
**When** I create a plan, write a TodoList via `todo_write`, execute items, block one, complete others
**Then** the store reflects all state changes
**And** the NAG mechanism fires after 3 turns without planning calls
**And** the NAG resets after a planning tool call

---

## Epic 4: Plan Lifecycle Management

**Goal:** Implement the PlanOrchestrator state machine that governs plan phase transitions, approval gates, and execution flow.

**Delivers:** FR12, FR13

### Story 4.1: PlanOrchestrator State Machine

As a developer,
I want a `PlanOrchestrator` that manages plan phase transitions,
So that plans follow the correct lifecycle (Explore→Plan→AwaitingApproval→Execute→Verify→Completed/Failed/Partial).

**Acceptance Criteria:**

**Given** `orchestrator.rs` in `agent-diva-agent::planning`
**When** a new plan is created
**Then** it starts in `Explore` phase
**And** `transition_to(PlanPhase::Plan)` is valid from `Explore`
**And** `transition_to(PlanPhase::AwaitingApproval)` is valid from `Plan`
**And** `transition_to(PlanPhase::Execute)` is valid from `AwaitingApproval` (requires approval)
**And** `transition_to(PlanPhase::Verify)` is valid from `Execute` (requires all non-canceled todos complete)
**And** terminal transitions to `Completed`, `Failed`, or `Partial` are valid from `Verify`
**And** invalid transitions return an error

### Story 4.2: Approval & Verification Gates

As a developer,
I want gate rules that enforce approval before execution and verification before completion,
So that plans cannot skip critical checkpoints.

**Acceptance Criteria:**

**Given** the orchestrator's approval gate
**When** a transition to `Execute` is attempted without prior approval
**Then** the transition is rejected with "Plan requires approval before execution"

**Given** the verification gate
**When** a transition to `Verify` is attempted with pending/in_progress TodoItems
**Then** the transition is rejected with "All todos must be completed, blocked, or canceled before verification"

**Given** `TodoPlanner` in `todo_planner.rs`
**When** the model generates a TodoList from PlanSteps
**Then** the generated items are validated and normalized before persisting

### Story 4.3: Plan Lifecycle Integration Tests

As a developer,
I want integration tests covering the full plan lifecycle,
So that phase transitions, gates, and edge cases are validated.

**Acceptance Criteria:**

**Given** integration tests
**When** I run a full lifecycle: create → explore → plan → approve → execute → verify → complete
**Then** all transitions succeed in order
**And** skipping a phase is rejected
**And** executing without approval is rejected
**And** verifying with pending todos is rejected

---

## Epic 5: Planning HTTP API

**Goal:** Expose plan state through the manager HTTP API so CLI, GUI, and external tools can read and manage plans.

**Delivers:** FR15, FR16, NFR10

### Story 5.1: ManagerCommand & PlanningService

As a developer,
I want `ManagerCommand` variants for planning and a lazy-created `PlanningService`,
So that the manager can handle planning requests through the existing command pipeline.

**Acceptance Criteria:**

**Given** `agent-diva-manager`
**When** I add `ManagerCommand` variants: `ListPlans`, `GetPlan`, `CreatePlan`, `UpdatePlan`, `DeletePlan`
**Then** each variant is routed to the corresponding `PlanningService` method
**And** `PlanningService` is lazy-created on first planning request (same pattern as `SkillService`)
**And** the service holds a reference to `SqlitePlanningStore`

### Story 5.2: REST Endpoints

As a developer,
I want REST endpoints for plan CRUD,
So that external clients can manage plans over HTTP.

**Acceptance Criteria:**

**Given** the manager HTTP router
**When** I send `GET /api/plans`
**Then** I receive a JSON array of plan summaries

**When** I send `POST /api/plans` with `{ title, goal }`
**Then** a new plan is created in `Explore` phase and the plan object is returned

**When** I send `GET /api/plans/:plan_id`
**Then** the full plan detail (with steps and todos) is returned

**When** I send `PUT /api/plans/:plan_id` with update fields
**Then** the plan is updated and the updated object is returned

**When** I send `DELETE /api/plans/:plan_id`
**Then** the plan and its associated data are deleted

### Story 5.3: Tauri GUI Commands

As a GUI developer,
I want Tauri commands for plan access,
So that the desktop app can display plan data.

**Acceptance Criteria:**

**Given** Tauri commands in `agent-diva-gui/src-tauri`
**When** the frontend calls `get_plans()`
**Then** a `PlanDto[]` is returned with: id, title, goal, phase, status, todo_count, todo_completed, is_active

**When** the frontend calls `get_plan(planId)`
**Then** a `PlanDetailDto` is returned with full steps, todos, and strategy

**When** the frontend calls `get_active_plan()`
**Then** the active plan detail is returned, or null if no active plan

### Story 5.4: API Tests

As a developer,
I want API integration tests,
So that the HTTP endpoints are validated end-to-end.

**Acceptance Criteria:**

**Given** integration tests using `wiremock` or `axum::test`
**When** I exercise all CRUD endpoints
**Then** correct status codes and JSON responses are returned
**And** error cases (not found, invalid input) return appropriate error responses

---

## Epic 6: Verification & Closure

**Goal:** Complete the plan lifecycle with verification recording and terminal state transitions.

**Delivers:** FR12 (terminal states), FR17 (complete event log)

### Story 6.1: PlanVerifier

As a developer,
I want a `PlanVerifier` that converts observations into a verification verdict,
So that plans have a structured completion assessment.

**Acceptance Criteria:**

**Given** `verifier.rs` in `agent-diva-agent::planning`
**When** all non-canceled TodoItems are completed
**And** verification evidence is recorded
**Then** `PlanVerifier` produces `VerificationVerdict::Pass`, `Fail`, or `Partial`
**And** the verdict is stored on the Plan record
**And** a `VerificationRecorded` event is appended

### Story 6.2: Terminal State Transitions

As a developer,
I want clean terminal state transitions that finalize the plan,
So that completed plans are properly archived.

**Acceptance Criteria:**

**Given** a plan in `Verify` phase with a recorded verdict
**When** the orchestrator transitions to a terminal state
**Then** the plan status is set to `Completed`, `Failed`, or `Partial`
**And** the `updated_at` timestamp is refreshed
**And** a `PlanCompleted`, `PlanFailed`, or `PlanPartial` event is appended
**And** the active plan singleton is cleared (allowing a new plan to be created)

### Story 6.3: Verification E2E Tests

As a developer,
I want end-to-end tests for the full plan lifecycle including verification,
So that the complete flow from creation to closure is validated.

**Acceptance Criteria:**

**Given** an E2E test
**When** I run: create plan → explore → draft → approve → execute all todos → verify → complete
**Then** the plan ends in `Completed` with `VerificationVerdict::Pass`
**And** the active plan is cleared
**And** all events are logged in order
**And** the final `todo.md` shows all items completed

---

## Epic 7: GUI Planning View

**Goal:** Add a standalone planning management page to the desktop GUI with plan list, active plan overview, and todo progress tracking.

**Delivers:** UX-DR5, UX-DR6, UX-DR7, UX-DR10

**Depends on:** Epic 4 (PlanOrchestrator must be functional for the view to have meaningful data)

### Story 7.1: PlanningView Page

As a user,
I want a dedicated Planning page accessible from the sidebar,
So that I can view and manage plans outside of the chat context.

**Acceptance Criteria:**

**Given** `PlanningView.vue` in `src/components/planning/`
**When** I navigate to the Planning section from the sidebar
**Then** a dual-pane layout is shown: left pane lists plans, right pane shows selected plan detail
**And** the active plan is highlighted in the list
**And** the sidebar shows a "Planning" entry with an appropriate icon

### Story 7.2: PlanStatusCard

As a user,
I want an active plan overview card showing key metrics at a glance,
So that I can quickly understand the current plan's progress.

**Acceptance Criteria:**

**Given** `PlanStatusCard.vue`
**When** an active plan exists
**Then** the card shows: plan title, current phase, total todos, completed count, blocked count
**And** a 4-grid status indicator displays the key metrics
**And** the card uses the project's status color system (pink for active, green for done, etc.)

### Story 7.3: TodoListPanel

As a user,
I want a todo progress panel grouped by status,
So that I can see what's done, what's in progress, and what's blocked.

**Acceptance Criteria:**

**Given** `TodoListPanel.vue` with `TodoItemRow.vue`
**When** the active plan has TodoItems
**Then** items are grouped by status: In Progress, Pending, Blocked, Completed
**And** each row shows: status icon, title, priority badge, evidence link (if completed), block reason (if blocked)
**And** a progress bar shows completed/total ratio
**And** the panel updates via polling (5s interval) when the PlanningView is open

### Story 7.4: GUI i18n & Integration

As a developer,
I want i18n strings and Tauri command integration for the planning view,
So that the feature works in both zh and en locales.

**Acceptance Criteria:**

**Given** `en.ts` and `zh.ts` locale files
**When** the planning view renders
**Then** all labels, tooltips, and status text are translated
**And** the Tauri commands (`get_plans`, `get_plan`, `get_active_plan`) are called from `desktop.ts`
**And** error states (no active plan, API failure) show appropriate user-facing messages

---

## Implementation Order Summary

```
Phase 1: Epic 1 (PR#1) — Types + Store + Render
Phase 2: Epic 2 (PR#2) + Epic 5 (PR#5) — Context + API (parallel)
Phase 3: Epic 3 (PR#3) — Todo Mutations + NAG
Phase 4: Epic 4 (PR#4) — PlanOrchestrator
Phase 5: Epic 6 (PR#6) — Verification & Closure
Phase 6: Epic 7 (GUI) — Planning View
```

**First PR to cut:** Epic 1, Story 1.1–1.4 (~300 lines Rust, zero user-facing change, pure foundation).
