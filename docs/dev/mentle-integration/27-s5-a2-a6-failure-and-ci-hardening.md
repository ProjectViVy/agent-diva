# Sprint 5 A2-A6: Failure and CI Hardening

## Purpose

This record captures the Sprint 5 implementation work that turns the failure
validation matrix into executable checks and CI coverage.

The work is intentionally regression-focused. It does not add new Mentle runtime
features, GUI settings, or tool-selection modes.

## Failure-Mode Coverage

| Matrix ID | Coverage Added or Confirmed | Evidence |
|---|---|---|
| S5-F01 | Mentle startup/open unavailable path keeps AgentLoop on Markdown memory with no L2 prompt or `memtle_status` tool | `test_mentle_open_failure_falls_back_to_markdown_memory` |
| S5-F02 | `memory` path collision prevents `memory/palace.db` parent creation and disables runtime construction | `test_mentle_memory_dir_create_failure_disables_runtime` |
| S5-F03 | Runtime without `memtle_status` keeps prompt routing inactive even if other `memtle_*` tools exist | `test_mentle_runtime_without_status_disables_prompt_routing` |
| S5-F04 | Invalid dynamic definitions are skipped by name rather than by expected count | `test_mentle_tools_skip_invalid_definitions`, `test_mentle_tool_registration_skips_invalid_definition_only` |
| S5-F05 | Toolkit transport call errors become tool execution failures | `test_mentle_tool_adapter_translates_call_errors` |
| S5-F06 | Error payloads and unknown tool payloads become tool execution failures | `test_mentle_tool_adapter_translates_payload_error`, `test_mentle_tool_adapter_translates_unknown_tool_payload` |
| S5-F07 | Prefetch failure is non-fatal and does not inject a stale recall block | `test_agent_loop_prefetch_failure_continues_without_recall_injection`, `hybrid_prefetch_invalid_room_is_recoverable_failure` |
| S5-F08 | `memtle_diary_write` failure leaves `HISTORY.md` authoritative and keeps sync persisted when file writes succeed | `hybrid_sync_turn_keeps_snapshot_when_diary_write_returns_tool_failure` |
| S5-F09 | File write failures surface as failed sync statuses | `hybrid_sync_turn_surfaces_memory_file_failure_as_status`, `hybrid_sync_turn_surfaces_history_file_failure_as_status` |
| S5-F10 | Cron/default rebuild preserves Mentle custom tools and active prompt state | `test_with_tools_startup_cron_preserves_mentle_custom_tools`, `test_register_default_tools_rebuild_keeps_active_mentle_prompt` |
| S5-F11 | `with_toolset()` without `memtle_status` keeps prompt routing inactive | `test_with_toolset_missing_memtle_tool_disables_prompt`, `test_with_toolset_memtle_tool_without_status_disables_prompt` |
| S5-F12 | `with_toolset()` with `memtle_status` activates prompt from registry contents only | `test_with_toolset_memtle_status_enables_prompt`, `test_with_toolset_keeps_external_registry_isolated_from_runtime_state` |
| S5-F13 | Subagent config, registry, and prompt remain isolated from parent Mentle tools | `subagent_does_not_receive_mentle_by_default`, `test_tool_assembly_subagent_mode_excludes_mentle_custom_tools`, `test_build_subagent_prompt_omits_mentle_routing` |
| S5-F14 | Default-lane regressions are callable locally and in CI | `just sprint5-default-check`, CI `Run Sprint 5 default-lane regressions` |
| S5-F15 | Mentle feature lane remains explicit and callable locally | `just mentle-check`, CI `mentle-check` |
| S5-F16 | Published package source policy is shared by local and CI checks | `scripts/ci/verify_mentle_package_policy.py` |

## CI and Local Command Changes

Sprint 5 adds these local commands to `justfile`:

- `just mentle-package-policy`
- `just sprint5-default-check`
- `just mentle-check`
- `just sprint5-check`

The GitHub Actions workflow now:

- triggers on `docs/dev/mentle-integration/**` changes
- runs `just sprint5-default-check` in the default Rust job
- uses `scripts/ci/verify_mentle_package_policy.py` for the Mentle package source gate
- keeps the Mentle feature lane on Rust `1.88.0` with native toolchain setup

## Behavioral Result

Sprint 5 keeps the Sprint 4 architecture intact:

- failed Mentle startup disables Mentle instead of partially advertising it
- failed recall does not block the main LLM call
- failed diary writes do not override file-backed history authority
- dynamic tool assembly stays registry-driven
- advanced entrypoints preserve prompt/tool consistency
- CI now checks the default assembly regressions continuously instead of relying
  only on a documentation baseline

## Residual Notes

- `with_toolset()` remains intentionally registry-only and does not construct a
  `HybridMemoryProvider`.
- Runtime hot-refresh of Mentle tool definitions remains out of scope and is
  still a construction-time snapshot.
- Sprint 7 GUI/tool-selection behavior remains gated behind Sprint 6 RC
  acceptance.
