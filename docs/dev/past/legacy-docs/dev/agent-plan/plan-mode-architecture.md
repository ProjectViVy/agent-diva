# Plan Mode Runtime Architecture

## Background And Goal

This document defines the Plan Mode MVP architecture for agent-diva. The scope is deliberately narrow: Plan Mode is a user-approved, inspectable execution workflow for complex tasks. It does not include autodream, autonomous evolution, or a long-running rhythm engine.

The design consolidates lessons from three sources:

- Codex: collaboration modes, UI-visible state transitions, and the protocol for asking users before execution.
- GenericAgent: disciplined exploration, execution tracking, and verification before completion.
- agent-diva: typed Rust state, event bus integration, manager/CLI entry points, and durable local state.

The core decision is to implement Plan Mode as an agent-diva-native runtime instead of copying GenericAgent's Markdown regex state machine or its `_stop`, `_keyinfo`, and `_intervene` magic-file control protocol. Plan state should live under `.agent-diva/plans/<plan-id>/`, backed by `state.json`, `events.jsonl`, and typed Rust domain models.

## Non Goals

- No automatic complexity trigger in the MVP. Users or explicit CLI/API calls enter Plan Mode.
- No parallel plan execution.
- No GUI implementation in the MVP.
- No autonomous evolution, autodream, or long-term rhythm processing.
- No use of Laputa, Mentle, or `MemoryProvider` as the owner of the plan state machine.
- No provider-routing or model-ID behavior changes.

## Three-Way Responsibility Map

| Source | Keep | Do Not Copy |
| --- | --- | --- |
| Codex | Mode boundary, approval before execution, concise user-facing questions, progress updates | Product-specific UI assumptions |
| GenericAgent | Explore before plan, evidence per step, verify before completion, partial/failure accounting | Markdown regex state machine, magic files, untyped phase inference |
| agent-diva | Typed state, event bus, manager/CLI control plane, durable plan folders | Letting memory or unrelated subsystems own plan lifecycle |

## MVP Flow

The MVP lifecycle is:

```text
Explore -> Plan -> AwaitingApproval -> Execute -> Verify -> Completed
                                                       -> Failed
                                                       -> Partial
```

Phase intent:

- `Explore`: gather repository, command, and dependency context. This phase must produce `exploration_findings.md`.
- `Plan`: produce user-readable `plan.md` with steps, expected outputs, verification, and risks.
- `AwaitingApproval`: stop before any execution side effects. The user must approve or revise the plan.
- `Execute`: run approved steps serially and record evidence for each step.
- `Verify`: validate the result and write `verification.md`.
- `Completed`, `Failed`, `Partial`: terminal outcomes derived from the verification verdict, not from optimistic task narration.

## Directory Protocol

Plan Mode stores durable state in the workspace, scoped to agent-diva:

```text
.agent-diva/
  plans/
    active.json
    <plan-id>/
      input.md
      exploration_findings.md
      plan.md
      state.json
      events.jsonl
      verification.md
      evidence/
        step-001.md
        step-002.md
```

File responsibilities:

- `active.json`: optional pointer to the active plan ID and high-level phase.
- `input.md`: original user request and normalized task frame.
- `exploration_findings.md`: facts gathered before planning, including files inspected and assumptions.
- `plan.md`: approved or pending plan steps and validation strategy.
- `state.json`: canonical typed state snapshot.
- `events.jsonl`: append-only event stream for audit and recovery.
- `verification.md`: final validation commands, observations, and verdict.
- `evidence/step-*.md`: execution record for individual approved steps.

`state.json` is canonical for machine decisions. Markdown files are human-facing projections and review artifacts.

## Component Boundaries

### PlanOrchestrator

Owns lifecycle transitions, validates gate rules, delegates execution to the agent loop, and appends `PlanEvent` records. It should expose a small API such as `start_plan`, `approve_plan`, `execute_next`, `record_verification`, `pause`, `resume`, and `fail`.

### PlanStateStore

Owns the `.agent-diva/plans/` directory protocol. It reads and writes `state.json`, appends `events.jsonl`, writes human-facing artifacts, and performs recovery checks on startup.

### PlanVerifier

Converts validation observations into `PASS`, `FAIL`, or `PARTIAL`. It does not decide that a plan is complete without explicit evidence.

### AgentLoop

Executes approved work items. It must not transition from planning to execution by itself. It receives explicit steps from `PlanOrchestrator` and returns structured step results.

