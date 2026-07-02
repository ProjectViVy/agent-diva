# Sprint 7 A1: Mentle Tool Selection and GUI Controls

## Purpose

Sprint 7 adds an enhanced operability layer for Mentle memory tools after the
Sprint 6 release-candidate baseline is complete.

This Sprint is not a first-phase production integration requirement. Sprint 1
through Sprint 6 remain focused on stable Mentle connection, runtime assembly,
failure handling, CI hardening, and RC handoff. Sprint 7 starts only after the
Sprint 6 RC baseline is accepted, so the release-readiness gate is not mixed
with optional tool-level configuration work.

## Theme

Mentle Tool Selection and GUI Controls.

The target outcome is:

- Agent-Diva can manage which `memtle_*` tools are assembled into an agent.
- The prompt only describes Mentle tools that are actually present in the
  runtime registry.
- `agent-diva-gui` exposes a General Settings section for enabling Mentle and
  choosing the active Mentle tool mode.
- The configuration format is durable enough for later per-tool GUI interaction
  without refactoring the runtime assembly contract.

## Scope

### In Scope

- Add a tool-selection policy at the Agent-Diva tool assembly boundary.
- Support a persisted Mentle configuration shape such as:

```toml
[mentle]
enabled = true
mode = "read_only"
allowed_tools = ["memtle_status", "memtle_search", "memtle_read"]
```

- Define supported modes:
  - `off`: no Mentle provider, prompt text, or `memtle_*` tools.
  - `read_only`: read/status/search tools only; mutation tools are excluded.
  - `full`: all valid dynamic `memtle_*` tools from the toolkit are eligible.
  - `custom`: only `allowed_tools` are assembled.
- Keep `memtle_status` as the minimum activation anchor for Mentle prompt
  routing.
- Filter dynamic `memtle_*` tools before they are passed into
  `build_agent_tools(...)`.
- Ensure prompt assembly derives Mentle capability from the post-filter registry,
  not from raw toolkit availability.
- Add a General Settings panel in `agent-diva-gui` for:
  - Mentle enabled/disabled state.
  - Mentle mode selection.
  - Per-tool activation when dynamic tool metadata is available.
  - Save/apply/reset behavior.
- Persist settings through the same configuration channel used by the rest of
  Agent-Diva desktop settings.
- Provide manual smoke tests for switching GUI settings and observing runtime
  prompt/tool consistency.

### Out of Scope

- Replacing the Sprint 1 through Sprint 6 production-readiness gates.
- Changing the published `memtle` package source policy.
- Adding local `mentle/` path or git dependency overrides.
- Designing a new memory model beyond L0/L1 Markdown memory and L2 Mentle
  Palace memory.
- Letting subagents inherit Mentle tools by default.
- Exposing hidden toolkit internals that are not present in
  `tool_definitions()`.

## Technical Design

### Configuration Model

Sprint 7 should introduce one configuration object that can be consumed by CLI,
service, runtime assembly, and GUI without separate translation rules.

Suggested Rust model:

```rust
pub struct MentleToolConfig {
    pub enabled: bool,
    pub mode: MentleToolMode,
    pub allowed_tools: Vec<String>,
}

pub enum MentleToolMode {
    Off,
    ReadOnly,
    Full,
    Custom,
}
```

Default behavior should preserve the Sprint 6 RC baseline:

- default builds still do not enable the `mentle` Cargo feature
- when Mentle is not configured, no Mentle runtime is opened
- when Mentle is configured but disabled, no prompt or tool exposure occurs
- when Mentle is enabled with no explicit GUI selection, the CLI/runtime default
  should follow the RC-approved behavior rather than silently exposing a broader
  tool set

### Tool Filtering Rules

Filtering must happen after dynamic toolkit metadata is loaded and before tools
are registered into the agent registry.

Rules:

- Always ignore non-`memtle_*` names in `allowed_tools`.
- Skip unknown tool names without failing the whole runtime.
- Preserve existing invalid-definition behavior: invalid toolkit definitions are
  skipped individually with warnings.
- `off` disables Mentle before prompt routing.
- `read_only` admits only the approved read/status allowlist.
- `full` admits every valid dynamic `memtle_*` tool.
- `custom` admits only names present in `allowed_tools`.
- If filtering removes `memtle_status`, Mentle prompt routing must remain
  inactive even when other Mentle tools are present.

The read-only allowlist should be documented and tested once the Sprint 6
baseline identifies which `memtle_*` tools are common and safe.

### Prompt and Tool Synchronization

The invariant from earlier Sprints remains mandatory:

> Prompt capability must be derived from the actual assembled registry.

Sprint 7 extends that invariant to user-selected tool subsets:

- Raw toolkit availability is not enough to activate prompt text.
- Saved GUI settings are not enough to activate prompt text.
- The post-filter registry must contain `memtle_status` before
  `with_mentle(true)` is allowed.
- If the user toggles a setting that removes required tools, the next runtime
  rebuild must also remove the matching prompt capability.

### GUI Behavior

The `agent-diva-gui` General Settings page should add a Mentle tools section.

