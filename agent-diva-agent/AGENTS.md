# agent-diva-agent

## OVERVIEW

This crate implements the agent loop runtime that drives LLM calls and tool invocations. It assembles context, loads skills, manages subagents, and builds the active tool registry for each turn.

## STRUCTURE

- `agent_loop/` — Turn runtime: admission, iteration, tool step, finalization, runtime control.
- `context/` — System prompt and message list assembly.
- `mask/` — Persona masks, registry, and tool policy filtering.
- `planning/` — Plan lifecycle hooks, orchestration, and todo tools.
- `tool_config/` — Built-in, network, and planning tool configuration.
- `compaction/` / `consolidation/` — Context summarization and memory boundary handling.

## WHERE TO LOOK

| Task | File |
|---|---|
| Agent loop entry, `AgentLoop::run`, turn dispatch | `agent_loop.rs` |
| Per-turn admission, iteration, tool execution | `agent_loop/turn/*.rs` |
| Runtime control command handling | `agent_loop/loop_runtime_control.rs` |
| System prompt + message list assembly | `context.rs`, `context/prompt.rs` |
| Skill discovery and loading | `skills.rs` |
| Background / supervised subagents | `subagent.rs`, `subagent_run_handler.rs` |
| Tool registry assembly | `tool_assembly.rs` |
| Built-in tool toggles (`BuiltInToolsConfig`) | `tool_config/builtin.rs` |
| Mask-based tool filtering | `mask/tool_policy.rs` |
| Plan phase policy mapping | `planning/mod.rs`, `planning/tools.rs` |
| `RuntimeControlCommand` definition | `runtime_control.rs` |

## CONVENTIONS

- Re-export public surface from `lib.rs`: `AgentLoop`, `AgentLoopToolSet`, `ToolConfig`, `AgentEvent`, `RuntimeControlCommand`, `SubagentSpawner`, `ToolAssembly`, `BuiltInToolsConfig`.
- `ToolAssembly::build()` produces the per-turn `ToolRegistry`; call `rebuild_tools_for_turn` when masks, plan phase, or approval policy change.
- `AgentLoop` constructs one `SubagentManager` shared across spawn tools and supervised runs.
- Context compaction and consolidation run against the `MemoryProvider` boundary, not legacy files.

## ANTI-PATTERNS

- Do not add gateway prefixes to native provider model IDs; that rule lives in providers, not here.
- Do not bypass `ToolAssembly` to register tools directly on the agent loop's registry.
- Do not treat legacy `SOUL.md`/`IDENTITY.md` files as prompt authority; consume them only through the memory provider boundary.
- Do not call `process_direct` or `process_direct_stream` without honoring `RuntimeControlCommand` channels.

## NOTES

- `agent_loop/turn/tool_step.rs` handles streaming LLM responses and parallel tool execution.
- `AgentLoopToolSet` is the preferred way to pre-build a tool registry for tests or custom embeddings.
