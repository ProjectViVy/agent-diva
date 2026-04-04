---
name: agent-diva-extend-integrations
description: Guide for extending Agent Diva with new LLM providers, chat channels, or tools. Use when adding or modifying integrations in agent-diva-providers (LLMProvider, providers.yaml, ProviderRegistry), agent-diva-channels (ChannelHandler, ChannelManager, lib.rs exports), agent-diva-tools (Tool trait), or agent-diva-agent (where ToolRegistry is populated). Covers model-ID safety for native vs LiteLLM gateways and where to add tests.
---

# Extend integrations (Provider / Channel / Tool)

## Provider (`agent-diva-providers`)

1. Implement HTTP logic against `LLMProvider` in `agent-diva-providers/src/base.rs` (and a dedicated module if the provider is large). Reuse one `reqwest::Client` per provider instance or shared where the codebase already does.
2. **Built-in catalog:** append/update `agent-diva-providers/src/providers.yaml`. `ProviderRegistry` embeds it via `include_str!`; fix `keywords`, `litellm_prefix`, `skip_prefixes`, `default_api_base`, `models`, and env key fields so `find_by_model` / UI catalog stay correct.
3. **Runtime resolution:** `ProviderCatalogService` (`catalog.rs`) merges YAML specs with `agent-diva-core` `Config` (built-in + `custom_providers`). Changing config shape requires compatible serde defaults and migration awareness.
4. **Model ID safety:** On a provider’s **native** OpenAI-compatible base URL, outbound requests must use the **raw** model id (e.g. `deepseek-chat`). Do **not** rewrite to LiteLLM-style `vendor/model` unless the request truly goes through a LiteLLM-style gateway. Add or update tests that assert the final JSON `model` field when you touch routing or prefix logic.

### Provider checklist

- [ ] `providers.yaml` entry complete and parseable
- [ ] Native vs gateway behavior documented in code/tests
- [ ] `cargo test -p agent-diva-providers` passes

## Channel (`agent-diva-channels`)

1. Implement `ChannelHandler` (`src/base.rs`): start/stop, map platform events ↔ `agent_diva_core::bus::{InboundMessage, OutboundMessage}`.
2. Add a module under `agent-diva-channels/src/<platform>.rs` and export it from `agent-diva-channels/src/lib.rs`.
3. **Wire in `ChannelManager`:** `agent-diva-channels/src/manager.rs` — extend `build_updated_handler` and any `match name` / registration lists so the new channel is constructed when `Config` is ready (mirror an existing channel: enable flag + required non-empty secrets).
4. **Config schema:** extend `agent-diva-core/src/config/schema.rs` (channels section and related) so JSON config and `AGENT_DIVA__...` env overrides deserialize; keep backward-compatible defaults.

### Channel checklist

- [ ] Handler registered in `manager.rs` with same readiness pattern as peers
- [ ] `lib.rs` exposes the module
- [ ] Core `Config` includes new channel block
- [ ] `cargo test -p agent-diva-channels` passes

## Tool (`agent-diva-tools` + `agent-diva-agent`)

1. Implement `Tool` in `agent-diva-tools` (`src/base.rs` trait; follow `filesystem.rs`, `shell.rs`, `web.rs`, etc.). Input/output shapes: `Serialize`, `Deserialize`, `Debug`, `Clone`; implement `name()`, `to_schema()`, `validate_params`, `execute`.
2. **`ToolRegistry::register` is not enough alone:** the agent builds the registry at runtime. Add `tools.register(Arc::new(YourTool::...))` in the same places as existing tools:
   - `agent-diva-agent/src/agent_loop.rs` and `agent-diva-agent/src/agent_loop/loop_tools.rs` (main loop / tool assembly)
   - `agent-diva-agent/src/subagent.rs` if subagents should see the tool
3. Respect tool enable flags / `Config` if other tools do (e.g. web tools gated on config).
4. Secrets: prefer `secrecy::SecretString` or config-only usage; never log raw tokens.

### Tool checklist

- [ ] Tool type lives in `agent-diva-tools`
- [ ] Registered in `loop_tools.rs` / `agent_loop.rs` (and `subagent.rs` if needed)
- [ ] Schema name matches what the LLM will call
- [ ] `cargo test -p agent-diva-tools` and `cargo test -p agent-diva-agent` pass

## Cross-cutting

- Dependencies: root `Cargo.toml` `[workspace.dependencies]` + `{ workspace = true }` in the member crate only.
- Errors: `thiserror` in libraries; `anyhow::Context` at CLI/gateway boundaries.
- Validation: run **`agent-diva-workspace-validate`** (`just ci` or fmt + clippy + test).

## Related skills

- **`agent-diva-rust-dev`** (`.cursor/skills/agent-diva-rust-dev/SKILL.md`): Rust/async/error-handling depth.
- **`agent-diva-core-data-flow`**: bus and agent loop placement.
- **`agent-diva-manager-gateway`**: HTTP surface for config/skills/tools exposed by the gateway.
