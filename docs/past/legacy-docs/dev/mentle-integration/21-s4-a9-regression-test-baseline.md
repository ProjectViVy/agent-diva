# Sprint 4 A9: Regression Test Baseline

## Purpose

S4-A9 freezes the Sprint 4 regression set for AgentLoop assembly paths that can
otherwise drift into prompt/tool mismatch.

This record extends the Sprint 3 verification baseline without reopening the
adapter/runtime contracts. It focuses on assembly, cron/default rebuild,
`with_toolset()`, subagent isolation, and prompt routing.

## Regression Scope

| Area | Required behavior | Evidence |
|---|---|---|
| Initial assembly | Custom Mentle tools remain available when registered through the shared assembly helper | `test_build_agent_tools_reuses_custom_tools_with_cron` |
| Cron/default rebuild | Rebuild paths preserve the existing custom tool vector | `test_register_default_tools_preserves_custom_tools_with_cron` |
| `with_toolset()` activation | Mentle prompt routing is enabled only when the supplied registry contains `memtle_status` | `test_with_toolset_memtle_status_enables_prompt` |
| `with_toolset()` inactive boundary | Configured Mentle or non-status Mentle tools do not enable prompt routing by themselves | `test_with_toolset_missing_memtle_tool_disables_prompt`, `test_with_toolset_memtle_tool_without_status_disables_prompt` |
| `with_toolset()` isolation | External registries remain outside runtime-owned custom tool and Mentle runtime state during construction | `test_with_toolset_keeps_external_registry_isolated_from_runtime_state` |
| Subagent config | Subagents default to Mentle, cron, spawn, and attachment disabled where inherited behavior would be unsafe | `subagent_does_not_receive_mentle_by_default` |
| Subagent registry | Subagent assembly excludes parent custom tools, including Mentle custom tools, and also excludes spawn/cron tools | `test_tool_assembly_subagent_mode_excludes_mentle_custom_tools` |
| Subagent prompt | Subagent prompt template does not advertise Mentle routing text or `memtle_*` tools | `test_build_subagent_prompt_omits_mentle_routing` |
| Prompt builder | Mentle prompt text follows explicit active state | `test_build_system_prompt_omits_mentle_routing_by_default`, `test_build_system_prompt_includes_mentle_routing_when_active` |

## Commands Run

```powershell
cargo fmt --all -- --check
cargo check -p agent-diva-agent --no-default-features
cargo test -p agent-diva-agent test_with_toolset
cargo test -p agent-diva-agent test_tool_assembly_subagent_mode_excludes_mentle_custom_tools
cargo test -p agent-diva-agent subagent_does_not_receive_mentle_by_default
cargo test -p agent-diva-agent test_build_agent_tools_reuses_custom_tools_with_cron
cargo test -p agent-diva-agent test_register_default_tools_preserves_custom_tools_with_cron
cargo test -p agent-diva-agent test_build_subagent_prompt_omits_mentle_routing
```

## Result

All default-lane S4-A9 regression checks passed.

The `with_toolset()` lane now explicitly locks the construction-time contract:
a registry containing `memtle_status` activates the prompt while the external
registry remains separate from runtime-owned custom tools and Mentle runtime
state. Subagent configuration remains Mentle-disabled even when the parent-facing
toolset enables Mentle.

## Acceptance

S4-A9 is accepted when:

- the default lane passes without requiring the `mentle` feature
- `with_toolset()` cannot activate Mentle prompt routing without `memtle_status`
- external registries remain external and do not populate `AgentLoop.custom_tools`
- subagent config, registry assembly, and prompt template all stay
  Mentle-isolated
- subagent registry assembly does not inherit parent custom tools; this is a
  deliberate isolation boundary, not a Mentle-only filter
- cron/default rebuild tests still prove custom tool preservation
