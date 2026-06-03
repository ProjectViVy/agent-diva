# Sprint 4 A1: AgentLoop Entry Audit

## Purpose

This record starts Sprint 4 by auditing the Sprint 3 review package, current
AgentLoop implementation, and test coverage before hardening runtime assembly
paths.

It does not reopen the Sprint 3 adapter or runtime contracts. Sprint 4 consumes
the frozen baseline and focuses on proving that every AgentLoop entrypoint keeps
Mentle prompt routing consistent with actual tool availability.

## Baseline Consumed from Sprint 3

Sprint 4 starts from the review package in
[18-s3-a8-sprint3-review-package.md](./18-s3-a8-sprint3-review-package.md).

The following assumptions are accepted as fixed:

- `MentleRuntime` is the internal owner of toolkit, runtime provider, reusable
  custom tools, and active state.
- `MemtleToolkit::tool_definitions()` is the only dynamic tool schema source.
- `MemtleToolkit::call_json(name, args).await` is the only generic adapter
  execution path.
- `memtle_status` is the activation anchor for Mentle prompt routing.
- Prompt routing must follow actual active state, not configuration alone.
- Custom Mentle tools must survive startup, cron, and default-tool rebuild paths.
- Subagents must not inherit Mentle long-term memory capability by default.

## Code Audit

| Sprint 4 item | Current implementation evidence | Audit result |
|---|---|---|
| S4-A2: initial AgentLoop assembly consumes `MentleRuntime` helpers | `AgentLoop::with_tools_and_memory_provider_inner` calls `MentleRuntime::try_build(...)`, then consumes `runtime.custom_tools()`, `runtime.active()`, and `runtime.memory_provider()` | Implemented; needs happy-path and rebuild coverage |
| S4-A3: `build_agent_tools(...)` is the single registry assembly helper | `build_agent_tools(...)` delegates built-in, MCP, cron, spawn, attachment, and custom tools through `ToolAssembly` | Implemented; keep as the single entrypoint |
| S4-A4: cron/default rebuild reuse `AgentLoop.custom_tools` | startup cron rebuild and `register_default_tools(...)` both pass `custom_tools.clone()` into `build_agent_tools(...)` | Implemented; needs entrypoint regression coverage |
| S4-A5: `ContextBuilder::with_mentle(...)` consumes active flag | `ContextBuilder` only stores a boolean and injects Mentle prompt text when it is true | Implemented |
| S4-A6: `with_toolset()` disables Mentle prompt without `memtle_status` | `with_toolset(...)` derives `mentle_active` from `toolset.registry.has("memtle_status")` | Implemented; needs boundary regression coverage |
| S4-A7: subagent isolation defaults Mentle off | `BuiltInToolsConfig::for_subagent()` sets `mentle: false`; `ToolAssembly::build_subagent_registry()` applies subagent mode | Implemented at config and registry layers; needs explicit Mentle custom-tool exclusion coverage |

## Existing Test Coverage

The default lane already covers these anchors:

- `test_build_agent_tools_reuses_custom_tools_with_cron`
- `test_register_default_tools_preserves_custom_tools_with_cron`
- `test_with_toolset_missing_memtle_tool_disables_prompt`
- `test_with_toolset_memtle_status_enables_prompt`
- `test_build_system_prompt_omits_mentle_routing_by_default`
- `test_build_system_prompt_includes_mentle_routing_when_active`
- `subagent_does_not_receive_mentle_by_default`

The Mentle feature lane already covers adapter and inactive-runtime scenarios,
but remains blocked on this Windows host when native `clang-cl.exe` is missing.

## Hardening Items for Sprint 4

Sprint 4 must still harden the following before closing:

- prove an injected active Mentle runtime makes initial AgentLoop registry and
  prompt routing active together
- prove startup cron rebuild preserves Mentle custom tools from the runtime
- prove `with_toolset()` keeps Mentle prompt disabled when `config.mentle=true`
  but the supplied registry lacks `memtle_status`
- prove subagent registry construction excludes Mentle custom tools even when a
  parent-like assembly path supplies them
- record validation results and blocked Mentle feature-lane evidence in the
  Sprint 4 iteration log

## Entry Audit Result

Sprint 4 may proceed. The interface baseline is stable, and the remaining work
is hardening and evidence capture rather than new adapter design.
