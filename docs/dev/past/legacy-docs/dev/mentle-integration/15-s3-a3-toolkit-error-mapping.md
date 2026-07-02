# Sprint 3 A3: Toolkit Call Error Mapping

## Purpose

S3-A3 freezes how Agent-Diva interprets Mentle toolkit failures at the adapter and runtime activation boundary.

The goal is to keep runtime startup, dynamic tool registration, and `MemtleToolkitTool` execution on one shared error model instead of allowing each call site to invent separate fallback behavior.

## Frozen Error Model

Agent-Diva records Mentle failures with three internal dimensions:

- phase: where the failure happened
- category: what kind of failure it appears to be
- fallback action: what Agent-Diva does next

The internal phases are:

- `startup_open`: creating the memory directory or opening the palace database failed
- `tool_definition`: a dynamic tool definition is missing required metadata or has a non-object schema
- `tool_call_transport`: `MemtleToolkit::call_json(name, args).await` returned an error
- `tool_call_payload`: `call_json` returned a JSON payload that represents tool failure

The internal categories are:

- `io`
- `database`
- `json`
- `config`
- `invalid_arguments`
- `unknown_tool`
- `not_found`
- `invalid_definition`
- `tool_payload`
- `internal`

The fallback actions are:

- `disable_mentle`: startup/open failure keeps Mentle inactive and falls back to Markdown memory
- `skip_tool`: invalid dynamic definitions are skipped individually
- `return_tool_error`: toolkit execution failures are exposed as a normal Agent-Diva tool failure

## Agent-Diva Exposure

Tool execution errors are exposed to the model and runtime as:

- `ToolError::ExecutionFailed`

The adapter must not expose Mentle-specific error enum types through Agent-Diva public APIs. Structured phase/category/fallback details are diagnostic logging fields, not public provider-facing schema.

Startup failure does not fail Agent-Diva startup. It disables Mentle runtime activation and leaves Markdown memory as the active fallback.

Invalid tool definitions do not disable an otherwise-open Mentle runtime. Only the invalid definition is skipped.

Payload-level failures returned by `call_json` are treated like execution failures when the JSON contains either:

- an `error` string
- `success: false`

## Prompt Routing Safety

Opening the toolkit is not sufficient to advertise Mentle routing.

Startup prompt routing must enable `with_mentle(true)` only when the runtime's registered dynamic tool vector contains `memtle_status`. This matches the existing `with_toolset()` rule and prevents the prompt from instructing the model to call `memtle_*` tools when the registry cannot satisfy that contract.

If `MemtleToolkit::open()` succeeds but `memtle_status` is absent after dynamic registration, Agent-Diva may keep the valid custom tool vector registered, but Mentle prompt routing remains inactive and a warning is logged.

## Verification

The S3-A3 verification lane is:

```bash
cargo fmt
cargo check -p agent-diva-agent --no-default-features
cargo check -p agent-diva-agent --features mentle
cargo test -p agent-diva-agent --features mentle mentle
cargo test -p agent-diva-agent test_build_agent_tools_reuses_custom_tools_with_cron
cargo test -p agent-diva-agent test_register_default_tools_preserves_custom_tools_with_cron
cargo test -p agent-diva-core --features mentle memory
```

Policy checks:

```bash
rg -n "memtle = \{ version = \"0.1.2\", default-features = false \}" Cargo.toml agent-diva-agent/Cargo.toml agent-diva-core/Cargo.toml
rg -n "\[patch.crates-io\]|memtle.*path|memtle.*git" Cargo.toml agent-diva-agent/Cargo.toml agent-diva-core/Cargo.toml
```
