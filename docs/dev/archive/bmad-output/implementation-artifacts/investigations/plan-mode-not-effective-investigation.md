# Investigation: Plan mode not effective

## Hand-off Brief

1. **What happened.** User-reported manual testing found Plan mode ineffective; code evidence confirms the ChatView Plan selector only changes local UI state and is not sent to the backend.
2. **Where the case stands.** Root cause is identified with high confidence: planning primitives exist, but they are not wired into the agent runtime tool/config/context path.
3. **What's needed next.** Implement runtime wiring: propagate chat execution mode to backend, add planning config/store to `ToolConfig`, register planning tools, inject active plan context, and expose matching GUI/Tauri planning commands.

## Case Info

| Field | Value |
| --- | --- |
| Ticket | N/A |
| Date opened | 2026-06-20 |
| Status | Concluded |
| System | macOS workspace `/Users/mastwet/Desktop/morediva/agent-diva-pro`, Rust workspace |
| Evidence sources | Source code scan, targeted `cargo check -p agent-diva-agent` |

## Problem Statement

User asked to investigate why the current Plan mode does not take effect after manual testing failed.

## Evidence Inventory

| Source | Status | Notes |
| --- | --- | --- |
| `agent-diva-gui/src/components/ChatView.vue` | Available | Plan mode selector is local-only UI state. |
| `agent-diva-gui/src/App.vue` | Available | `send_message` invoke does not include execution mode. |
| `agent-diva-agent/src/agent_loop.rs` | Available | `ToolConfig` has no planning field. |
| `agent-diva-agent/src/tool_assembly.rs` | Available | Tool assembly registers filesystem/shell/web/spawn/mcp/cron/custom tools only. |
| `agent-diva-agent/src/planning/hooks.rs` | Available | Planning hooks exist but only tests call them. |
| `agent-diva-manager/src/manager.rs` | Available | Manager planning service is standalone CRUD over `.agent-diva/planning.db`. |
| `cargo check -p agent-diva-agent` | Available | Passed on 2026-06-20. |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --- | --- | --- | --- |
| 1 | Verify Plan mode UI propagation | High | Done | `execMode` is not emitted or sent to Tauri. |
| 2 | Verify planning tool registration | High | Done | No planning store/config reaches tool assembly. |
| 3 | Verify active plan context injection | High | Done | Hook exists, runtime call absent. |
| 4 | Verify GUI planning view integration | Medium | Done | Component exists, but commands/nav are not registered. |
| 5 | Add regression tests during fix | High | Open | Needed for mode propagation, tool registration, and context injection. |

## Confirmed Findings

### Finding 1: Chat Plan mode is local UI state only

**Evidence:** `agent-diva-gui/src/components/ChatView.vue:162`, `agent-diva-gui/src/components/ChatView.vue:227`, `agent-diva-gui/src/components/ChatView.vue:232`, `agent-diva-gui/src/components/ChatView.vue:447`, `agent-diva-gui/src/components/ChatView.vue:905`

**Detail:** `execMode` includes `'plan'`, the placeholder changes for Plan mode, and the dropdown mutates `execMode`. However `handleSend` emits only `(text, attachments)`, so the selected mode never leaves `ChatView`.

### Finding 2: Backend send path does not accept or forward execution mode

**Evidence:** `agent-diva-gui/src/App.vue:582`, `agent-diva-gui/src/App.vue:634`

**Detail:** `sendMessage` accepts only `content` and `attachments`, and Tauri `send_message` is invoked with message/channel/chatId/attachments/streamRequestId only. No `mode`, `exec_mode`, or planning flag reaches Rust.

### Finding 3: Agent runtime has no planning config slot

**Evidence:** `agent-diva-agent/src/tool_config/mod.rs:8`, `agent-diva-agent/src/tool_config/mod.rs:11`, `agent-diva-agent/src/agent_loop.rs:42`

**Detail:** `PlanningConfig` exists and says planning tools/hooks should be active when present, but `AgentLoop::ToolConfig` does not contain a `planning: Option<PlanningConfig>` field. CLI and Manager construct `ToolConfig` without planning state.

### Finding 4: Planning tools are not registered in the main tool assembly

**Evidence:** `agent-diva-agent/src/tool_assembly.rs:9`, `agent-diva-agent/src/tool_assembly.rs:127`, `agent-diva-agent/src/tool_assembly.rs:218`

**Detail:** `ToolAssembly` imports and registers standard tools plus custom tools. It does not import or register `PlanCreateTool`, `PlanShowTool`, `PlanApproveTool`, `PlanTransitionTool`, `TodoShowTool`, or `TodoWriteTool`.

