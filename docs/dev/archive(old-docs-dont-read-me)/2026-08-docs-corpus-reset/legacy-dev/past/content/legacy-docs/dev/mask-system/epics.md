---
stepsCompleted: [1, 2, 3, 4]
inputDocuments:
  - docs/dev/mask-system/prd.md
  - docs/dev/mask-system/.decision-log.md
status: complete
completedAt: 2026-06-10
---

# agent-diva Mask System — Epic Breakdown

> 恢复自 2026-06-04 session 记录。3 Epic / 13 Story，覆盖 30 条 FR。
> BMad Rubric 评审评级：Good。

---

## Epic 1: Mask Management & User-Facing Lifecycle

Users can discover, activate, switch, and manage masks through both CLI and GUI, with context-aware switching that preserves session continuity.

**FRs covered**: FR-1–FR-8, FR-19–FR-20, FR-22–FR-30

### Story 1.1: Define Mask Schema and Default Mask

As a DiVA user,
I want the system to define a clear mask file format and ship a built-in default mask,
So that the mask system has a stable foundation and I always have a fallback.

**Acceptance Criteria:**

**Given** the system starts
**When** mask schema is loaded
**Then** it shall support Markdown + YAML frontmatter with fields: name, icon, description, model, subagent_defaults, tool_limits

**Given** no masks exist in `workspace/masks/`
**When** the system initializes
**Then** the built-in default mask `"我就是我"` shall be available

**Given** the default mask
**When** inspected
**Then** it shall have no extra prompt, no tool limits, no model override

### Story 1.2: Build Mask Registry from Workspace Directory

As a DiVA user,
I want the system to scan `workspace/masks/` and build a registry of available masks,
So that I can see and use all masks I've created.

**Acceptance Criteria:**

**Given** `workspace/masks/` contains mask files
**When** the registry scans the directory
**Then** it shall parse all `.md` files with valid frontmatter
**And** support nested subdirectories (e.g., `coding/rust-coder.md`)

**Given** a mask file has invalid frontmatter
**When** the registry scans
**Then** it shall log a warning and skip the file
**And** continue loading other valid masks

**Given** the registry is built
**When** queried
**Then** it shall return a list of all valid masks with their metadata

### Story 1.3: Runtime Mask Activation via ContextBuilder

As a DiVA user,
I want activating a mask to inject its prompt into the system prompt,
So that the agent behaves according to the mask's role.

**Acceptance Criteria:**

**Given** a mask is activated
**When** the system prompt is assembled
**Then** the mask prompt shall be appended after SOUL/IDENTITY prompts
**And** the prompt hierarchy shall be: system base → 松本 core → mask

**Given** the default mask is active
**When** the system prompt is assembled
**Then** no additional prompt shall be injected

**Given** a mask specifies a model override
**When** the agent makes LLM calls
**Then** the mask's model shall take precedence over global default

### Story 1.4: Implement Mask Switching with Context Compression

As a DiVA user,
I want to switch masks seamlessly with clean context,
So that the agent doesn't carry stale behavior from a previous mask.

**Acceptance Criteria:**

**Given** I execute `/mask wear <name>`
**When** the switch occurs
**Then** the system shall first compress the current context
**And** inject a system message announcing the switch
**And** then inject the new mask's prompt

**Given** I execute `/mask off`
**When** the switch occurs
**Then** the system shall return to the default mask
**And** follow the same compress → inject → notify sequence

### Story 1.5: Add Mask Status Header to GUI

As a DiVA user,
I want to see the current mask in the GUI header,
So that I always know which mask is active.

**Acceptance Criteria:**

**Given** a mask is active
**When** the GUI header renders
**Then** it shall display the mask name and icon
**And** provide a dropdown to switch masks

**Given** no mask is active (default)
**When** the GUI header renders
**Then** it shall show the default mask name

### Story 1.6: Create, Edit, and Ship Predefined Masks

As a DiVA user,
I want to create and edit masks in the GUI and start from useful built-in mask patterns,
So that I can personalize mask behavior without manually authoring every file from scratch.

**Acceptance Criteria:**

**Given** the user enters the Masks settings view
**When** they choose to create a new mask
**Then** the GUI shall open a mask editor with sensible defaults
**And** allow editing of basic fields including name, icon, description, prompt, tools, skills, and model.

**Given** an existing workspace mask is selected for editing
**When** the editor opens
**Then** the current mask configuration shall be loaded into the editor
**And** saving shall write the updated Markdown + frontmatter back to disk.

**Given** the system ships predefined masks
**When** a fresh workspace enables the feature
**Then** researcher, coder, reviewer, and assistant mask definitions shall be available as first-party reference masks

---

## Epic 2: Safe Capability Modes & Runtime Enforcement

Users can trust masks and reviewer/assist mode to create real capability boundaries enforced by the runtime rather than soft prompt-only rules.

