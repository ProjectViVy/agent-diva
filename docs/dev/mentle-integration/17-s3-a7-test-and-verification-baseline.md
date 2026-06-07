# Sprint 3 A7: Test and Verification Baseline

## Purpose

S3-A7 freezes the minimum verification set for Sprint 3 Mentle adapter and
runtime work.

The goal is to prevent Sprint 4 from inventing ad hoc test entry criteria for
runtime assembly, dynamic tools, or inactive fallback behavior.

## Minimum Verification Set

Sprint 3 is not review-ready unless the following verification buckets are
covered:

1. Default build isolation
2. Dynamic tool definition and registration behavior
3. Runtime activity and inactive prompt-routing behavior
4. Custom tool reuse across startup and rebuild paths
5. Mentle feature-lane build and test evidence, or an explicit environment
   block with recorded failure cause

## Required Test Scope

### 1. Default Build Isolation

Sprint 3 must prove that the non-Mentle lane still builds without pulling the
Mentle runtime path into the default Agent-Diva build.

Required command:

```bash
cargo check -p agent-diva-agent --no-default-features
```

Acceptance:

- default lane passes
- no Sprint 3 work requires `mentle` feature enablement to compile the default
  agent path

### 2. Dynamic Tool Baseline

Sprint 3 must cover the dynamic Mentle tool adapter contract with tests that
exercise:

- schema extraction from `tool_definitions()`
- rejection of incomplete or non-object `inputSchema`
- dynamic registration without fixed tool-count assumptions
- direct execution through `call_json(name, args).await`
- transport and payload error mapping into `ToolError::ExecutionFailed`

Acceptance anchors:

- `memtle_status`
- `memtle_search`
- `memtle_diary_write`

Tests may use these as presence anchors, but must not assert a fixed total
Mentle tool count.

### 3. Runtime Active and Inactive Scenarios

Sprint 3 must distinguish:

- active runtime: prompt routing enabled only when registered tools contain
  `memtle_status`
- inactive runtime due to open failure: Markdown fallback remains active and
  prompt omits Mentle routing
- inactive prompt-routing despite successful runtime construction: valid custom
  tools may exist, but `with_mentle(false)` must be used when `memtle_status` is
  absent

Acceptance:

- no prompt/tool mismatch remains open
- inactive scenarios are explicitly tested, not inferred from happy-path tests

### 4. Rebuild and Reuse Behavior

Sprint 3 must prove that Mentle custom tools are not a startup-only artifact.

Minimum runtime/rebuild coverage:

- initial registry assembly can include custom Mentle tools
- `register_default_tools()` preserves the existing custom tool vector
- cron/default tool rebuild paths still expose the same custom Mentle tools

Acceptance:

- Sprint 4 cron work can treat custom tool reuse as a frozen assumption

## Verification Commands

The Sprint 3 minimum verification lane is:

```bash
cargo fmt
cargo fmt --all -- --check
cargo check -p agent-diva-agent --no-default-features
cargo check -p agent-diva-agent --features mentle
cargo test -p agent-diva-agent test_build_agent_tools_reuses_custom_tools_with_cron
cargo test -p agent-diva-agent test_register_default_tools_preserves_custom_tools_with_cron
cargo test -p agent-diva-agent --features mentle mentle
cargo test -p agent-diva-core --features mentle memory
```

Static policy checks:

```bash
rg -n "memtle = \\{ version = \"0.1.2\", default-features = false \\}" Cargo.toml agent-diva-agent/Cargo.toml agent-diva-core/Cargo.toml
rg -n "\\[patch.crates-io\\]|memtle.*path|memtle.*git" Cargo.toml agent-diva-agent/Cargo.toml agent-diva-core/Cargo.toml
```

## Recorded Interpretation Rules

Sprint 3 verification is considered complete only when each command is recorded
as one of:

- `passed`
- `failed` because of a repo regression
- `blocked` because of an environment prerequisite outside the repo

For the current Windows Mentle lane, the canonical environment block is:

```text
error occurred in cc-rs: failed to find tool "clang-cl.exe": program not found
```

That block does not satisfy the Mentle feature lane, but it is acceptable as
Sprint 3 review evidence if:

- the failure is recorded explicitly
- default-lane and non-Mentle rebuild tests pass
- Sprint 4 inherits the native toolchain prerequisite as an open risk

## Sprint 4 Entry Use

Sprint 4 may treat this baseline as frozen:

- dynamic tool adapter tests already define the minimum schema/execution/error
  contract
- runtime/inactive prompt-routing tests already define the activation rule
- rebuild-path tests already define the custom tool preservation requirement
- Mentle feature-lane verification still depends on a host with `clang-cl.exe`