### SubagentManager

May assist exploration or execution in later phases, but MVP execution is serial. Subagents must not bypass approval or mutate plan state directly.

### MessageBus

Publishes plan lifecycle events for manager/CLI and future GUI consumers. It is an observation and coordination surface, not the persistence source of truth.

### MemoryProvider

May store summaries after a plan completes. It must not own active plan state, enforce phase transitions, or replace `PlanStateStore`.

### Manager And CLI

Provide user entry points:

- `plan start`: create a plan and run exploration.
- `plan approve`: approve the current `plan.md`.
- `plan status`: show active plan phase, pending step, and last event.
- `plan verify`: run or record verification.
- `plan cancel`: end a plan with an explicit canceled status.

The manager exposes the same operations over its HTTP control plane for future UI integration.

## Data Model Draft

```rust
pub struct Plan {
    pub id: PlanId,
    pub title: String,
    pub input_path: PathBuf,
    pub phase: PlanPhase,
    pub status: PlanStatus,
    pub steps: Vec<PlanStep>,
    pub verification: Option<VerificationVerdict>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct PlanStep {
    pub id: String,
    pub title: String,
    pub rationale: String,
    pub status: PlanStatus,
    pub evidence_path: Option<PathBuf>,
}

pub enum PlanStatus {
    Pending,
    InProgress,
    Blocked,
    Completed,
    Failed,
    Partial,
    Canceled,
}

pub enum PlanPhase {
    Explore,
    Plan,
    AwaitingApproval,
    Execute,
    Verify,
    Completed,
    Failed,
    Partial,
}

pub enum VerificationVerdict {
    Pass,
    Fail,
    Partial,
}

pub enum PlanEvent {
    Created,
    ExplorationStarted,
    ExplorationRecorded,
    PlanDrafted,
    ApprovalRequested,
    Approved,
    StepStarted { step_id: String },
    StepCompleted { step_id: String },
    StepFailed { step_id: String },
    VerificationRecorded { verdict: VerificationVerdict },
    Completed,
    Failed,
    Partial,
    Canceled,
}
```

The final implementation should avoid exposing filesystem paths across API boundaries unless the caller needs an artifact link. Stable plan IDs and artifact names are better public handles.

## Gate Rules

- Execution is forbidden before explicit user approval.
- A plan cannot enter `Plan` without `exploration_findings.md`.
- A plan cannot enter a terminal completion phase without `verification.md`.
- `verification.md` must include a verdict: `PASS`, `FAIL`, or `PARTIAL`.
- `FAIL` and `PARTIAL` are never auto-promoted to success.
- Each executed step must write an evidence artifact before it can be marked completed.
- Recovery must prefer `state.json` plus replayable `events.jsonl` over Markdown parsing.

## Recovery Semantics

On startup, `PlanStateStore` should read `.agent-diva/plans/active.json`, load the referenced `state.json`, and reconcile it with `events.jsonl`. If the snapshot and event log disagree, the runtime should mark the plan `Blocked` and require an explicit repair or cancel command.

For MVP, recovery can be conservative: resume only from `AwaitingApproval`, `Execute`, or `Verify` when all required artifacts exist. Otherwise, report the missing artifact and block.

## Implementation Roadmap

### Phase 0: Documents And Types

- Land this architecture document.
- Add Rust domain types behind a narrow module boundary.
- Add serialization tests for plan state and events.

### Phase 1: Single-Plan Serial Loop

- Implement `PlanStateStore`.
- Implement `PlanOrchestrator` for one active plan.
- Add CLI and manager endpoints for start, approve, status, verify, and cancel.
- Enforce approval and verification gates.

### Phase 2: Recovery, Pause, And Retry

- Add startup recovery.
- Add pause/resume.
- Add retry from failed or blocked steps with appended evidence.

### Phase 3: GUI And Policy Enhancements

- Add GUI views over manager APIs.
- Add optional policy controls for when Plan Mode is suggested.
- Add richer verification templates.

## Acceptance Criteria

- Plan Mode has a typed lifecycle and durable state protocol.
- Human-facing Markdown is treated as review/evidence, not the source of truth.
- The MVP cannot execute before approval.
- The MVP cannot complete without an explicit verification verdict.
- Manager/CLI boundaries are clear enough for implementation without coupling plan state to memory, Laputa, or Mentle.
