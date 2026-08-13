# Sprint 6 A3: Downgrade and Failure Acceptance

## Purpose

This record replays the Sprint 5 failure-mode evidence against the Sprint 6
release-candidate boundary. It verifies that the RC still degrades safely when
Mentle is unavailable, when recall or secondary writes fail, and when tool
registration cannot support the prompt capability that would otherwise be
advertised.

Sprint 6 is RC and handoff work. This acceptance does not add new Mentle
features, GUI controls, per-tool selection, runtime hot-refresh, or dependency
source changes.

## Baseline Consumed

- [26-s5-a1-failure-validation-matrix.md](./26-s5-a1-failure-validation-matrix.md)
- [27-s5-a2-a6-failure-and-ci-hardening.md](./27-s5-a2-a6-failure-and-ci-hardening.md)
- [28-s5-a7-sprint5-review-package.md](./28-s5-a7-sprint5-review-package.md)
- [29-s6-a1-rc-scope-baseline.md](./29-s6-a1-rc-scope-baseline.md)

The S6-A1 RC baseline keeps the relevant contract frozen:

- `MemtleToolkit::open()` failure must disable Mentle and preserve Markdown
  memory continuity.
- Prompt activation must be registry-derived and anchored on `memtle_status`.
- Recall failure must not block the primary LLM call.
- `memtle_diary_write` failure must not override `HISTORY.md` authority.
- `system_prompt_block()` must not perform fresh async SQLite or runtime
  blocking work in the prompt path.
- `with_toolset()` must remain external-registry driven and must not synthesize
  a hidden Mentle runtime or hybrid provider.

## Code Evidence Reviewed

| Area | Evidence | RC interpretation |
|---|---|---|
| Startup/open downgrade | `agent-diva-agent/src/mentle_runtime.rs` maps memory-dir creation failure and `MemtleToolkit::open(&db_path).await` failure to `None` with `fallback_action = "disable_mentle"` | Runtime construction fails closed instead of producing partial Mentle capability |
| AgentLoop fallback | `agent-diva-agent/src/agent_loop.rs` uses `MemoryManager` when Mentle runtime is unavailable and sets `with_mentle(mentle_active)` from runtime/toolset state | Main loop remains Markdown-backed when Mentle is inactive |
| Runtime inactive semantics | `MentleRuntime::from_parts()` sets `active` only when custom tools include `memtle_status` | A runtime without the status tool cannot advertise L2 Palace prompt routing |
| Tool filtering and prompt consistency | `with_toolset()` derives `mentle_active` from `toolset.registry.has("memtle_status")` and tests cover missing status, status present, and other `memtle_*` tools without status | External registries do not create prompt/tool mismatch |
| Query failure downgrade | `HybridMemoryProvider::prefetch()` returns `PrefetchStatus::Failed { reason }` with no prompt block on toolkit search error | Live turn can continue without stale recall injection |
| Write failure downgrade | `HybridMemoryProvider::sync_turn()` writes Markdown history first, logs failed `memtle_diary_write`, and returns persisted when file-backed history succeeds | `HISTORY.md` remains authoritative; secondary Palace write failure is degraded persistence |
| Prompt path non-blocking | `MemoryProvider` contract requires `system_prompt_block()` to use synchronous local state only; `HybridMemoryProvider::system_prompt_block()` combines `MemoryManager` output with the cached Palace snapshot | Prompt assembly does not open/query SQLite through Mentle at render time |

## Acceptance Matrix

| Check | Result | Evidence |
|---|---|---|
| `MemtleToolkit::open()` failure degrades smoothly | Accepted | `test_mentle_open_failure_falls_back_to_markdown_memory` passed; prompt retained Markdown fallback text and omitted `memtle_status`, `memtle_search`, and L2 Palace text |
| Memory directory/open precondition failure disables runtime | Accepted | `test_mentle_memory_dir_create_failure_disables_runtime` passed; `MentleRuntime::try_build(...)` returned `None` |
| Query failure does not break main flow | Accepted | `test_agent_loop_prefetch_failure_continues_without_recall_injection` passed; provider returned `"done"` and no `## Prefetch Recall` block was injected |
| Write failure does not break main flow | Accepted | `test_agent_loop_consolidation_sync_failure_keeps_main_response` passed; core `hybrid_sync_turn_keeps_snapshot_when_diary_write_returns_tool_failure` also passed |
| Prompt does not advertise nonexistent tools | Accepted | `mentle` and `with_toolset` test filters passed, including runtime without `memtle_status` and registry containing `memtle_search` without status |
| Runtime inactive state remains semantically aligned | Accepted | `test_mentle_runtime_active_matches_registered_status_tool` and `test_mentle_runtime_without_status_disables_prompt_routing` passed |
| Tool filtering or missing tools do not create prompt/tool mismatch | Accepted | `test_with_toolset_missing_memtle_tool_disables_prompt`, `test_with_toolset_memtle_tool_without_status_disables_prompt`, and `test_with_toolset_keeps_external_registry_isolated_from_runtime_state` passed |
| `system_prompt_block()` does not block on DB | Accepted by contract and implementation review | The trait contract forbids async/runtime blocking work; hybrid implementation reads `MemoryManager` plus cached snapshot only. No new runtime DB access was found in the prompt render path |
| Package source policy remains RC-compliant | Accepted | `python scripts/ci/verify_mentle_package_policy.py` passed: crates.io `memtle 0.1.2` |

