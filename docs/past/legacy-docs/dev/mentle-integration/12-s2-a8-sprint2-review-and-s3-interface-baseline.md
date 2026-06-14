# Sprint 2 A8: Review Package and Sprint 3 Interface Baseline

## 1. Purpose

This record closes Sprint 2 at the planning and interface level. It packages the current provider work for review and freezes the interfaces that Sprint 3 runtime and tool-adapter work must consume.

Sprint 3 must treat the code and contracts listed here as the alignment target. It should not redesign the provider boundary, dependency source rule, or toolkit call shape unless a separate architecture review changes this baseline.

## 2. Review Scope

Sprint 2 is review-ready when the following are true:

- the published `memtle 0.1.2` package source rule is recorded and enforced
- the `agent-diva-core/mentle` and `agent-diva-agent/mentle` feature boundaries remain explicit
- `HybridMemoryProvider` is the provider bridge for Markdown memory plus Mentle Palace memory
- `system_prompt_block()` remains synchronous and reads only cached or local state
- async Mentle I/O is limited to lifecycle hooks that already run on async paths
- Markdown memory remains the authoritative fallback when Mentle open, query, or secondary write paths fail

This package is documentation and review packaging. It does not introduce new Rust behavior.

## 3. Sprint 2 Acceptance Matrix

| Acceptance item | Baseline evidence | Review result |
|---|---|---|
| Default build does not require Mentle | Workspace and crate features keep `memtle` optional behind `mentle` | Ready for verification |
| Mentle dependency uses published package | Workspace dependency is pinned to `memtle = { version = "0.1.2", default-features = false }` | Ready for verification |
| No local Mentle source coupling | Policy forbids `path`, `git`, and `[patch.crates-io]` overrides for `memtle` | Ready for verification |
| Prompt path does not block on DB I/O | `MemoryProvider::system_prompt_block()` contract is synchronous; `HybridMemoryProvider` renders cached Palace snapshot state | Ready for verification |
| Markdown fallback remains authoritative | `HybridMemoryProvider` delegates file memory to `MemoryManager` and treats Mentle diary write failure as degraded secondary persistence | Ready for verification |
| Prefetch uses async recall path | `MemoryProvider::prefetch()` is the async recall seam and maps intent to Mentle search behavior | Ready for verification |
| Turn sync uses async persistence path | `MemoryProvider::sync_turn()` is the async persistence seam and can write Markdown plus Mentle diary entries | Ready for verification |
| Session shutdown remains idempotent | `MemoryProvider::on_session_end()` remains the shutdown hook and delegates idempotence through the file provider | Ready for verification |

## 4. Provider Interface Baseline

Sprint 3 must consume the Agent-Diva-owned `MemoryProvider` contract as-is:

- `system_prompt_block(&SystemPromptRequest) -> Result<SystemPromptResponse>`
- `prefetch(PrefetchRequest) -> Result<PrefetchResponse>`
- `sync_turn(SyncTurnRequest) -> Result<SyncTurnResponse>`
- `on_session_end(SessionEndRequest) -> Result<SessionEndResponse>`

Contract rules:

- `system_prompt_block()` is startup prompt assembly only. It must not perform async I/O, call `block_in_place`, or call `Handle::block_on`.
- `prefetch()` is recall-only. Recoverable Mentle failures should return `PrefetchStatus::Failed` without failing the user turn.
- `sync_turn()` is persistence-only. Markdown file writes are authoritative; Mentle secondary write failure should be logged and degraded, not treated as loss of Markdown persistence.
- `on_session_end()` is shutdown-only and must remain idempotent for duplicate session end hooks.
- Request and response types remain Agent-Diva domain structs. They must not expose MCP schemas, CLI arguments, HTTP routes, or upstream backend row types.

## 5. Mentle Package and Toolkit Baseline

The frozen dependency baseline is:

- crate: `memtle`
- source: crates.io
- version: exact `0.1.2`
- dependency shape: `default-features = false`
- default Agent-Diva Rust floor: `1.80.0`
- Mentle feature Rust floor: `1.88.0+`

Sprint 3 may rely on these published-crate surfaces:

- `memtle::toolkit::MemtleToolkit::open(path).await`
- `MemtleToolkit::tool_definitions()`
- `MemtleToolkit::call_json(name, args).await`
- typed toolkit APIs already used by the provider path, such as status, graph stats, search, and drawer/diary write behavior where available from the published crate

Sprint 3 must not rely on:

- a local sibling `memtle/` checkout
- unpublished APIs not represented by `memtle 0.1.2`
- upstream default CLI or MCP feature behavior
- a fixed global count of `memtle_*` tools

## 6. Sprint 3 Runtime and Tool Adapter Baseline

Sprint 3 runtime/tool work must follow these rules:

- Build `MentleToolkitTool` instances dynamically from `MemtleToolkit::tool_definitions()`.
- Interpret each tool definition using `name`, `description`, and `inputSchema`.
- Execute tool calls through `MemtleToolkit::call_json(name, args).await`.
- Convert toolkit call errors into Agent-Diva `ToolError::ExecutionFailed` or the closest existing tool error shape.
- Keep the toolkit behind a shared async lock when the adapter needs mutable or serialized access.
- Register anchor tools such as `memtle_status`, `memtle_search`, and `memtle_diary_write` in tests, but never assert a fixed tool count.
- Treat `MemtleToolkit::open()` failure as Mentle inactive: no Mentle prompt routing and no `memtle_*` tool exposure.

`MentleRuntime` is the Sprint 3 runtime ownership boundary. It should own or preserve:

- the opened toolkit handle
- the active `Arc<dyn MemoryProvider>`
- reusable custom `Vec<Arc<dyn Tool>>`
- the active/inactive state needed by prompt routing

Initial tool assembly and cron rebuild must reuse the same custom Mentle tool vector. `with_toolset()` must enable Mentle prompt routing only when the supplied registry contains `memtle_status`.

## 7. Required Verification

The review package is not complete until these commands are run and their results are recorded by the implementation owner:

```bash
cargo check -p agent-diva-agent --no-default-features
cargo check -p agent-diva-agent --features mentle
cargo test -p agent-diva-core --features mentle memory
cargo test -p agent-diva-agent --features mentle mentle
```

Static policy checks must also confirm:

- the workspace dependency declaration still pins `memtle 0.1.2`
- no manifest introduces `path`, `git`, or `[patch.crates-io]` overrides for `memtle`
- Sprint 2 provider docs do not require async DB I/O inside `system_prompt_block()`
- Sprint 3 tool/runtime docs do not hard-code a Mentle tool count

## 8. Handoff Notes

Sprint 3 may start from this baseline with these defaults:

- `HybridMemoryProvider` is the accepted provider bridge.
- `MemtleToolkit::tool_definitions()` is the only source of Mentle tool schemas.
- `MemtleToolkit::call_json()` is the generic adapter call path.
- `mentle_active` is runtime state, not merely config state.
- Prompt routing must follow actual runtime/tool availability.
- Cron rebuild and initial assembly must share the same custom Mentle tools.
- Subagents remain Mentle-disabled by default unless a later reviewed task changes that policy.

Residual risks carried into Sprint 3:

- the published `memtle 0.1.2` feature surface must continue to compile with `default-features = false`
- Windows Mentle verification still depends on an available native toolchain for upstream native dependencies
- tool schema evolution in future `memtle` releases must go through the dedicated upgrade policy before Agent-Diva changes its baseline
