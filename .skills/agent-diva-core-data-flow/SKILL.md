---
name: agent-diva-core-data-flow
description: Inbound/outbound message flow, message bus, session persistence, and memory files in agent-diva. Use when modifying agent-diva-core (bus, config, sessions), agent-diva-agent (agent loop, context builder, skills/subagents), or tracing a message from channel to LLM and back. Not for HTTP route surface—that is agent-diva-manager-gateway.
---

# Core data flow

## Pipeline (end-to-end)

1. **Channel handler** (`agent-diva-channels`) receives a user message and pushes an **`InboundMessage`** onto the **inbound** side of the bus.
2. **Agent loop** (`agent-diva-agent`, e.g. `src/agent_loop.rs` and `agent_loop/` submodules) consumes inbound work, runs turns, and calls the LLM via **`agent-diva-providers`**.
3. **Context** (`agent-diva-agent/src/context.rs`) assembles prompts: session history, skills, consolidation, memory snippets—follow existing ordering and token limits when changing behavior.
4. **Tools** (`agent-diva-tools`): the loop registers executable tools (see **`agent-diva-agent/src/agent_loop/loop_tools.rs`**) and feeds tool results back into the model turn.
5. **Outbound** messages go through the bus; the **channel** delivers replies to the user.

Bus implementation lives under **`agent-diva-core/src/bus/`** (`mod.rs`, queues, events).

## Persistence and memory

- **Sessions:** JSONL and session lifecycle are owned by **`agent-diva-core`** session management; paths derive from config/workspace. Changing record shapes can break existing user data—prefer additive fields with serde defaults.
- **Long-term memory:** project convention uses **`MEMORY.md`** and append-only **`HISTORY.md`** where configured; keep append/merge semantics consistent with existing helpers.

## Boundaries

- Put **shared types and bus contracts** in **`agent-diva-core`**; avoid letting `agent-diva-channels` depend on `agent-diva-agent` or vice versa except through core abstractions.
- **Subagents** (`agent-diva-agent/src/subagent.rs`) use a smaller tool set and their own loop—if you change global tool registration, check whether subagents need the same tool.

## Validation

- `cargo test -p agent-diva-core` and `cargo test -p agent-diva-agent` after bus, session, or loop changes.
- Full workspace: follow **`agent-diva-workspace-validate`**.

## Related skills

- **`agent-diva-extend-integrations`**: adding channels, tools, or providers that plug into this flow.
- **`agent-diva-manager-gateway`**: HTTP control plane that configures runtime but is not the bus itself.
