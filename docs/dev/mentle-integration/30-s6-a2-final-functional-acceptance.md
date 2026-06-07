# Sprint 6 A2: Final Functional Acceptance

## Purpose

This record completes the Sprint 6 A-QA final functional acceptance replay for
the Mentle x Agent-Diva release-candidate boundary.

Sprint 6 remains an RC and handoff sprint. This report does not add runtime
features, GUI controls, dependency changes, broad refactors, commits, or source
fixes. It consumes the frozen S6-A1 scope and records fresh, focused validation
evidence for the final functional acceptance surface.

## Instructions and Baselines Used

Project instructions reviewed:

- Workspace `AGENTS.md`
- `agent-diva/AGENTS.md`
- `memtle/AGENTS.md`
- `memtle/STYLEGUIDE.md`

Sprint and design baselines reviewed:

- [09-project-management.md](./09-project-management.md)
- [12-s2-a8-sprint2-review-and-s3-interface-baseline.md](./12-s2-a8-sprint2-review-and-s3-interface-baseline.md)
- [13-s3-a1-memtle-toolkit-tool-interface.md](./13-s3-a1-memtle-toolkit-tool-interface.md)
- [14-s3-a2-dynamic-tool-registration-model.md](./14-s3-a2-dynamic-tool-registration-model.md)
- [17-s3-a7-test-and-verification-baseline.md](./17-s3-a7-test-and-verification-baseline.md)
- [18-s3-a8-sprint3-review-package.md](./18-s3-a8-sprint3-review-package.md)
- [24-s4-a12-sprint4-review-package.md](./24-s4-a12-sprint4-review-package.md)
- [26-s5-a1-failure-validation-matrix.md](./26-s5-a1-failure-validation-matrix.md)
- [27-s5-a2-a6-failure-and-ci-hardening.md](./27-s5-a2-a6-failure-and-ci-hardening.md)
- [28-s5-a7-sprint5-review-package.md](./28-s5-a7-sprint5-review-package.md)
- [29-s6-a1-rc-scope-baseline.md](./29-s6-a1-rc-scope-baseline.md)

Related Sprint 6 files were read for consistency and were not modified:

- [31-s6-a3-downgrade-failure-acceptance.md](./31-s6-a3-downgrade-failure-acceptance.md)
- [32-s6-a4-advanced-entrypoint-acceptance.md](./32-s6-a4-advanced-entrypoint-acceptance.md)
- [33-s6-a5-ci-build-dependency-validation.md](./33-s6-a5-ci-build-dependency-validation.md)
- [34-s6-a6-final-technical-handoff-docs.md](./34-s6-a6-final-technical-handoff-docs.md)

## Environment

Commands were run from `agent-diva/` on Windows PowerShell.

| Item | Observed result |
|---|---|
| `cargo --version` | `cargo 1.93.0 (083ac5135 2025-12-15)` |
| `rustc --version` | `rustc 1.93.0 (254b59607 2026-01-19)` |
| `just --version` | `just 1.46.0` |
| `python --version` | `Python 3.13.2` |
| `where.exe tmux` | Not found in PATH |
| `where.exe clang-cl` | Not found in PATH |
| `Test-Path "C:\Program Files\LLVM\bin\clang-cl.exe"` | `True` |

The Mentle feature lane used the documented Windows prerequisite prefix:

```powershell
$env:PATH = 'C:\Program Files\LLVM\bin;' + $env:PATH
```

The workspace root `d:\newdev\new-mentle` is not itself a Git repository.
`agent-diva/` is the relevant project repository. Its working tree already
contained many modified/untracked files before this report was created,
including S6-A3 through S6-A6 records. This task only adds the S6-A2 report.

## Code Evidence Reviewed

Search and source review covered the requested keywords: `mentle`, `memtle`,
`MemtleToolkit`, `MEMORY.md`, `system_prompt_block`, `dynamic tool`, and `tools`.

