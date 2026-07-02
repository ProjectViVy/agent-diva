# Sprint 3 A1: `MemtleToolkitTool` Interface Freeze

## Purpose

This record freezes the Agent-Diva boundary for adapting Mentle tools. It is the Sprint 3 input for `MemtleToolkitTool` implementation work and later dynamic registration work.

The goal is not to redesign the Mentle provider or runtime. The goal is to make the tool adapter contract explicit and keep the implementation narrow enough that future work can depend on it without rediscovering the boundary.

## Frozen Adapter Contract

Agent-Diva must discover Mentle tools only from:

- `MemtleToolkit::tool_definitions()`

Each tool definition may read only these fields:

- `name`
- `description`
- `inputSchema`

The `inputSchema` field is the JSON Schema object exposed by the upstream Mentle definition. Agent-Diva stores it as the tool's parameter schema. The provider-facing schema remains the existing Agent-Diva `Tool::to_schema()` format, where the Mentle `inputSchema` maps to:

```json
{
  "type": "function",
  "function": {
    "parameters": "<inputSchema>"
  }
}
```

Execution must go only through:

- `MemtleToolkit::call_json(name, args).await`

The shared toolkit handle is fixed as:

- `Arc<tokio::sync::Mutex<MemtleToolkit>>`

Toolkit call failures must be translated into:

- `ToolError::ExecutionFailed`

Return values are fixed as strings:

- JSON string payloads are returned directly.
- Non-string JSON payloads are serialized with pretty JSON formatting.

## Invalid Definition Handling

If a tool definition is missing `name`, `description`, or `inputSchema`, or if `inputSchema` is not a JSON object, Agent-Diva skips only that definition and logs a warning.

A successful `MemtleToolkit::open()` must not be downgraded to inactive solely because one tool definition is invalid.

## Internal Implementation Boundary

`MentleToolkitTool` remains an `agent-diva-agent` internal implementation detail. It is not a cross-crate public API.

Sprint 3 adapter code should stay concentrated around internal helpers equivalent to:

- `mentle_tool_metadata_from_definition(def) -> Option<(String, String, serde_json::Value)>`
- `mentle_tool_from_definition(def, toolkit) -> Option<Arc<dyn Tool>>`
- `mentle_tools_from_definitions(defs, toolkit) -> Vec<Arc<dyn Tool>>`

These helpers are allowed to evolve internally, but the adapter must not widen the public Agent-Diva interface with Mentle-specific MCP or CLI types.

## Non-Goals

This interface freeze explicitly does not:

- hard-code the full `memtle_*` tool list
- assert a fixed Mentle tool count
- depend on a local sibling `memtle/` checkout
- expose MCP or CLI-only Mentle types through Agent-Diva public APIs
- rewrite business semantics in the adapter layer
- fill missing arguments in the adapter layer
- route prompts in the adapter layer
- own provider lifecycle logic in the adapter layer

## Anchor Tools

Tests may use these tools only as availability anchors:

- `memtle_status`
- `memtle_search`
- `memtle_diary_write`

Tests must not assert the total number of Mentle tools returned by `tool_definitions()`.

## Verification

The Sprint 3 A1 verification lane is:

```bash
cargo check -p agent-diva-agent --no-default-features
cargo check -p agent-diva-agent --features mentle
cargo test -p agent-diva-agent --features mentle mentle
```

The default lane must remain Mentle-free. The Mentle lane is allowed to require the toolchain needed by the published `memtle 0.1.2` crate.
