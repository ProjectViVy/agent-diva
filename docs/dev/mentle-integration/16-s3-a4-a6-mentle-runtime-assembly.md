# Sprint 3 A4-A6: Mentle Runtime Assembly Boundary

## Purpose

This record freezes the Agent-Diva side runtime assembly boundary for Mentle.
It completes:

- S3-A4: design `MentleRuntime`
- S3-A5: define runtime lifecycle and ownership
- S3-A6: define the initial assembly helper interface

The intent is to give Sprint 4 a stable AgentLoop integration point without
reopening toolkit/provider/custom tool ownership.

## Runtime Shape

`MentleRuntime` is an internal `agent-diva-agent` type compiled only with the
`mentle` feature. It owns the reusable Mentle integration state:

- shared toolkit handle: `Arc<tokio::sync::Mutex<MemtleToolkit>>`
- memory provider: `Arc<dyn MemoryProvider>`
- custom tools: `Vec<Arc<dyn Tool>>`
- active flag: `bool`

The runtime is not a public cross-crate API. Agent-Diva public construction
continues to use existing `AgentLoop` and `AgentLoopToolSet` entrypoints.

## Ownership and Lifecycle

The toolkit is opened once during runtime assembly and shared by cloneable
`Arc` handles.

The same toolkit instance is used by:

- `HybridMemoryProvider`
- every dynamic `MentleToolkitTool`

`AgentLoop` owns `Option<MentleRuntime>` for the life of the loop. It also keeps
a cloned `custom_tools` vector so initial assembly, `register_default_tools()`,
and cron rebuilds use the same custom tool source.

If a caller explicitly injects a memory provider, that provider remains the
active AgentLoop provider. The Mentle runtime provider is still retained inside
the runtime for later reuse, but it does not override explicit injection.

## Active Flag Rule

The runtime active flag is true only when dynamic tool registration contains
`memtle_status`.

Toolkit open success alone is not enough to enable Mentle prompt routing. If the
toolkit opens but `memtle_status` is absent, Agent-Diva may still register valid
Mentle tools, but `ContextBuilder::with_mentle(false)` must be used.

Startup/open failures disable Mentle and fall back to Markdown memory. Invalid
tool definitions skip only the invalid definition.

## Frozen Helper Interface

The internal helper interface is:

```rust
MentleRuntime::try_build(workspace: &Path) -> impl Future<Output = Option<MentleRuntime>>
runtime.memory_provider() -> Arc<dyn MemoryProvider>
runtime.custom_tools() -> Vec<Arc<dyn Tool>>
runtime.active() -> bool
```

AgentLoop assembly must consume runtime state through these helpers.

`build_agent_tools(...)` remains the single registry assembly helper for built-in
tools, MCP tools, cron, spawn, attachments, and Mentle custom tools.

`AgentLoopToolSetBuilder::with_tools(...)` remains the stable external assembly
hook for prebuilt custom tools. It does not rediscover Mentle tools.

## Sprint 4 Contract

Sprint 4 AgentLoop and cron work should:

- build Mentle runtime once during initial assembly
- pass `runtime.custom_tools()` into the existing tool assembly path
- preserve `AgentLoop.custom_tools` across cron/default tool rebuilds
- drive prompt routing from `runtime.active()` or a supplied registry containing
  `memtle_status`
- avoid direct calls to `MemtleToolkit::open()` outside the runtime helper

## Verification Lane

```bash
cargo fmt
cargo check -p agent-diva-agent --no-default-features
cargo check -p agent-diva-agent --features mentle
cargo test -p agent-diva-agent --features mentle mentle
cargo test -p agent-diva-agent test_build_agent_tools_reuses_custom_tools_with_cron
cargo test -p agent-diva-agent test_register_default_tools_preserves_custom_tools_with_cron
cargo test -p agent-diva-core --features mentle memory
```