| Acceptance area | Code evidence | Interpretation |
|---|---|---|
| Package source and feature isolation | `Cargo.toml` pins `memtle = { version = "0.1.2", default-features = false }`; `agent-diva-agent` and `agent-diva-core` expose optional `mentle` features | Default builds stay Mentle-free; Mentle is explicit |
| Runtime assembly | `agent-diva-agent/src/mentle_runtime.rs` builds `MentleRuntime` from `MemtleToolkit::open(...)`, `HybridMemoryProvider`, and dynamic custom tools | Runtime owns toolkit, provider, reusable tools, and active state |
| Dynamic tool registration | `mentle_tools_from_definitions(...)` consumes definitions from `tool_definitions()` and maps `name`, `description`, and object `inputSchema` only | No fixed tool-count or copied tool manifest is required |
| Tool execution | `MentleToolkitTool::execute(...)` forwards JSON arguments through `MemtleToolkit::call_json(name, args).await` | Generic adapter execution path remains the published toolkit call path |
| Active prompt anchor | `MentleRuntime::from_parts(...)` sets `active` only when custom tools contain `memtle_status`; `AgentLoop::with_toolset(...)` derives active state from `toolset.registry.has("memtle_status")` | Prompt routing is registry/tool availability driven |
| Dual-track memory | `HybridMemoryProvider::system_prompt_block(...)` combines `MemoryManager` output with cached Palace snapshot; `prefetch(...)` searches Palace; `sync_turn(...)` writes Markdown history and then `memtle_diary_write` | Markdown L0/L1 stays authoritative; Mentle is L2 Palace memory |
| Main session behavior | AgentLoop tests cover prefetch failure continuation and consolidation sync-failure continuation | Main response path continues when recall or secondary persistence fails |
| Advanced entrypoints | `build_agent_tools(...)`, `register_default_tools(...)`, `with_toolset(...)`, subagent config, subagent registry, and subagent prompt tests exist | Cron/default rebuild, external registry, and subagent isolation remain covered |

## Fresh Validation Commands

### Package Source Policy

```powershell
python scripts/ci/verify_mentle_package_policy.py
```

Result: passed.

Observed output:

```text
Mentle package policy verified: crates.io memtle 0.1.2
```

Lockfile evidence:

```text
name = "memtle"
version = "0.1.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
```

Static search result:

- No `memtle` `path` dependency found in `Cargo.toml` files.
- No `memtle` `git` dependency found in `Cargo.toml` files.
- No `[patch.crates-io]` override for `memtle` found in `Cargo.toml` files.

### Default Lane

```powershell
just sprint5-default-check
```

Result: passed.

Observed sub-results:

- `cargo check -p agent-diva-agent --no-default-features`: passed
- `cargo test -p agent-diva-agent test_with_toolset`: passed, 4 tests
- `cargo test -p agent-diva-agent test_tool_assembly_subagent_mode_excludes_mentle_custom_tools`: passed, 1 test
- `cargo test -p agent-diva-agent subagent_does_not_receive_mentle_by_default`: passed, 1 test
- `cargo test -p agent-diva-agent test_build_agent_tools_reuses_custom_tools_with_cron`: passed, 1 test
- `cargo test -p agent-diva-agent test_register_default_tools_preserves_custom_tools_with_cron`: passed, 1 test
- `cargo test -p agent-diva-agent test_build_subagent_prompt_omits_mentle_routing`: passed, 1 test
- `cargo test -p agent-diva-agent test_agent_loop_prefetch_failure_continues_without_recall_injection`: passed, 1 test
- `cargo test -p agent-diva-agent test_agent_loop_consolidation_sync_failure_keeps_main_response`: passed, 1 test

### Mentle Feature Lane

```powershell
$env:PATH = 'C:\Program Files\LLVM\bin;' + $env:PATH
just mentle-check
```

Result: passed.

Observed sub-results:

- `python scripts/ci/verify_mentle_package_policy.py`: passed
- `cargo check -p agent-diva-agent --features mentle`: passed
- `cargo test -p agent-diva-core --features mentle memory`: passed, 39 tests
- `cargo test -p agent-diva-agent --features mentle mentle`: passed, 20 tests

### Focused Active and Dynamic Tool Checks

```powershell
$env:PATH = 'C:\Program Files\LLVM\bin;' + $env:PATH
cargo test -p agent-diva-agent --features mentle test_tool_definitions_are_dynamic
```

Result: passed, 1 test.

This verifies the runtime reads tool definitions dynamically and anchors on
actual `memtle_status`, `memtle_search`, and `memtle_diary_write` definitions
without asserting a fixed total tool count.

```powershell
$env:PATH = 'C:\Program Files\LLVM\bin;' + $env:PATH
cargo test -p agent-diva-agent --features mentle test_with_tools_active_runtime_enables_registry_and_prompt
```

Result: passed, 1 test.

This verifies that an active Mentle runtime exposes `memtle_status` in the
registry and enables L2 Palace Memory prompt routing.

## Main Scenario Verification Checklist