### Finding 5: Planning context hooks exist but are not called by the agent loop

**Evidence:** `agent-diva-agent/src/planning/hooks.rs:16`, `agent-diva-agent/src/agent_loop/loop_turn.rs:150`

**Detail:** `inject_plan_context` can build an active-plan system block, but the turn code builds messages and proceeds to prefetch/LLM calls without invoking that hook.

### Finding 6: Manager planning API is separate from the agent runtime

**Evidence:** `agent-diva-manager/src/manager.rs:415`, `agent-diva-manager/src/manager.rs:464`

**Detail:** Manager lazily opens `.agent-diva/planning.db` for CRUD handlers, but this store is not shared into the `AgentLoop` planning hooks/tools. Plans created through the manager would not automatically affect chat turns.

### Finding 7: Planning GUI component is incomplete at integration boundary

**Evidence:** `agent-diva-gui/src/components/planning/PlanningView.vue:37`, `agent-diva-gui/src/components/planning/PlanningView.vue:60`, `agent-diva-gui/src-tauri/src/lib.rs:290`, `agent-diva-gui/src/components/NormalMode.vue:148`

**Detail:** `PlanningView` invokes `get_plans`, `get_plan`, and `get_active_plan`, but the Tauri invoke handler list does not register these commands, and `NormalMode` sidebar sections do not include a planning route.

## Deduced Conclusions

### Deduction 1: Manual Plan mode failure is expected from current wiring

**Based on:** Findings 1, 2, 3, 4, and 5.

**Reasoning:** The UI selector does not reach backend; even if it did, the agent loop has no planning config/store, no planning tool registration, and no active plan context injection.

**Conclusion:** Current Plan mode is a UI affordance and a set of unconnected domain primitives, not an effective runtime mode.

## Hypothesized Paths

### Hypothesis 1: The intended implementation stopped after scaffolding

**Status:** Confirmed

**Theory:** Planning models, store, tools, hooks, manager service, and GUI components were added in separate layers, but the final integration path was not completed.

**Supporting indicators:** Planning code compiles and has tests; runtime wiring points are absent.

**Would confirm:** Source evidence that scaffolding exists without integration.

**Would refute:** Evidence of a working mode propagation and store/tool/context path.

**Resolution:** Confirmed by source scan.

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | --- | --- |
| Exact manual-test steps | Could identify which visible symptom the user hit first. | Ask user or reproduce from GUI once a dev server/app launch path is chosen. |
| Full GUI compile status | Could reveal additional planning integration errors. | Run GUI-specific validation when implementing the fix. |

## Source Code Trace

| Element | Detail |
| --- | --- |
| Error origin | `ChatView.vue` execution mode local state and missing backend propagation. |
| Trigger | User selects Plan mode and sends a chat message. |
| Condition | `execMode` only changes placeholder/dropdown label; message payload remains normal chat payload. |
| Related files | `agent-diva-gui/src/App.vue`, `agent-diva-agent/src/agent_loop.rs`, `agent-diva-agent/src/tool_assembly.rs`, `agent-diva-agent/src/planning/hooks.rs`, `agent-diva-manager/src/manager.rs` |

## Conclusion

**Confidence:** High

Plan mode does not take effect because the runtime integration is missing. The repository has planning domain/storage/tool/hook scaffolding, but the visible Chat Plan selector is not propagated to backend, and the backend does not wire planning store/tools/context into normal agent turns.

## Recommended Next Steps

### Fix direction

1. Add an execution mode field to the GUI send event and Tauri `send_message` payload.
2. Add planning runtime configuration/store to `ToolConfig`.
3. Register planning tools when planning is enabled.
4. Call `inject_plan_context` before the LLM request when an active plan exists.
5. Decide whether Manager planning CRUD and AgentLoop planning store share the same `.agent-diva/planning.db`.
6. Register Tauri planning commands or remove/hide unfinished `PlanningView` until it is wired.

### Diagnostic

Add tests that fail on the current state:

- Chat mode propagation test for `execMode = 'plan'`.
- Tool assembly test that planning mode exposes `plan_create`, `todo_write`, `plan_transition`.
- Agent loop/context test that active plan text appears in messages sent to the provider.
- GUI command registration test or smoke test for `get_plans`/`get_active_plan`.

## Reproduction Plan

1. In the GUI, select Plan mode in the chat toolbar.
2. Send a message.
3. Observe that `send_message` payload lacks mode metadata.
4. Observe that the agent receives a normal chat turn with the standard tool set and no active-plan context injection.

## Side Findings

- `cargo check -p agent-diva-agent` passed, so the issue is not a current compile failure in the agent crate.