Minimum controls:

- Mentle integration toggle.
- Mode selector: Off, Read only, Full, Custom.
- Dynamic checklist of discovered `memtle_*` tools when the toolkit can provide
  metadata.
- Disabled or explanatory state when the `mentle` feature or toolkit runtime is
  unavailable.
- Save/apply button that persists configuration and triggers the same runtime
  rebuild path used for other settings changes.
- Reset-to-default action.

The GUI should avoid presenting theoretical tools that were not reported by
`MemtleToolkit::tool_definitions()`.

### Runtime Rebuild Behavior

Runtime rebuilds must reuse the Sprint 4 and Sprint 6 assembly invariants:

- one shared registry assembly path
- no prompt/tool mismatch
- cron/default rebuilds preserve selected Mentle custom tools
- `with_toolset()` remains registry-driven
- subagents continue to default to Mentle disabled

Changing the GUI selection should invalidate and rebuild the active Mentle tool
set through the established runtime helper rather than mutating prompt text
directly.

## Work Breakdown

| ID | Work Item | Owner | Output |
|---|---|---|---|
| S7-A1 | Freeze Mentle tool-selection product boundary | A-ARCH | Accepted scope and non-goals |
| S7-A2 | Define persisted config model and defaults | A-CORE | Config schema, defaults, migration note |
| S7-A3 | Implement assembly-level `memtle_*` filtering | A-LOOP | Filtered custom tool vector before registry assembly |
| S7-A4 | Add prompt/tool synchronization tests | A-QA | Registry-derived prompt activation coverage |
| S7-A5 | Add GUI General Settings Mentle section | A-GUI | Toggle, mode selector, dynamic checklist |
| S7-A6 | Wire GUI persistence and runtime rebuild | A-GUI | Saved settings affect next active registry |
| S7-A7 | Manual GUI smoke and handoff package | A-QA | Verification record and review package |

## Acceptance Criteria

Sprint 7 is complete when:

- `mentle.enabled = false` produces no Mentle prompt text and no `memtle_*`
  tools.
- `read_only` exposes only the approved read/status subset.
- `full` exposes all valid dynamic `memtle_*` tools.
- `custom` exposes only selected valid tools.
- Unknown or unavailable tool names do not crash startup.
- Prompt routing activates only when the post-filter registry contains
  `memtle_status`.
- GUI settings persist across restart.
- Changing GUI settings triggers or schedules a runtime rebuild using the normal
  assembly path.
- Subagents still do not inherit Mentle tools by default.
- Manual GUI smoke testing records the behavior for off, read-only, full, and
  custom modes.

## Verification Plan

Minimum technical checks:

- Config serialization and deserialization tests for `MentleToolConfig`.
- Unit tests for filtering modes and unknown tool names.
- Agent assembly tests proving filtered tools reach the registry.
- Prompt tests proving filtered-out `memtle_status` keeps Mentle prompt routing
  inactive.
- Cron/default rebuild regression tests for selected Mentle tools.
- `with_toolset()` tests confirming prompt activation remains registry-driven.
- Subagent isolation regression tests.
- GUI smoke test for General Settings save, reload, and runtime effect.

Required command families:

- `just fmt-check`
- `just check`
- `just test`
- `cargo check -p agent-diva-agent --features mentle`
- targeted `agent-diva-gui` validation or smoke test documented in
  `verification.md`

## Entry Gate

Sprint 7 may start only when:

- Sprint 6 RC and handoff package are accepted.
- Sprint 1 through Sprint 6 production-readiness gates remain green or have
  explicit accepted exceptions.
- The team has identified the common `memtle_*` tools that should appear in the
  read-only preset.
- GUI settings persistence ownership is clear.

## Open Risks

- The published `memtle` toolkit may change tool names or metadata fields,
  requiring the GUI to tolerate missing or renamed tools.
- A per-tool GUI checklist can become misleading if it shows stale metadata from
  a previous runtime path.
- Tool subset changes can create partial capability states unless prompt routing
  continues to depend only on the assembled registry.
- Runtime rebuild UX may need progress/error feedback if toolkit open or dynamic
  metadata loading is slow.

## Implementation Status (S7-A1 through S7-A5)

Completed in this iteration:

- **Config model**: root `[mentle]` with `enabled`, `mode`, `allowed_tools`; defaults keep Mentle off.
- **Assembly filter**: `filter_mentle_tools()` applied after `tool_definitions()` conversion.
- **Prompt sync**: `ContextBuilder` uses post-filter registry state; `memtle_status` gates Mentle prompt routing.
- **Runtime refresh**: `RuntimeControlCommand::UpdateMentle` rebuilds Mentle tools and prompt state on config save.
- **GUI**: General Settings → Mentle card with enable/mode/custom checklist; persists via `/api/tools`.
- **Discovery API**: `GET /api/tools/mentle/available` lists discovered `memtle_*` tools when Mentle feature is enabled.

Deferred to S7-A6/A7:

- Full manual acceptance matrix execution and review package sign-off.
- Optional GUI progress feedback during slow Mentle toolkit open.