| Scenario | Expected RC behavior | Fresh result | Evidence |
|---|---|---|---|
| Default build path | Agent-Diva builds without enabling Mentle | Passed | `just sprint5-default-check` passed `cargo check -p agent-diva-agent --no-default-features` |
| Mentle feature build path | Mentle compiles when feature is explicit and Windows LLVM PATH prerequisite is applied | Passed | `just mentle-check` passed after adding `C:\Program Files\LLVM\bin` to PATH |
| Startup with active Mentle | Active runtime exposes `memtle_status` and prompt includes L2 Palace routing | Passed | `test_with_tools_active_runtime_enables_registry_and_prompt` passed |
| `MEMORY.md` plus Mentle dual-track memory | Markdown startup memory remains authoritative while Palace snapshot, recall, and diary write paths work | Passed | `cargo test -p agent-diva-core --features mentle memory` passed 39 tests |
| `memtle_*` tools callable | Adapter executes a real toolkit JSON call through `call_json` and maps dynamic schema to Agent-Diva tools | Passed | `cargo test -p agent-diva-agent --features mentle mentle` passed `test_mentle_tool_adapter_executes_json_call` |
| Dynamic tool registration | Tool list comes from `tool_definitions()` and includes anchor tools without fixed-count assumptions | Passed | `test_tool_definitions_are_dynamic` passed |
| Prompt activation correctness | Prompt only advertises Mentle when registry/runtime contains `memtle_status` | Passed | Mentle and `with_toolset` tests passed status-present, status-missing, and non-status `memtle_*` cases |
| Main session memory behavior | Recall failure and secondary sync failure do not break the primary response path | Passed | `just sprint5-default-check` passed prefetch failure and consolidation sync-failure tests |
| Cron/default rebuild | Rebuild paths preserve custom Mentle tools | Passed | `just sprint5-default-check` passed both cron/default custom-tool preservation tests |
| Subagent isolation | Subagents do not inherit Mentle tools or prompt routing by default | Passed | `just sprint5-default-check` passed config, registry, and prompt isolation tests |
| Package source policy | Agent-Diva consumes crates.io `memtle 0.1.2`, not local `memtle/` | Passed | Policy script passed; lockfile resolves registry source |

## Pass, Fail, and Not-Executed Summary

Passed:

- Package policy check.
- Default-lane functional acceptance checks.
- Mentle feature-lane build and focused tests.
- Active Mentle runtime and prompt-routing check.
- Dynamic tool definition check.
- Dual-track memory tests.
- `memtle_*` adapter execution and error-mapping tests.
- Main session prefetch/sync failure-continuation checks.
- Cron/default rebuild and subagent isolation checks.

Failed:

- None in this S6-A2 pass.

Not executed:

- Full workspace `just test` was not rerun. S6-A1 and Sprint 5 classify the
  existing provider export issue around `agent_diva_providers::ollama` as outside
  the Mentle RC blocker set unless Sprint 6 owners explicitly promote full
  workspace tests to a blocker.
- GUI smoke tests were not run. GUI controls and Sprint 7 tool-selection modes
  are outside the S6 RC scope.
- Local `memtle/` project tests were not run as the integration acceptance source
  of truth. The frozen Agent-Diva policy requires the published crates.io
  `memtle 0.1.2` package, and the acceptance commands validate Agent-Diva against
  that package.
- tmux-based interactive sessions were not used because `tmux` is unavailable on
  this Windows host and no long-running interactive service was part of the S6-A2
  acceptance surface.

## Blocker and Non-Blocker Classification

### Blockers

None found.

The fresh focused command suite passed all S6-A2 acceptance surfaces required by
the S6-A1 RC boundary.

### Non-Blockers and Accepted Exceptions

- `clang-cl.exe` is not on this shell's PATH by default. This is a known Windows
  environment prerequisite, not an RC blocker on this host, because LLVM exists at
  `C:\Program Files\LLVM\bin` and the Mentle feature lane passed after applying
  the documented PATH prefix.
- `tmux` is not installed or not discoverable in PATH. This is non-blocking for
  this acceptance pass because the validated behavior is covered by deterministic
  cargo/just commands rather than a long-running interactive service.
- Full workspace `just test` remains an accepted non-blocker unless Sprint 6
  explicitly promotes it to the RC gate.
- `with_toolset()` still does not install a `HybridMemoryProvider`; this is an
  intentional external-registry boundary from S6-A1, not a bug.
- Runtime dynamic-tool hot refresh remains out of RC scope.
- Sprint 7 GUI/tool-selection work remains post-RC and must not be claimed as
  part of this acceptance.
- Documentation follow-up, not fixed here: `34-s6-a6-final-technical-handoff-docs.md`
  shows `cargo run --manifest-path scripts/ci/verify_mentle_package_policy.py`
  for the Python policy script. The command used and verified in this pass is
  `python scripts/ci/verify_mentle_package_policy.py`.

## Acceptance Conclusion

S6-A2 final functional acceptance is accepted for the frozen Sprint 1 through
Sprint 6 RC scope.

The default Agent-Diva path remains Mentle-free. The explicit Mentle feature lane
builds and passes targeted memory, runtime, dynamic tool, and adapter checks on
this Windows host after applying the documented LLVM PATH prerequisite.
`MEMORY.md` plus Mentle dual-track memory, dynamic `memtle_*` tool registration,
actual toolkit-backed tool invocation, registry-derived prompt activation, and
main session memory failure behavior all match the S6-A1 acceptance boundary.

No RC blocker was identified, and no source-code change is recommended from this
functional acceptance pass.