## Known Fallback Behaviors

- If Mentle is requested but runtime construction fails, Agent-Diva logs the
  downgrade and continues with Markdown memory through `MemoryManager`.
- If the runtime opens but registered dynamic tools do not include
  `memtle_status`, Mentle prompt routing remains inactive even if other
  `memtle_*` tools exist.
- If `with_toolset()` receives an external registry without `memtle_status`, the
  prompt remains Markdown-only. If the registry has `memtle_status`, prompt
  activation is derived from that registry, but no runtime-owned custom tools or
  hybrid memory provider are synthesized.
- If Palace recall/search fails during prefetch, the turn continues without a
  recall block.
- If `memtle_diary_write` fails after `HISTORY.md` is written, file-backed
  history remains authoritative and the cached Palace snapshot is preserved.
- If `MEMORY.md` or `HISTORY.md` file writes fail, sync returns a failed status
  instead of claiming full persistence.

## Validation Commands

Environment prerequisite notes:

- `where.exe tmux` failed on this Windows host: tmux is not available in PATH.
  No tmux session was created.
- `where.exe clang-cl` failed on this shell: `clang-cl.exe` is not available in
  PATH. The focused Mentle feature tests below still compiled and passed in this
  environment.

Commands run from `agent-diva/`:

```powershell
cargo test -p agent-diva-agent --features mentle test_mentle_open_failure_falls_back_to_markdown_memory
```

Result: passed, 1 test passed.

```powershell
cargo test -p agent-diva-agent --features mentle test_mentle_memory_dir_create_failure_disables_runtime
```

Result: passed, 1 test passed.

```powershell
cargo test -p agent-diva-agent --features mentle test_with_toolset_missing_memtle_tool_disables_prompt
```

Result: passed, 1 test passed.

```powershell
cargo test -p agent-diva-agent --features mentle mentle; cargo test -p agent-diva-agent --features mentle test_agent_loop_prefetch_failure_continues_without_recall_injection; cargo test -p agent-diva-agent --features mentle test_agent_loop_consolidation_sync_failure_keeps_main_response; cargo test -p agent-diva-core --features mentle hybrid_
```

Result: passed.

- Agent `mentle` filter: 20 passed, 0 failed.
- Agent prefetch failure test: 1 passed, 0 failed.
- Agent consolidation sync failure test: 1 passed, 0 failed.
- Core `hybrid_` filter: 12 passed, 0 failed.

```powershell
cargo test -p agent-diva-agent --features mentle with_toolset; python scripts/ci/verify_mentle_package_policy.py
```

Result: passed.

- Agent `with_toolset` filter: 4 passed, 0 failed.
- Package policy: `Mentle package policy verified: crates.io memtle 0.1.2`.

An initial attempt to chain the same checks with `&&` failed because the current
PowerShell parser did not accept `&&` as a statement separator. The command was
rerun with semicolon separators and passed.

## Blocker and Non-Blocker Classification

### Blockers

None found in this S6-A3 downgrade/failure acceptance pass.

### Non-Blockers and Accepted Exceptions

- `tmux` is unavailable on this Windows host, so the QA prerequisite for tmux
  sessions was not satisfied. This is non-blocking for this task because the
  requested acceptance surface is covered by deterministic cargo tests and
  source review rather than a long-running interactive service.
- `clang-cl.exe` is not available in this shell PATH. This remains the known
  Windows Mentle feature-lane environment note from S5/S6, but the focused
  Mentle feature tests used for this acceptance still passed.
- The workspace root `d:\newdev\new-mentle` is not itself a Git repository.
  `agent-diva/` is the relevant project repository for status and validation.
- Full workspace `just test` was not promoted or rerun as an S6-A3 gate. S6-A1
  keeps the pre-existing provider export issue outside the Mentle RC blocker
  set unless Sprint 6 owners explicitly promote full workspace tests.

## Conclusion

S6-A3 accepts the downgrade and failure behavior for the RC scope. The current
code and focused tests preserve the Sprint 5 conclusion: Mentle failures degrade
to Markdown-backed continuity, recall and secondary write failures do not break
the main flow, and prompt routing does not advertise Mentle capability unless
`memtle_status` is present in the assembled registry.

No code change or blocker fix is recommended from this pass. The remaining items
are environment/documented-scope non-blockers and should stay visible in the
Sprint 6 handoff and risk register.