**FRs covered**: FR-9–FR-12, FR-18

### Story 2.1: Compute Effective Capabilities from Mask Policy

As a DiVA user,
I want mask tool policies to produce a clear effective capability set,
So that each mask exposes only the tools it is meant to use.

**Acceptance Criteria:**

**Given** global built-in tools are available
**When** a mask defines `allow` and `deny` lists
**Then** the runtime shall compute effective capabilities as `global_builtin ∩ allow − deny`
**And** the result shall be deterministic.

**Given** the effective capabilities for a mask are computed
**When** the model-facing tool list is assembled
**Then** only effective tools shall be exposed to the model for that mask.

### Story 2.2: Enforce Reviewer Assist Mode as True Read-Only Behavior

As a DiVA user,
I want reviewer mode to be truly read-only,
So that code review and audit tasks cannot accidentally mutate files or execute unsafe write-oriented actions.

**Acceptance Criteria:**

**Given** the reviewer mask is active
**When** runtime mode is resolved
**Then** the session shall use `AgentMode::Assist`
**And** reviewer behavior shall not rely on prompt wording alone.

**Given** the reviewer mask is active
**When** tool exposure is assembled
**Then** write-capable and mutation-capable tools shall be excluded
**And** only reviewer-approved read-only capabilities shall remain available.

### Story 2.3: Restrict Child Agents to Parent-Bounded Capabilities

As a DiVA user,
I want child agents to stay within the capability boundary of the current session,
So that delegation never becomes a hidden privilege escalation path.

**Acceptance Criteria:**

**Given** a parent session has an effective capability set
**When** a child agent is spawned
**Then** the child's capability set shall be derived as a subset of the parent effective capabilities
**And** any sub-agent policy shall only further reduce that set.

**Given** a child agent requests tools not available to the parent
**When** capability resolution occurs
**Then** those tools shall not be granted
**And** the spawn process shall not silently escalate privileges.

---

## Epic 3: Parallel Sub-Agent Orchestration

Users can spawn, observe, and collect structured results from parallel child agents that remain isolated, bounded, and model-flexible.

**FRs covered**: FR-13–FR-17, FR-21, FR-30

### Story 3.1: Define Batch Spawn and Structured Result Contracts

As a DiVA user,
I want child-agent spawning and result collection to use typed request and response contracts,
So that parallel delegation is reliable, inspectable, and easy for the GUI to consume.

**Acceptance Criteria:**

**Given** the runtime exposes batch delegation
**When** a caller constructs a batch spawn request
**Then** the request shall support multiple child tasks
**And** each task shall include at least task identity, task goal, and task context.

**Given** a child agent completes or terminates
**When** the runtime produces a result
**Then** it shall return a structured `SubAgentResult`
**And** that result shall include task id, status, summary, elapsed time, tool call count, token usage, and optional tool trace.

### Story 3.2: Execute Isolated Parallel Child Agents

As a DiVA user,
I want child agents to run in parallel as isolated workers,
So that I can delegate multiple tasks without leaking DiVA identity or creating cross-child coordination complexity.

**Acceptance Criteria:**

**Given** a batch spawn request contains multiple child tasks
**When** the runtime starts execution
**Then** child agents shall be able to run in parallel
**And** each child shall have its own isolated execution context.

**Given** a child agent is created
**When** its prompt and runtime context are assembled
**Then** it shall receive only the task/context provided by the parent
**And** it shall not inherit DiVA personality state as an autonomous identity.

**Given** one child fails while others continue
**When** the batch execution completes
**Then** the runtime shall return partial success results
**And** successful children shall not be discarded because one sibling failed.

### Story 3.3: Resolve Child Models and Runtime Limits Predictably

As a DiVA user,
I want child agents to use predictable model selection and execution limits,
So that delegation behavior stays controllable across masks and tasks.

**Acceptance Criteria:**

**Given** a child spawn request explicitly specifies a model
**When** the child runtime resolves model selection
**Then** the explicit spawn request model shall take highest priority.

**Given** no explicit child model is specified
**When** model selection is resolved
**Then** the runtime shall follow the documented fallback chain: child override → mask subagent_defaults → mask model → global default.

### Story 3.4: Stream Child Lifecycle Events to the Sub-Agent Panel

As a DiVA user,
I want to observe child-agent progress and outcomes in the GUI,
So that delegation remains understandable and trustworthy while work is in flight.

**Acceptance Criteria:**

**Given** child-agent execution begins
**When** lifecycle events are emitted
**Then** the runtime shall publish typed events for spawned, progress, completed, failed, timeout, and cancelled states.

**Given** the GUI sub-agent panel subscribes to runtime events
**When** child lifecycle events arrive
**Then** the panel shall render the current state of each child
**And** update without polling as new events arrive.

**Given** no child agents are active
**When** the panel is rendered
**Then** it shall display an empty state rather than stale or misleading content.
