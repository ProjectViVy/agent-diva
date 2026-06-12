# Sprint 3 A2: Dynamic Tool Registration Model

## Purpose

S3-A2 freezes the dynamic registration model for Mentle tools in Agent-Diva.

This is a design-solidification deliverable. It does not add a new public Rust API and does not change the S3-A1 adapter boundary. Its output is the reusable custom tool vector model that S3-A3 runtime assembly work must consume.

## Inputs and Outputs

Input dependencies:

- S2-A8 interface baseline: [12-s2-a8-sprint2-review-and-s3-interface-baseline.md](./12-s2-a8-sprint2-review-and-s3-interface-baseline.md)
- S3-A1 adapter freeze: [13-s3-a1-memtle-toolkit-tool-interface.md](./13-s3-a1-memtle-toolkit-tool-interface.md)
- published `memtle 0.1.2` with `default-features = false`

Output for S3-A3:

- a reusable `Vec<Arc<dyn Tool>>` containing valid dynamic `memtle_*` tools
- one source of truth for initial tool assembly and later rebuild paths
- no fixed Mentle tool count, copied Mentle tool list, local checkout dependency, or provider-specific schema fork

## Source Boundary

Agent-Diva must source Mentle tools only from the published Cargo package:

```toml
memtle = { version = "0.1.2", default-features = false }
```

The dynamic schema source is only:

- `MemtleToolkit::tool_definitions()`

The implementation must not use:

- a local `D:\newdev\new-mentle\memtle` checkout
- `path` dependency overrides
- `git` dependency overrides
- `[patch.crates-io]` overrides
- a copied Mentle tool manifest
- a hard-coded full `memtle_*` tool list

## Registration Flow

The frozen registration flow is:

1. Open the toolkit with `MemtleToolkit::open(path).await`.
2. Read dynamic definitions with `toolkit.tool_definitions()`.
3. For each definition, map only valid metadata into a `MentleToolkitTool`.
4. Collect mapped tools into a reusable `Vec<Arc<dyn Tool>>`.
5. Pass that vector into the same custom-tool assembly path used by runtime startup and future rebuild work.

In pseudocode:

```rust
let toolkit = MemtleToolkit::open(path).await?;
let toolkit = Arc::new(tokio::sync::Mutex::new(toolkit));
let definitions = toolkit.lock().await.tool_definitions();
let tools = mentle_tools_from_definitions(definitions, toolkit.clone());
```

S3-A3 must treat `tools` as the reusable Mentle custom tool vector. Runtime startup, cron rebuild, and `with_toolset()` safety work should reason from this vector rather than re-discovering or re-declaring Mentle tools independently.

## Definition Mapping

Each Mentle definition may consume only:

- `name`
- `description`
- `inputSchema`

`name` maps to `Tool::name()`.

`description` maps to `Tool::description()`.

`inputSchema` maps to `Tool::parameters()`.

Agent-Diva then relies on the existing `Tool::to_schema()` path to produce the provider-facing function schema. S3-A2 does not introduce a provider-specific Mentle schema conversion path.

Conceptually, the provider-facing schema remains:

```json
{
  "type": "function",
  "function": {
    "name": "<name>",
    "description": "<description>",
    "parameters": "<inputSchema>"
  }
}
```

The adapter must not read Mentle-specific fields beyond the three frozen metadata fields. Future upstream fields can exist without becoming part of the Agent-Diva integration contract.

## Execution Flow

Tool execution remains fixed to:

- `MemtleToolkit::call_json(name, args).await`

The registered Agent-Diva tool stores the dynamic Mentle `name`, forwards the user/provider arguments unchanged as JSON, and returns the toolkit result as the tool result string according to the S3-A1 adapter freeze.

S3-A2 does not allow:

- filling missing arguments in Agent-Diva
- translating business semantics outside Mentle
- routing execution through MCP or CLI-only Mentle paths
- replacing `call_json()` with per-tool custom Rust calls

## Invalid Definition Handling

Invalid definition handling is local to the bad definition:

- missing `name`: skip that definition and warn
- missing `description`: skip that definition and warn
- missing `inputSchema`: skip that definition and warn
- non-object `inputSchema`: skip that definition and warn

A successful `MemtleToolkit::open()` must not be downgraded to inactive just because one definition is invalid. The resulting `Vec<Arc<dyn Tool>>` contains only valid mapped tools.

If `MemtleToolkit::open()` itself fails, Mentle runtime remains inactive and no `memtle_*` tools are exposed. That failure mode belongs to runtime activation, not dynamic definition validation.

## Acceptance Boundary

S3-A2 is accepted when the design and project tracker record that:

- no new public Rust API is required
- tool registration is dynamic from `tool_definitions()`
- tests may use anchor tools but must not assert a fixed total tool count
- the adapter consumes only `name`, `description`, and `inputSchema`
- `inputSchema` is passed through `Tool::parameters()` and then `Tool::to_schema()`
- execution goes through `call_json(name, args).await`
- invalid definitions are skipped individually with warnings
- the runtime uses published `memtle 0.1.2`, not a local checkout or override
- S3-A3 depends on the reusable custom tool vector model for startup, cron rebuild, `with_toolset()`, and active prompt routing

## Verification

Static checks:

```bash
rg -n "fixed .*tool count|assert.*tool count|hard-code.*memtle" docs/dev/mentle-integration
rg -n "memtle = \\{ version = \"0.1.2\", default-features = false \\}" Cargo.toml agent-diva-agent/Cargo.toml agent-diva-core/Cargo.toml
rg -n "name = \"memtle\"|source = \"registry\\+https://github.com/rust-lang/crates.io-index\"|version = \"0.1.2\"" Cargo.lock
rg -n "\\[patch.crates-io\\]|memtle.*path|memtle.*git" Cargo.toml agent-diva-agent/Cargo.toml agent-diva-core/Cargo.toml
```

Build and test lane:

```bash
cargo check -p agent-diva-agent --no-default-features
cargo check -p agent-diva-agent --features mentle
cargo test -p agent-diva-agent --features mentle mentle
```

The default/no-default feature lane must pass without Mentle. The Mentle lane may require the native toolchain prerequisites of the published `memtle 0.1.2` dependency; on Windows that includes a working `clang-cl.exe` for its native dependency chain.
