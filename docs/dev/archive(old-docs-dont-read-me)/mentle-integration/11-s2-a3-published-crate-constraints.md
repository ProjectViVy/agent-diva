# Sprint 2 A3: Published `memtle` Constraint Record

## 1. Purpose

This record freezes the upstream package contract that Sprint 2 and Sprint 3 implementation work must follow.

It exists to prevent Agent-Diva from designing against a local sibling `memtle/` checkout or against unpublished API assumptions. All Mentle-facing implementation work must justify itself from the published crate contract already enforced in the workspace and CI.

## 2. Published Package Contract

The current integration target is the published Cargo package:

- crate: `memtle`
- source: `crates.io`
- version: exact `0.1.2`
- dependency shape in Agent-Diva: `default-features = false`

The upstream package facts currently relevant to Agent-Diva are:

- crate edition: `2024`
- `rust-version`: `1.88`
- default features: `["cli", "mcp"]`
- binary target `memtle` requires feature `cli`

Agent-Diva must treat those facts as the boundary of the supported integration lane. The main workspace must not switch to a path dependency, git dependency, or `[patch.crates-io]` override for `memtle`.

## 3. Downstream Design Constraints

### 3.1 Feature-Surface Constraint

Agent-Diva embeds `memtle` with `default-features = false`. That means Sprint 2 and Sprint 3 work must assume only the toolkit and tool-schema surfaces available from the published library build without the upstream default CLI/MCP feature bundle.

Implementation must not rely on:

- CLI-only behavior
- assumptions that `mcp` is enabled
- unpublished local checkout changes not represented by `memtle 0.1.2`

### 3.2 Toolchain Constraint

The default Agent-Diva workspace baseline remains Rust `1.80.0`. The Mentle feature lane is the only Rust `1.88+` lane.

This means:

- default builds must remain Mentle-free
- `--features mentle` is the only path allowed to pull the `memtle` toolchain floor into the build
- Sprint 2 and Sprint 3 docs must not describe the Mentle path as available on the default workspace MSRV

### 3.3 Runtime Constraint

Prompt assembly must not depend on async Mentle I/O.

The provider contract and current implementation shape both require:

- `system_prompt_block()` remains synchronous
- `system_prompt_block()` must use cached status/snapshot state only
- `prefetch()` and `sync_turn()` are the allowed async Mentle interaction seams
- Mentle prompt and tools remain runtime-gated by successful toolkit activation, not by Cargo feature enablement alone

### 3.4 Dynamic Tool Constraint

Sprint 3 tool exposure must continue to derive from the published package's `tool_definitions()` output instead of relying on local hard-coded tool counts or local checkout-only metadata.

Smoke tests may assert anchor tools such as:

- `memtle_status`
- `memtle_search`
- `memtle_diary_write`

But the implementation must not encode a fixed global tool count.

## 4. Current Repo Alignment

As of `2026-05-24`, the repository already reflects part of this contract:

- workspace `Cargo.toml` pins `memtle = { version = "0.1.2", default-features = false }`
- `agent-diva-core` and `agent-diva-agent` both gate Mentle through an explicit `mentle` feature
- CI validates the published-package policy and runs a dedicated Rust `1.88.0` Mentle job
- provisional Mentle implementation already exists for:
  - `HybridMemoryProvider`
  - `MentleRuntime`
  - `MentleToolkitTool`
  - `with_toolset()` Mentle prompt/tool safety tests

Sprint 2 planning and implementation notes must therefore be written as contract-alignment work over an existing provisional code path, not as a greenfield feature kickoff.

## 5. Required Verification Set

The canonical verification commands for this contract are:

```bash
cargo check -p agent-diva-agent --no-default-features
cargo check -p agent-diva-agent --features mentle
cargo test -p agent-diva-core --features mentle memory
cargo test -p agent-diva-agent --features mentle mentle
```

Document review is incomplete unless all of the following stay true:

- the workspace dependency declaration still matches the published package contract
- no manifest introduces `path`, `git`, or `[patch.crates-io]` overrides for `memtle`
- Sprint 2 provider planning references only published-crate-confirmed surfaces
- Sprint 3 tool/runtime planning continues to assume dynamic tool registration

## 6. Effect on Sprint 2 and Sprint 3 Cards

This record is the dependency freeze point for:

- `K-05` Plan `HybridMemoryProvider` skeleton
- `K-06` Plan cached Palace snapshot
- `K-07` Plan async Mentle prefetch
- `K-08` Plan `sync_turn()` Mentle write path
- `K-09` Plan `MentleToolkitTool` adapter
- `K-10` Plan dynamic tool registration
- `K-11` Plan `MentleRuntime` helper

Those cards must not introduce new dependency-source assumptions beyond what is recorded here without a separate package upgrade review.
