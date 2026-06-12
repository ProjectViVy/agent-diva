# Main Closeout Cards (2026-06)

These cards convert the current `main` working tree into executable closeout steps.

Status legend:

- `todo` = not started
- `doing` = active split/cleanup work
- `done` = cleanly closed and ready to keep on `main`
- `moved-out` = intentionally excluded from `main` closeout

## MAIN-CLOSE-01 Runtime Safety Baseline

- Status: `done`
- Scope: circuit breaker, loop guard, context budget, overflow retry, subagent runtime guardrails, tool timeout plumbing
- Primary areas:
  - `agent-diva-agent/src/agent_loop.rs`
  - `agent-diva-agent/src/agent_loop/context_retry.rs`
  - `agent-diva-agent/src/agent_loop/loop_guard.rs`
  - `agent-diva-agent/src/agent_loop/loop_runtime_control.rs`
  - `agent-diva-agent/src/agent_loop/loop_tools.rs`
  - `agent-diva-agent/src/agent_loop/loop_turn.rs`
  - `agent-diva-agent/src/context_budget.rs`
  - `agent-diva-agent/src/loop_guard.rs`
  - `agent-diva-agent/src/subagent.rs`
  - `agent-diva-agent/src/subagent_policy.rs`
  - `agent-diva-agent/src/tool_assembly.rs`
  - `agent-diva-agent/src/tool_config/builtin.rs`
  - `agent-diva-tooling/src/registry.rs`
- Acceptance:
  - main-agent and subagent safety behavior are aligned
  - overflow-like provider failures retry once with stronger trimming
  - identical failing tool loops stop with an explicit reason
  - timeout behavior is enforced at the registry/runtime boundary
- Evidence:
  - `docs/logs/2026-06-agent-loop-safety/`
  - `docs/logs/2026-06-tool-timeout/`

## MAIN-CLOSE-02 Session Truth Source

- Status: `done`
- Scope: backend session durability, restore semantics, runtime state consistency, CLI/manager wiring needed for the same contract
- Primary areas:
  - `agent-diva-core/src/session/manager.rs`
  - `agent-diva-core/src/session/mod.rs`
  - `agent-diva-manager/src/runtime.rs`
  - `agent-diva-manager/src/state.rs`
  - `agent-diva-cli/src/chat_commands.rs`
  - `agent-diva-cli/src/main.rs`
  - `agent-diva-cli/tests/config_commands.rs`
  - `agent-diva-cli/tests/direct_chat_smoke.rs`
- Acceptance:
  - inbound/user/session state is persisted before downstream execution
  - save/load failure behavior is explicit instead of silently resetting state
  - runtime and manager carry the same session truth-source assumptions
- Evidence:
  - `docs/logs/2026-06-session-truth-source/`

## MAIN-CLOSE-03 Logging, Redaction, and Provider Safety

- Status: `done`
- Scope: redaction, error context, provider/http safety helpers, filesystem/tool safety adjustments, minimal manager hooks needed for those paths
- Primary areas:
  - `agent-diva-core/src/error.rs`
  - `agent-diva-core/src/error_context.rs`
  - `agent-diva-core/src/logging.rs`
  - `agent-diva-core/src/redaction.rs`
  - `agent-diva-providers/src/base.rs`
  - `agent-diva-providers/src/http_util.rs`
  - `agent-diva-providers/src/lib.rs`
  - `agent-diva-tools/src/filesystem.rs`
  - `agent-diva-files/src/manager.rs`
  - `agent-diva-manager/src/skill_service.rs`
  - `agent-diva-channels/tests/qq_reconnect_integration.rs`
  - `agent-diva-providers/tests/ollama_streaming.rs`
- Acceptance:
  - sensitive values are redacted in normal logging paths
  - provider/runtime failures carry actionable context without leaking secrets
  - any retained filesystem/provider changes are backend-safety changes, not product UI behavior
- Evidence:
  - `docs/logs/2026-06-log-redaction/`
  - `docs/logs/2026-06-observability/`

## MAIN-CLOSE-04 Backend Multimodal Boundary

- Status: `done`
- Scope: backend-only multimodal and embedded-server support needed on `main`
- Primary areas:
  - `agent-diva-manager/src/handlers.rs`
  - `agent-diva-manager/src/manager/runtime_control.rs`
  - `agent-diva-gui/src-tauri/src/embedded_server.rs`
  - `agent-diva-migration/src/config_migration.rs`
- Acceptance:
  - backend attachment/request boundary works without relying on frontend page changes
  - embedded gateway/server wiring remains valid on `main`
  - config migration covers any retained backend-only options
- Evidence:
  - `docs/logs/2026-06-multimodal-m1-contract/`
  - `docs/logs/2026-06-multimodal-prephase/`

## MAIN-CLOSE-05 Backlog and Documentation Closeout

- Status: `done`
- Scope: make closeout visible, finite, and auditable
- Primary areas:
  - `TODOLIST.md`
  - `docs/dev/awesomeagents/decisions.md`
  - `docs/dev/main-closeout-plan-2026-06.md`
  - `docs/dev/main-closeout-cards-2026-06.md`
- Acceptance:
  - `TODOLIST.md` links to the closeout plan and cards
  - excluded frontend/product files are explicitly marked as outside `main`
  - the repo has one authoritative document describing what remains on `main`

## MAIN-CLOSE-EXCLUDED Product/UI changes

- Status: `moved-out`
- Files:
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
- Rationale:
  - these are product/UI-facing changes and should not be closed inside `main`
  - they need either a `pro` destination or a separate product-facing cleanup pass
