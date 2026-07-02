# Main Closeout Plan (2026-06)

## Goal

Close the `main` line as a clean backend/runtime branch instead of leaving it as a mixed working tree.

This closeout does **not** mean "commit everything currently modified". It means:

- keep only `main`-owned backend, runtime, safety, and infrastructure work
- split mixed frontend/product changes out of this line
- convert the current mixed working tree into a sequence of clean commits
- leave a stable handoff trail in docs and `TODOLIST.md`

## Closeout rules

`main` should retain:

- agent loop and subagent runtime safety
- session truth-source and durability fixes
- logging, redaction, error-context, and provider/runtime safety support
- backend-only multimodal and embedded-server wiring
- config, migration, CLI, manager, and tests that directly support the above

`main` should not retain in this closeout round:

- frontend page composition
- chat/product UI behavior
- locale-only product copy changes
- long-form GUI planning notes that belong to product shaping

## Current working-tree classification

### Keep in `main`

- `agent-diva-agent/src/agent_loop.rs`
- `agent-diva-agent/src/agent_loop/context_retry.rs`
- `agent-diva-agent/src/agent_loop/loop_guard.rs`
- `agent-diva-agent/src/agent_loop/loop_runtime_control.rs`
- `agent-diva-agent/src/agent_loop/loop_tools.rs`
- `agent-diva-agent/src/agent_loop/loop_turn.rs`
- `agent-diva-agent/src/context_budget.rs`
- `agent-diva-agent/src/lib.rs`
- `agent-diva-agent/src/loop_guard.rs`
- `agent-diva-agent/src/skills.rs`
- `agent-diva-agent/src/subagent.rs`
- `agent-diva-agent/src/subagent_policy.rs`
- `agent-diva-agent/src/tool_assembly.rs`
- `agent-diva-agent/src/tool_config/builtin.rs`
- `agent-diva-cli/src/chat_commands.rs`
- `agent-diva-cli/src/main.rs`
- `agent-diva-cli/tests/config_commands.rs`
- `agent-diva-cli/tests/direct_chat_smoke.rs`
- `agent-diva-core/src/config/schema.rs`
- `agent-diva-core/src/config/validate.rs`
- `agent-diva-core/src/error.rs`
- `agent-diva-core/src/error_context.rs`
- `agent-diva-core/src/lib.rs`
- `agent-diva-core/src/logging.rs`
- `agent-diva-core/src/redaction.rs`
- `agent-diva-core/src/session/manager.rs`
- `agent-diva-core/src/session/mod.rs`
- `agent-diva-files/src/manager.rs`
- `agent-diva-gui/src-tauri/src/embedded_server.rs`
- `agent-diva-manager/src/handlers.rs`
- `agent-diva-manager/src/manager/runtime_control.rs`
- `agent-diva-manager/src/runtime.rs`
- `agent-diva-manager/src/skill_service.rs`
- `agent-diva-manager/src/state.rs`
- `agent-diva-migration/src/config_migration.rs`
- `agent-diva-providers/src/base.rs`
- `agent-diva-providers/src/http_util.rs`
- `agent-diva-providers/src/lib.rs`
- `agent-diva-providers/tests/ollama_streaming.rs`
- `agent-diva-tooling/Cargo.toml`
- `agent-diva-tooling/src/registry.rs`
- `agent-diva-tools/src/filesystem.rs`
- `agent-diva-channels/tests/qq_reconnect_integration.rs`

### Remove from this closeout round

- `agent-diva-gui/src/App.vue`
- `agent-diva-gui/src/components/ChatView.vue`
- `agent-diva-gui/src/components/NormalMode.vue`
- `agent-diva-gui/src/locales/en.ts`
- `agent-diva-gui/src/locales/zh.ts`
- `docs/dev/agent-plan/planning-gui-design-supplement.md`
- `docs/logs/2026-06-multimodal-gui-finish/`
- `docs/logs/2026-06-ui-transition-strategy/`
- `docs/design/`
- `docs/research/`

### Keep only as closeout metadata

- `TODOLIST.md`
- `docs/dev/awesomeagents/decisions.md`
- `docs/logs/2026-06-agent-loop-safety/`
- `docs/logs/2026-06-log-redaction/`
- `docs/logs/2026-06-session-truth-source/`
- `docs/logs/2026-06-tool-timeout/`

## Commit sequence

1. Runtime safety closeout
2. Session truth-source closeout
3. Logging/provider safety closeout
4. Backend multimodal boundary closeout
5. Docs and backlog closeout

Every closeout commit should contain:

- code for exactly one closeout theme
- matching tests or verification updates
- matching `docs/logs` references where that theme already exists

## Done condition

The `main` closeout is complete when:

- the working tree no longer mixes frontend/product changes with backend closeout work
- each backend theme can be explained as a single clean commit
- `TODOLIST.md` points to the closeout cards and no longer treats this as an open-ended mixed branch
