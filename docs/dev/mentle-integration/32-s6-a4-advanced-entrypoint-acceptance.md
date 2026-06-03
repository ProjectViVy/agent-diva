# Sprint 6 A4: Advanced Entrypoint Acceptance

## Purpose

This record validates the Sprint 6 RC behavior for Mentle advanced entrypoints
without reopening feature scope.

The acceptance scope follows the frozen S6-A1 baseline:

- cron rebuild paths preserve Mentle tools
- `with_toolset()` keeps prompt activation aligned with the supplied registry
- subagents do not inherit Mentle by default
- runtime assembly stays consistent across advanced entrypoints

No runtime behavior, GUI control, tool-selection mode, dependency source, or
subagent inheritance behavior is changed by this record.

## Inputs Reviewed

- `29-s6-a1-rc-scope-baseline.md`
- `21-s4-a9-regression-test-baseline.md`
- `24-s4-a12-sprint4-review-package.md`
- `27-s5-a2-a6-failure-and-ci-hardening.md`
- `28-s5-a7-sprint5-review-package.md`
- `agent-diva-agent/src/agent_loop.rs`
- `agent-diva-agent/src/mentle_runtime.rs`
- `agent-diva-agent/src/tool_assembly.rs`
- `agent-diva-agent/src/tool_config/builtin.rs`
- `agent-diva-agent/src/subagent.rs`
- `justfile`

## Acceptance Matrix

| Advanced entrypoint | Expected RC behavior | Evidence | Result |
|---|---|---|---|
| Startup cron rebuild | When Mentle is active, startup rebuild through the cron service keeps the runtime custom tools in the registry | `AgentLoop::with_tools_and_memory_provider_inner(...)` builds tools with `custom_tools`, then rebuilds with the same `agent.custom_tools` when `cron_service` is present; `test_with_tools_startup_cron_preserves_mentle_custom_tools` passed in the Mentle feature lane | Accepted |
| Later default-tool rebuild | `register_default_tools(...)` keeps active Mentle tools and prompt state when rebuilding default tools | S4/S5 evidence names `test_register_default_tools_preserves_custom_tools_with_cron`; fresh S6 run passed. Mentle feature lane also passed `test_register_default_tools_rebuild_keeps_active_mentle_prompt` | Accepted |
| `with_toolset()` with `memtle_status` | Prompt routing is active only because the supplied registry contains `memtle_status`; no Mentle runtime is synthesized | `AgentLoop::with_toolset(...)` sets `mentle_active = toolset.registry.has("memtle_status")`, keeps `custom_tools` empty, and has no `mentle_runtime`; fresh `test_with_toolset` group passed | Accepted |
| `with_toolset()` without `memtle_status` | Configured Mentle or non-status `memtle_*` tools do not advertise L2 Palace Memory in the prompt | Fresh `test_with_toolset_missing_memtle_tool_disables_prompt` and `test_with_toolset_memtle_tool_without_status_disables_prompt` passed | Accepted |
| Subagent config | Subagents default Mentle, cron, spawn, and attachment to disabled for inherited execution safety | `BuiltInToolsConfig::for_subagent()` forces `mentle`, `cron`, `spawn`, and `attachment` to false; fresh `subagent_does_not_receive_mentle_by_default` passed | Accepted |
| Subagent registry | Subagent assembly drops parent custom tools, including Mentle custom tools, rather than filtering only Mentle names | `ToolAssembly::build_subagent_registry()` clears `custom_tools`, removes cron and spawner state, and builds subagent mode; fresh `test_tool_assembly_subagent_mode_excludes_mentle_custom_tools` passed | Accepted |
| Subagent prompt | Subagent prompt does not advertise Mentle routing or `memtle_*` tools | Fresh `test_build_subagent_prompt_omits_mentle_routing` passed | Accepted |
| Runtime active state | Runtime activation remains anchored on actual `memtle_status` registration, not config or toolkit-open success alone | `MentleRuntime::from_parts(...)` sets `active` from `custom_tools.iter().any(|tool| tool.name() == "memtle_status")`; fresh Mentle lane passed `test_mentle_runtime_active_matches_registered_status_tool` and `test_mentle_runtime_without_status_disables_prompt_routing` | Accepted |

## Fresh Verification

Run from `agent-diva/` on Windows PowerShell:

```powershell
$env:PATH = 'C:\Program Files\LLVM\bin;' + $env:PATH
just sprint5-default-check
cargo test -p agent-diva-agent --features mentle mentle
```

Result:

- `cargo check -p agent-diva-agent --no-default-features`: passed
- `cargo test -p agent-diva-agent test_with_toolset`: passed, 4 tests
- `cargo test -p agent-diva-agent test_tool_assembly_subagent_mode_excludes_mentle_custom_tools`: passed, 1 test
- `cargo test -p agent-diva-agent subagent_does_not_receive_mentle_by_default`: passed, 1 test
- `cargo test -p agent-diva-agent test_build_agent_tools_reuses_custom_tools_with_cron`: passed, 1 test
- `cargo test -p agent-diva-agent test_register_default_tools_preserves_custom_tools_with_cron`: passed, 1 test
- `cargo test -p agent-diva-agent test_build_subagent_prompt_omits_mentle_routing`: passed, 1 test
- `cargo test -p agent-diva-agent test_agent_loop_prefetch_failure_continues_without_recall_injection`: passed, 1 test
- `cargo test -p agent-diva-agent test_agent_loop_consolidation_sync_failure_keeps_main_response`: passed, 1 test
- `cargo test -p agent-diva-agent --features mentle mentle`: passed, 20 tests

The fresh run used the documented LLVM PATH prefix. No full workspace `just test`
was run for this S6-A4 record because S6-A1 and S5-A7 already classify the
existing provider export failure around `agent_diva_providers::ollama` as outside
the Mentle RC advanced-entrypoint gate unless Sprint 6 explicitly promotes full
workspace tests to a blocker.

## Blockers

None found for S6-A4.

The advanced entrypoints reviewed here do not show a prompt/tool split, unexpected
subagent inheritance, or cron rebuild loss of Mentle tools.

## Non-Blockers and Handoff Notes

- `with_toolset()` intentionally does not install a `HybridMemoryProvider`; this
  remains the external-registry boundary from S6-A1.
- Runtime dynamic-tool hot refresh remains out of RC scope; `MentleRuntime`
  active state is construction-time only.
- Local Windows Mentle feature checks require `clang-cl.exe` on PATH. This record
  used the documented `C:\Program Files\LLVM\bin` PATH prefix and the focused
  Mentle lane passed.
- Full workspace `just test` retains the pre-existing non-Mentle provider export
  issue recorded in Sprint 5.

## Acceptance Conclusion

S6-A4 is accepted for the RC handoff package.

Cron rebuild, `with_toolset()`, subagent, and runtime assembly paths preserve the
frozen Sprint 6 prompt/tool consistency contract. No RC blocker was identified,
and no code changes are required for this task.
