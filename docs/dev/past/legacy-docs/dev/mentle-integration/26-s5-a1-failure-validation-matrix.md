# Sprint 5 A1: Failure Validation Matrix

## Purpose

Sprint 5 hardens the Mentle x Agent-Diva integration against failure modes that
can otherwise create prompt/tool mismatch, broken dialogue flow, or unverified
CI coverage.

This Sprint does not add new Mentle capabilities. It validates and hardens the
Sprint 2 through Sprint 4 baseline so the Sprint 6 release-candidate package can
rely on documented downgrade behavior.

## Scope

### In Scope

- Mentle startup/open failure.
- Palace recall/search failure.
- `sync_turn()` and `memtle_diary_write` write failure.
- Invalid dynamic tool definitions.
- Runtime active/inactive state consistency.
- Cron/default registry rebuild.
- `with_toolset()` external registry construction.
- Subagent default isolation.
- Default and Mentle feature CI coverage.
- Published `memtle 0.1.2` package source validation.

### Out of Scope

- Sprint 7 tool-selection modes such as `read_only`, `full`, or `custom`.
- GUI controls for Mentle settings.
- Runtime hot-refresh of Mentle tool definitions.
- Replacing the `memtle_status` prompt-routing activation anchor.
- Switching Mentle to a local path, git dependency, or workspace sibling.

## Validation Matrix

| ID | Area | Failure Mode | Expected Behavior | Evidence Target |
|---|---|---|---|---|
| S5-F01 | Startup | `MemtleToolkit::open()` cannot build a runtime | AgentLoop starts with Markdown memory, no `memtle_*` tools, no L2 Palace prompt text | Mentle feature test in `agent_loop.rs` |
| S5-F02 | Startup | `memory/palace.db` parent directory cannot be created | Runtime construction returns inactive/unavailable state and falls back to Markdown memory | Mentle feature test around `MentleRuntime::try_build(...)` |
| S5-F03 | Prompt | Runtime exists but dynamic tools omit `memtle_status` | Registry may contain other `memtle_*` tools, but prompt routing remains inactive | Existing and hardened Mentle feature tests |
| S5-F04 | Tool definition | A toolkit definition is missing `name`, `description`, or object `inputSchema` | Invalid definition is skipped individually; valid definitions still register | Dynamic tool metadata tests |
| S5-F05 | Tool call | `call_json(...)` returns a transport error | Tool invocation returns `ToolError::ExecutionFailed`; main runtime remains usable | Mentle tool adapter tests |
| S5-F06 | Tool payload | `call_json(...)` returns `success=false` or an `error` payload | Tool invocation returns `ToolError::ExecutionFailed` with a useful message | Mentle tool adapter tests |
| S5-F07 | Recall | Palace search fails during prefetch | Main LLM call continues without a recall block; failure is represented as `PrefetchStatus::Failed` | Core hybrid memory tests and AgentLoop prefetch test |
| S5-F08 | Write | `memtle_diary_write` fails during `sync_turn()` | `HISTORY.md` remains authoritative; sync returns persisted if file writes succeeded | Core hybrid memory tests |
| S5-F09 | File write | `MEMORY.md` or `HISTORY.md` write fails | Sync reports a failed status for the file failure and does not claim full persistence | Existing core hybrid memory tests |
| S5-F10 | Cron rebuild | Startup or later default rebuild happens after Mentle custom tools are present | Registry preserves `memtle_*` custom tools and cron tool together | Default and Mentle feature AgentLoop tests |
| S5-F11 | `with_toolset()` | External registry has Mentle enabled in config but lacks `memtle_status` | Prompt routing remains inactive; no runtime-owned custom tools are synthesized | Default-lane AgentLoop tests |
| S5-F12 | `with_toolset()` | External registry contains `memtle_status` | Prompt routing becomes active from the registry only; Mentle runtime remains absent | Default-lane AgentLoop tests |
| S5-F13 | Subagent | Parent tool config or registry has Mentle enabled | Subagent config, registry, and prompt remain Mentle-disabled by default | Default-lane subagent tests |
| S5-F14 | Default build | Default CI path accidentally pulls Mentle or skips assembly regressions | Default build/test lane passes without `mentle` feature and covers S4/S5 regressions | CI `rust-check` and local just recipe |
| S5-F15 | Mentle build | Mentle feature path misses native/toolchain or integration regressions | Mentle feature check and targeted memory/agent tests pass on Rust 1.88+ | CI `mentle-check` and local just recipe |
| S5-F16 | Package source | Workspace reintroduces `path`, `git`, or `[patch.crates-io]` for `memtle` | CI fails before tests; lockfile must resolve crates.io `memtle 0.1.2` | CI package policy check |

## Execution Order

S5-A2 through S5-A6 consume this matrix:

1. S5-A2 validates startup/open downgrade and prompt visibility.
2. S5-A3 validates recall and write failure semantics.
3. S5-A4 validates dynamic runtime/tool assembly consistency.
4. S5-A5 validates cron, `with_toolset()`, and subagent advanced entrypoints.
5. S5-A6 converts the default and Mentle validation lanes into CI-local parity.
6. S5-A7 records final results and residual risk against this matrix.

## Acceptance

S5-A1 is accepted when every Sprint 5 activity can cite one or more matrix rows
as its verification target, and no Sprint 5 implementation item depends on
unplanned Sprint 7 GUI or tool-selection behavior.
