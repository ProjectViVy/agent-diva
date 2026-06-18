# TODOLIST

This file is the project-level backlog for bugs, gaps, and unfinished work found during development or review.

## Open

- [ ] Replace Notebook monthly placeholder generation with full report-system monthly synthesis.
  - Context: During Story 4.1 on 2026-06-15, Notebook monthly trigger was closed by adding a Report-owned generation path that writes `{workspace}/reports/monthly/{YYYY-MM}.md` with v1 frontmatter and placeholder body sections. This unblocks the ownership/trigger contract, but it is not yet a true monthly synthesis pipeline with session aggregation, LLM summarization, retries, or cron scheduling.
  - Expected behavior: Monthly trigger and scheduled generation should produce a substantive month summary that follows the Report System PRD schema, records real `session_count`/`token_used`, and handles failure markers per PRD FR-3.
  - Related files/docs: `agent-diva-gui/src-tauri/src/commands.rs`, `agent-diva-gui/src-tauri/src/notebook.rs`, `docs/prd-report-system/prd.md`, `_bmad-output/implementation-artifacts/4-1-consume-autodream-and-report-owned-paths-correctly.md`.

- [ ] Track and enforce isolated workspace handling when the project is in a parallel state.
  - Context: On 2026-06-15, project guidance was updated so that if a user says this project is currently in a "parallel" state, terminal work must move to an isolated branch workspace before development continues. Acceptable isolation includes a dedicated git worktree/branch or a copied sibling folder, as long as it does not affect other active partitions.
  - Current status: The 2026-06-15 TODOLIST closeout was performed from an isolated worktree and treated root dirty-work as input rather than editing it directly. The durable backlog item remains open until this isolation behavior is enforced mechanically or documented as an operational checklist.
  - Expected behavior: When "parallel" state is mentioned, create or switch to an isolated workspace first, develop on that branch/workspace, and keep the isolation status visible in this backlog until the process is fully operational.
  - Related files/docs: `AGENTS.md`, `TODOLIST.md`.

## Done

- [x] Fix pre-existing full GUI vitest environment failures.
  - Context: During Story 2.1 validation on 2026-06-14, targeted Evolution/NormalMode tests passed, but full `pnpm test` failed in unrelated suites. The backlog recorded two specific blockers: `SubAgentPanel.test.ts` lacked a working vue-i18n install/mock for `useI18n`, and `DivaPetView.test.ts` had a `lucide-vue-next` mock missing `ChevronDown`.
  - Completed: Current code already contains both fixes. On 2026-06-18, `SubAgentPanel.test.ts` was verified to mount with a real `createI18n` plugin, `DivaPetView.test.ts` was verified to mock `ChevronDown`, and the full `agent-diva-gui` vitest suite passed. This backlog item is now stale and closed as verified.
  - Verification: `pnpm test -- --run`; `rg -n "createI18n|ChevronDown" agent-diva-gui/src/components/SubAgentPanel.test.ts agent-diva-gui/src/features/diva-pet/components/DivaPetView.test.ts`.
  - Related files/docs: `agent-diva-gui/src/components/SubAgentPanel.test.ts`, `agent-diva-gui/src/features/diva-pet/components/DivaPetView.test.ts`, `TODOLIST.md`, `docs/logs/2026-06-todolist-closeout/v0.0.4-gui-vitest-backlog-closeout/verification.md`.

- [x] Clean pre-existing workspace rustfmt drift.
  - Context: During Story 1.1 validation on 2026-06-14, `cargo fmt --all -- --check` failed on unrelated pre-existing formatting diffs outside the governance domain type changes, including `agent-diva-agent`, `agent-diva-core/src/planning`, `agent-diva-manager`, and `agent-diva-sandbox` files.
  - Completed: Current workspace rustfmt drift no longer blocks validation. The remaining actual diff was narrowed to `agent-diva-agent/src/memory_boundary.rs`, then reformatted with `rustfmt` and revalidated through workspace-wide format checks.
  - Verification: `rustfmt --check agent-diva-agent/src/memory_boundary.rs`; `cargo fmt --all -- --check`; `just fmt-check`.
  - Related files/docs: `agent-diva-agent/src/memory_boundary.rs`, `TODOLIST.md`, `docs/logs/2026-06-todolist-closeout/v0.0.3-workspace-rustfmt-drift/verification.md`, `docs/logs/2026-06-governance-domain-types/v0.0.1-governance-domain-types/verification.md`.

- [x] Replace the static Mentle tool-filtering release-gate test with enabled-runtime governance-boundary coverage.
  - Context: During Epic 6 combined review on 2026-06-17, Story 6.5 Mentle regression coverage was found to validate only `MentleToolRuntimeConfig` filtering behavior. It did not exercise the higher-risk case where Mentle runtime is enabled and governance flows may still expose or depend on default Mentle routing.
  - Completed: Replaced the release-gate test with AgentLoop-level enabled-runtime coverage. The test now constructs enabled Mentle toolsets with `memtle_status`/`memtle_search`, verifies `mentle_active()` can be true, and fails if the assembled governance prompt exposes default Mentle recall/routing names or Palace Memory guidance.
  - Verification: `rustfmt --check agent-diva-agent/src/agent_loop.rs agent-diva-agent/tests/mentle_governance_boundaries.rs`; `cargo test -p agent-diva-agent --test mentle_governance_boundaries`; `just epic6-release-gate`. Package-wide `cargo fmt -p agent-diva-agent -- --check` remains blocked by pre-existing `agent-diva-agent/src/memory_boundary.rs` drift tracked under the open rustfmt TODO.
  - Related files/docs: `agent-diva-agent/src/agent_loop.rs`, `agent-diva-agent/tests/mentle_governance_boundaries.rs`, `justfile`, `_bmad-output/implementation-artifacts/6-5-add-metrics-and-release-gate-validation.md`, `docs/logs/2026-06-todolist-closeout/v0.0.2-mentle-release-gate-runtime-coverage/verification.md`.

- [x] Close verified TODOLIST items from the 2026-06 backlog triage.
  - Context: During the 2026-06-18 TODOLIST closeout, several Open items were found to be stale because current code and focused backlog entries already cover their expected behavior.
  - Completed: Confirmed `just epic6-release-gate` now includes `cargo test -p agent-diva-laputa --test authority_boundaries`; confirmed the governance direct-write guard now calls the shared authority-boundary guard instead of exempting `notebook.rs`; moved checked items out of the Open section; and closed the broad Story 5.3 validation aggregate because remaining blockers are tracked as focused Open items.
  - Verification: `git status --short --untracked-files=all`; `rg -n "epic6-release-gate|authority_boundaries|direct_write_guard|notebook.rs" justfile agent-diva-laputa/tests`; targeted inspection of `TODOLIST.md`.
  - Related files/docs: `TODOLIST.md`, `justfile`, `agent-diva-laputa/tests/direct_write_guard.rs`, `agent-diva-laputa/tests/authority_boundary_guard.rs`, `docs/logs/2026-06-todolist-closeout/v0.0.1-backlog-triage/verification.md`.

- [x] Preserve session compaction persistence when Laputa becomes the default `MemoryProvider`.
  - Context: During Epic 5 combined review on 2026-06-17, Story 5.1 was found to switch the default provider to `LaputaMemoryProvider`, but that provider returns `SyncTurnStatus::Noop` for `sync_turn`. The agent loop still runs consolidation through the active `MemoryProvider`, and the consolidation path treats `Noop` as success and advances `last_consolidated`.
  - Expected behavior: When Laputa is the authority-read provider, existing session compaction durability must either still persist through a compatible session-local path or fail loudly/degrade without falsely marking the session as consolidated.
  - Related files/docs: `agent-diva-laputa/src/memory_provider.rs`, `agent-diva-agent/src/agent_loop/loop_turn.rs`, `agent-diva-agent/src/consolidation.rs`, `_bmad-output/implementation-artifacts/5-1-plug-applied-laputa-reads-into-memoryprovider.md`.

- [x] Remove legacy `MemoryManager` fallback when Laputa authority provider initialization fails.
  - Context: During Epic 5 combined review on 2026-06-17, Story 5.1 was found to fall back to `MemoryManager` when `.laputa` exists but `LaputaMemoryProvider::open()` fails. That fallback restores legacy authority reads and legacy `MEMORY.md` / `HISTORY.md` write behavior.
  - Expected behavior: Laputa read failures should degrade safely without re-enabling legacy authority as default prompt authority and without resuming legacy authority writes outside an explicit compatibility adapter or migration path.
  - Related files/docs: `agent-diva-agent/src/agent_loop.rs`, `agent-diva-agent/src/context.rs`, `agent-diva-manager/src/runtime.rs`, `agent-diva-core/src/memory/manager.rs`, `_bmad-output/implementation-artifacts/5-1-plug-applied-laputa-reads-into-memoryprovider.md`.

- [x] Route subagent authority context through the injected `MemoryProvider` boundary instead of directly instantiating `LaputaMemoryProvider`.
  - Context: During Epic 5 combined review on 2026-06-17, Story 5.1 was found to have subagent prompt assembly directly check `.laputa` and instantiate `agent_diva_laputa::LaputaMemoryProvider`, bypassing the runtime-selected `MemoryProvider` boundary used by the main agent.
  - Expected behavior: Subagent authority context should be derived from the same runtime-selected/injected `MemoryProvider` contract as the main agent so future compatibility adapters or alternative providers do not diverge.
  - Related files/docs: `agent-diva-agent/src/subagent.rs`, `agent-diva-agent/src/agent_loop.rs`, `_bmad-output/implementation-artifacts/5-1-plug-applied-laputa-reads-into-memoryprovider.md`.

- [x] Remove default Mentle recall routing from governance prompt assembly when Mentle runtime is enabled.
  - Context: During Epic 5 combined review on 2026-06-17, Story 5.2 was found to still inject Mentle recall guidance into default governance prompt assembly whenever `mentle_active()` is true. `ContextBuilder` adds `memtle_search` / `memtle_kg_query` routing and dense-facts-to-Mentle guidance, which violates Story 5.2 AC4.
  - Expected behavior: Governance flows should not inject Mentle recall or default Mentle memory-routing guidance into prompts, even when feature-gated Mentle runtime/tools are enabled. Any future Mentle recall must stay explicit, read-only, and user-triggered.
  - Related files/docs: `agent-diva-agent/src/context.rs`, `agent-diva-agent/src/agent_loop.rs`, `_bmad-output/implementation-artifacts/5-2-enforce-mentle-governance-exclusion.md`.

- [x] Add enabled-runtime regression coverage for Mentle governance exclusion instead of only static/no-runtime guard tests.
  - Context: During Epic 5 combined review on 2026-06-17, Story 5.2 guard tests were found to validate only static tool filtering or no-Mentle temporary-directory flows. They do not exercise the real risk path where Mentle runtime is enabled and governance prompt assembly can still expose default Mentle recall guidance.
  - Expected behavior: Regression coverage should fail if governance prompt assembly or related EVO-DIVA flows depend on enabled Mentle runtime state or expose default Mentle recall routing.
  - Related files/docs: `agent-diva-agent/tests/mentle_governance_boundaries.rs`, `agent-diva-autodream/tests/mentle_governance.rs`, `agent-diva-laputa/tests/mentle_governance.rs`, `_bmad-output/implementation-artifacts/5-2-enforce-mentle-governance-exclusion.md`.

- [x] Enforce compaction-only evidence rejection at the Laputa proposal boundary, not only in AutoDream output emission.
  - Context: During Epic 5 combined review on 2026-06-17, Story 5.3 was found to reject compaction-only evidence in `agent-diva-autodream/src/outputs.rs`, but `agent-diva-laputa` proposal creation/edit validation still accepts any non-empty `evidence_refs`.
  - Expected behavior: Proposal persistence and edit boundaries should reject or downgrade compaction-only evidence sets consistently, so context compaction can never become the sole durable authority basis through alternate proposal creation paths.
  - Related files/docs: `agent-diva-core/src/evolution/types.rs`, `agent-diva-autodream/src/outputs.rs`, `agent-diva-laputa/src/proposals.rs`, `_bmad-output/implementation-artifacts/5-3-keep-context-compaction-session-local.md`.

- [x] Close Epic 4 Notebook frontend/backend linkage follow-up batch.
  - Context: During Epic 4 combined review on 2026-06-17, Notebook still had five user-visible integration gaps after Stories 4.1-4.3: malformed report files could abort the whole list, Notebook-created proposals misused `source_run_id`, session evidence search results were not attachable from the UI, failed preview requests could leave stale modal content visible, and Notebook-to-Evolution deep-links could land on the inbox without loading the target proposal detail.
  - Completed: `load_notebook_reports` now skips malformed markdown/frontmatter files with a warning instead of failing the whole list; Notebook proposal creation no longer writes a fake report id into `source_run_id`; `NotebookView.vue` now lets users search/select session hits and passes them into preview/create commands; preview failures clear stale modal state; and Evolution deep-links now carry a refresh key and resolve the target proposal after the list refreshes.
  - Verification: `cargo test -p agent-diva-gui notebook`; `pnpm test -- --run src/components/NotebookView.test.ts src/components/EvolutionView.test.ts`.
  - Related files/docs: `agent-diva-gui/src-tauri/src/notebook.rs`, `agent-diva-gui/src/components/NotebookView.vue`, `agent-diva-gui/src/components/EvolutionView.vue`, `docs/logs/2026-06-epic4-notebook-closeout/v0.0.1-notebook-ui-backend-linkage/verification.md`.

- [x] Fix Story 4.4 regression tests to compile after the `session_hits` parameter was added.
  - Context: During Epic 4 combined review on 2026-06-17, Story 4.4 was flagged because Notebook proposal preview/build call sites needed to pass the new `session_hits` argument after the session-evidence attachment work landed.
  - Completed: Current `agent-diva-gui/src-tauri/src/notebook.rs` test call sites pass the updated signatures, including `proposal_build_does_not_mutate_any_authority_paths_before_apply`, and the Notebook test suite covers the `session_hits` path without compile drift.
  - Verification: `cargo test -p agent-diva-gui notebook`.
  - Related files/docs: `agent-diva-gui/src-tauri/src/notebook.rs`, `docs/logs/2026-06-epic4-notebook-closeout/v0.0.2-notebook-low-risk-closeout/verification.md`, `_bmad-output/implementation-artifacts/4-4-add-solidification-regression-coverage.md`.

- [x] Hold the Laputa global write lock during legacy migration commits.
  - Context: During Epic 6 combined review on 2026-06-17, Story 6.1 migration was found to write section files and `state.json` without taking the same Laputa write lock used by apply/rollback flows. This means migration can race normal governance writes and violate the single durable authority write boundary.
  - Completed: Migration now acquires the Laputa proposals write lock before staging and committing section/state writes, matching the apply/rollback governance boundary.
  - Related files/docs: `agent-diva-laputa/src/migration.rs`, `agent-diva-laputa/src/proposals.rs`, `agent-diva-laputa/src/service.rs`, `_bmad-output/implementation-artifacts/6-1-implement-legacy-schema-migration.md`.

- [x] Preserve existing `state.json` fields when updating Laputa schema version during migration.
  - Context: During Epic 6 combined review on 2026-06-17, Story 6.1 migration was found to rewrite `state.json` with a migration-only JSON payload containing `schema_version` and `legacy_migration`, instead of reading and updating the existing state document.
  - Completed: Migration now merges the upgraded schema version and `legacy_migration` metadata into the existing state payload instead of dropping unrelated top-level fields.
  - Related files/docs: `agent-diva-laputa/src/migration.rs`, `_bmad-output/implementation-artifacts/6-1-implement-legacy-schema-migration.md`.

- [x] Discover and migrate root-level legacy `MEMORY.md` and `HISTORY.md` files, not only `memory/` copies.
  - Context: During Epic 6 combined review on 2026-06-17, Story 6.1 legacy source discovery was found to scan `memory/MEMORY.md` and `memory/HISTORY.md` but not root-level `MEMORY.md` or `HISTORY.md`, even though other governance tests still treat root-level authority files as real legacy paths.
  - Completed: Legacy discovery now includes root-level `MEMORY.md` and `HISTORY.md` alongside the nested `memory/` copies.
  - Related files/docs: `agent-diva-laputa/src/migration.rs`, `agent-diva-laputa/tests/governance_proof_loop.rs`, `agent-diva-laputa/tests/migration.rs`, `_bmad-output/implementation-artifacts/6-1-implement-legacy-schema-migration.md`.

- [x] Stop treating `BOOTSTRAP.md` as runtime prompt authority after Laputa migration/import.
  - Context: During Epic 6 combined review on 2026-06-17, Story 6.1 marked `BOOTSTRAP.md` as bootstrap-only migration input, but runtime prompt assembly still reads and injects `BOOTSTRAP.md` directly after migration. This leaves a legacy prompt-authority path outside Laputa.
  - Completed: `BOOTSTRAP.md` is no longer injected into the runtime prompt; the builder only marks bootstrap completion state when bootstrap gating is active.
  - Related files/docs: `agent-diva-laputa/src/migration.rs`, `agent-diva-agent/src/context.rs`, `_bmad-output/implementation-artifacts/6-1-implement-legacy-schema-migration.md`.

- [x] Return the migrated Laputa schema version through read APIs instead of hardcoding `1.0.0`.
  - Context: During Epic 6 combined review on 2026-06-17, Story 6.1 migration wrote schema version `1.1.0` into state, but `LaputaService` snapshot/section responses still report hardcoded schema version `1.0.0`.
  - Completed: Laputa read APIs now surface the schema version stored in `.laputa/state.json` so callers see the migrated value.
  - Related files/docs: `agent-diva-laputa/src/migration.rs`, `agent-diva-laputa/src/service.rs`, `_bmad-output/implementation-artifacts/6-1-implement-legacy-schema-migration.md`.

- [x] Remove the broad production-file allowlist from the governance direct-write guard.
  - Context: During Epic 6 combined review on 2026-06-17, Story 6.4 direct-write guard was found to exempt whole production files including `context.rs`, `subagent.rs`, and `notebook.rs`. That allowlist could hide future forbidden authority-path writes in exactly the runtime surfaces the guard is meant to protect.
  - Completed: Replaced the file allowlist with a shared authority-boundary guard and kept only the explicit bootstrap snippet exception for the legacy read path.
  - Related files/docs: `agent-diva-laputa/tests/direct_write_guard.rs`, `agent-diva-laputa/tests/authority_boundary_guard.rs`, `agent-diva-laputa/tests/authority_boundaries.rs`, `_bmad-output/implementation-artifacts/6-4-add-end-to-end-governance-proof-loop.md`.

- [x] Make the governance proof loop invoke the Story 6.3 authority-boundary guard instead of only checking a few output files.
  - Context: During Epic 6 combined review on 2026-06-17, Story 6.4 `governance_proof_loop` was found to assert only that a short list of external files was not written. It did not actually embed or call the direct-write/read boundary guard from Story 6.3, so AC2 was not fully proven.
  - Completed: `governance_proof_loop` now calls the shared authority-boundary guard directly before the legacy file absence checks.
  - Related files/docs: `agent-diva-laputa/tests/governance_proof_loop.rs`, `agent-diva-laputa/tests/authority_boundary_guard.rs`, `agent-diva-laputa/tests/authority_boundaries.rs`, `agent-diva-laputa/tests/direct_write_guard.rs`, `_bmad-output/implementation-artifacts/6-4-add-end-to-end-governance-proof-loop.md`.

- [x] Fix current Story 5.2 validation blockers outside Mentle governance exclusion.
  - Context: During Story 5.2 validation on 2026-06-15, targeted 5.2 guardrails passed, but full story-required validation was blocked by unrelated current test failures. `cargo test -p agent-diva-autodream` failed in `inputs::tests::collector_marks_compaction_capsules_as_secondary_evidence`, and `cargo test -p agent-diva-laputa` failed to compile existing tests because `LaputaMigrationTestFailure::AfterSectionCommitBeforeState` and `LaputaService::apply_proposal_with_options` were absent.
  - Completed: Current code includes the compaction secondary-evidence marker path and the missing Laputa migration/apply APIs. Verification on 2026-06-15 in the isolated TODOLIST closeout worktree passed: `cargo test -p agent-diva-autodream inputs::tests::collector_marks_compaction_capsules_as_secondary_evidence`; `cargo test -p agent-diva-laputa`. A read-only subagent also reported full `cargo test -p agent-diva-autodream` passing.
  - Related files/docs: `agent-diva-autodream/src/inputs.rs`, `agent-diva-laputa/src/migration.rs`, `agent-diva-laputa/src/service.rs`, `agent-diva-laputa/tests/migration.rs`, `agent-diva-laputa/tests/service.rs`.

- [x] Fix pre-existing `agent-diva-laputa` clippy failures blocking `just check`.
  - Context: During Story 3.3 validation on 2026-06-14, `just check` failed outside the AutoDream worker changes. Reported issues included `too_many_arguments` in proposal rollback handling and `manual_inspect` in rollback cleanup error handling.
  - Completed: Laputa-specific clippy cleanup is verified. Subagent validation passed `cargo clippy -p agent-diva-laputa --all-targets -- -D warnings`; isolated closeout validation also passed `cargo test -p agent-diva-laputa`. Workspace `just check` may still be blocked by non-Laputa TODOs tracked in Open.
  - Related files/docs: `agent-diva-laputa/src/proposals.rs`, `agent-diva-laputa/src/service.rs`; `docs/logs/2026-06-autodream-restricted-worker/v0.0.1-restricted-reflection-worker/verification.md`.

- [x] Fix pre-existing `agent-diva-sandbox` compile failures blocking `just test`.
  - Context: During Story 1.3 validation on 2026-06-14, `just test` failed outside the Laputa proposal changes. Errors included missing `lock_exclusive` method for `std::fs::File` in `agent-diva-sandbox/src/exec_policy.rs` and `bool`/`&bool` match arm mismatch in `agent-diva-sandbox/src/platform/macos.rs`.
  - Completed: The original sandbox compile failures are resolved in current dirty-work; isolated closeout verification passed `cargo check -p agent-diva-sandbox`, and `cargo test -p agent-diva-sandbox` now gets past compilation into test execution. That test command still fails one assertion, and the remaining all-target lint/test failures are tracked under the current Open sandbox validation TODO.
  - Related files/docs: `agent-diva-sandbox/Cargo.toml`, `agent-diva-sandbox/src/exec_policy.rs`, `agent-diva-sandbox/src/platform/macos.rs`; `docs/logs/2026-06-laputa-proposals/v0.0.1-proposal-crud-state-transitions/verification.md`.

- [x] Fix current `agent-diva-sandbox` all-target validation blockers after compile cleanup.
  - Context: During TODOLIST closeout on 2026-06-15, dirty-work verified that the original sandbox compile blockers are resolved: `cargo check -p agent-diva-sandbox` passes after adding Unix `fs2`, importing `fs2::FileExt`, fixing `bool`/`&bool` matches, and removing an unused `WritableRoot` import. The remaining all-target blockers were an unused-import lint in `agent-diva-sandbox/src/platform/macos.rs` tests and a faulty shell-injection assertion in `manager::tests::test_to_command_string_prevents_shell_injection`.
  - Completed: Removed the unused test imports and corrected the injection regression test so it asserts exact literal-argument output instead of treating a literal substring as evidence of injection. Verification on 2026-06-15 passed: `cargo clippy -p agent-diva-sandbox --all-targets -- -D warnings`; `cargo test -p agent-diva-sandbox`.
  - Related files/docs: `agent-diva-sandbox/src/manager.rs`, `agent-diva-sandbox/src/platform/macos.rs`.

- [x] Fix current `agent-diva-gui` Rust compile blockers outside Story 6.2.
  - Context: During Story 6.2 validation on 2026-06-15, `just test` failed outside Laputa changes while compiling `agent-diva-gui`. Current dirty-work adds the missing `agent-diva-agent` dependency, so unresolved `agent_diva_agent` imports/usages are no longer the observed blocker. The remaining compile failures were `AgentState::http_client()` missing in `src-tauri/src/commands.rs` and `WebviewWindowBuilder::transparent` being unavailable on macOS without the matching Tauri private-API feature/config.
  - Completed: Switched the Laputa PUT helper back to the existing `state.client` field, enabled Tauri `macos-private-api`, and aligned `tauri.conf.json` with `app.macOSPrivateApi = true` so the desktop-pet transparent window API matches the configured window behavior. Verification on 2026-06-15 passed: `cargo check -p agent-diva-gui`.
  - Related files/docs: `agent-diva-gui/src-tauri/src/commands.rs`, `agent-diva-gui/src-tauri/Cargo.toml`, `agent-diva-gui/src-tauri/tauri.conf.json`.

- [x] Fix `agent-diva-manager` skill service unit tests under the current built-in skill fixture set.
  - Context: During Epic 2/3 review remediation validation on 2026-06-15, `cargo test -p agent-diva-core -p agent-diva-laputa -p agent-diva-manager` passed core/Laputa tests but failed two unrelated manager tests: `skill_service::tests::delete_workspace_skill_and_restore_builtin_view` unwraps a missing `weather` built-in skill, and `skill_service::tests::delete_builtin_skill_is_rejected` no longer receives a "builtin" error message.
  - Completed: Added a test-only builtin-skill directory injection path for `SkillService` and updated the two skill-service tests to create their own temporary builtin `weather` fixture instead of depending on a missing repository-level builtin directory. Verification on 2026-06-15 passed: `cargo test -p agent-diva-manager delete_workspace_skill_and_restore_builtin_view`; `cargo test -p agent-diva-manager delete_builtin_skill_is_rejected`.
  - Related files/docs: `agent-diva-manager/src/skill_service.rs`, `docs/logs/2026-06-epic23-review-remediation/v0.0.1-evolution-governance-remediation/verification.md`.

- [x] Fix pre-existing `agent-diva-agent` clippy failures blocking `just check`.
  - Context: During Story 1.3 validation on 2026-06-14, `just check` failed outside the Laputa proposal changes. Earlier reports mentioned unused `ToolPolicy` import and unread `parent_tool_limits` in `agent-diva-agent/src/subagent.rs`, needless borrows in `agent-diva-agent/src/agent_loop/loop_turn.rs` and `agent-diva-agent/src/context.rs`, and manual char comparison in `agent-diva-agent/src/mask/mask_file.rs`. Follow-up 2026-06-15 validation in the isolated worktree showed the remaining crate-level blockers had drifted to a new set of test and helper lints in `agent_loop.rs`, `context.rs`, and compaction tests.
  - Completed: Cleaned the remaining `agent-diva-agent` all-target clippy failures by removing redundant test imports, replacing `&[value.clone()]` with `std::slice::from_ref`, removing needless borrows for file-manager paths, and initializing default-backed test structs without field reassignment. Verification on 2026-06-15 passed: `cargo clippy -p agent-diva-agent --all-targets -- -D warnings`.
  - Related files/docs: `agent-diva-agent/src/agent_loop.rs`, `agent-diva-agent/src/context.rs`, `agent-diva-agent/tests/compaction_integration.rs`, `agent-diva-agent/tests/compaction_real_test.rs`; `docs/logs/2026-06-laputa-proposals/v0.0.1-proposal-crud-state-transitions/verification.md`.

- [x] Fix pre-existing mask/runtime warnings in `agent-diva-agent` blocking warning-free validation.
  - Context: During Story 2.2 validation on 2026-06-14, targeted `cargo test -p agent-diva-agent mask::mask_registry` and `mask::tool_policy` passed, but both surfaced unrelated existing warnings in the agent crate. By 2026-06-15, the warning surface had shifted to compaction and helper-test imports captured by full clippy validation.
  - Completed: The remaining warning-producing imports/helpers in the current agent crate test surface were removed as part of the all-target clippy cleanup, restoring warning-free validation under `-D warnings`. Verification on 2026-06-15 passed: `cargo clippy -p agent-diva-agent --all-targets -- -D warnings`.
  - Related files/docs: `agent-diva-agent/src/agent_loop.rs`, `agent-diva-agent/tests/compaction_integration.rs`, `agent-diva-agent/tests/compaction_real_test.rs`; `docs/logs/2026-06-reviewer-assist-readonly/v0.0.1-reviewer-assist-readonly/verification.md`.

- [x] Fix current Story 2.4 GUI validation blockers outside Evolution views.
  - Context: During Story 2.4 validation on 2026-06-15, `pnpm build` failed on unrelated unused symbols/import-path errors, and the named targeted vitest blockers were `SubAgentPanel.test.ts` missing a working vue-i18n install/mock and `DivaPetView.test.ts` missing `ChevronDown` in its lucide mock.
  - Completed: Cleaned the unrelated GUI `vue-tsc` blockers, installed a real test i18n plugin in `SubAgentPanel.test.ts`, filled the `ChevronDown` lucide mock for `DivaPetView.test.ts`, and updated the stale mood assertion to match the current embedded-host mood behavior. Verification on 2026-06-15 passed: `pnpm build`; `pnpm test -- --run SubAgentPanel.test.ts DivaPetView.test.ts`.
  - Related files/docs: `agent-diva-gui/src/components/SubAgentPanel.test.ts`, `agent-diva-gui/src/features/diva-pet/components/DivaPetView.test.ts`, `agent-diva-gui/src/components/DecisionCard.vue`, `agent-diva-gui/src/components/NotebookView.vue`, `agent-diva-gui/src/components/TodoCard.vue`, `docs/logs/2026-06-evolution-runs-audit-policy/v0.0.1-runs-audit-policy-views/verification.md`.

- [x] Fix pre-existing GUI build blockers outside Evolution workspace work.
  - Context: During Story 2.1 validation on 2026-06-14, `pnpm build` in `agent-diva-gui` failed after touched-file type issues were fixed. Remaining failures were unrelated existing issues: unused imports/variables in `DecisionCard.vue`, `NotebookView.vue`, several settings components, and an invalid `../../api/desktop` import in `TodoCard.vue`.
  - Completed: Removed the stale unused imports/locals across the affected components and corrected the `TodoCard.vue` API import path. Verification on 2026-06-15 passed: `pnpm build`.
  - Related files/docs: `agent-diva-gui/src/components/DecisionCard.vue`, `agent-diva-gui/src/components/NotebookView.vue`, `agent-diva-gui/src/components/settings/ChannelsSettings.vue`, `agent-diva-gui/src/components/settings/ChannelWizardModal.vue`, `agent-diva-gui/src/components/settings/MarketplaceTab.vue`, `agent-diva-gui/src/components/settings/ProvidersCardView.vue`, `agent-diva-gui/src/components/settings/ProvidersSettings.vue`, `agent-diva-gui/src/components/settings/SandboxSettingsSection.vue`, `agent-diva-gui/src/components/TodoCard.vue`.

- [x] Split current Story 2.4 workspace validation blockers into active focused TODOs.
  - Context: During Story 2.4 validation on 2026-06-15, a single workspace blocker entry mixed sandbox compile issues, Laputa lint issues, and GUI validation failures.
  - Completed: Follow-up verification shows the named Laputa lint blocker is resolved (`cargo clippy -p agent-diva-laputa --all-targets -- -D warnings`) and the named sandbox compile blockers are resolved (`cargo check -p agent-diva-sandbox`). Remaining sandbox all-target lint/test and GUI build/test failures stay open under their focused TODOs.
  - Related files/docs: `agent-diva-sandbox/src/exec_policy.rs`, `agent-diva-sandbox/src/platform/macos.rs`, `agent-diva-laputa/src/memory_provider.rs`, `docs/logs/2026-06-evolution-runs-audit-policy/v0.0.1-runs-audit-policy-views/verification.md`.

- [x] Split pre-existing Story 3.1 validation blockers into active focused TODOs.
  - Context: During Story 3.1 validation on 2026-06-14, one backlog entry mixed `agent-diva-agent` rustfmt drift, GUI Tauri compile failures, and sandbox compile blockers.
  - Completed: Follow-up verification shows the sandbox compile blocker no longer applies (`cargo check -p agent-diva-sandbox` passed). Remaining agent formatting/lint and GUI compile blockers remain tracked under the current focused Open TODOs.
  - Related files/docs: `agent-diva-agent/src/agent_loop/loop_turn.rs`, `agent-diva-agent/src/context.rs`, `agent-diva-agent/src/mask/*`, `agent-diva-agent/src/planning/*`, `agent-diva-agent/src/subagent.rs`, `agent-diva-agent/src/tool_assembly.rs`, `agent-diva-gui/src-tauri/src/commands.rs`; `docs/logs/2026-06-autodream-manual-run-lifecycle/v0.0.1-manual-run-lifecycle/verification.md`.

- [x] Fix pre-existing `agent-diva-manager` AutoDream error match compile blocker.
  - Context: During Story 5.1 validation on 2026-06-14, `cargo check -p agent-diva-manager` reached unrelated existing code and failed because `agent-diva-manager/src/handlers/autodream.rs` did not handle `AutoDreamError::InputCollection(_)` and `AutoDreamError::ProposalPersistence(_)`.
  - Completed: Epic 2/3 review remediation mapped both variants to the existing AutoDream internal error response class, and `cargo check -p agent-diva-core -p agent-diva-laputa -p agent-diva-manager -p agent-diva-migration` passed on 2026-06-15.
  - Related files/docs: `agent-diva-manager/src/handlers/autodream.rs`, `agent-diva-autodream/src/error.rs`; `_bmad-output/implementation-artifacts/5-1-plug-applied-laputa-reads-into-memoryprovider.md`.

- [x] Complete Story 2.2 inbox behavior that Story 2.3 still depends on.
  - Context: During Story 2.3 implementation on 2026-06-14, the proposal detail pane and governance actions were added on top of the existing Evolution inbox shell, but Story 2.2 requirements remained incomplete: row metadata density, filters, keyboard navigation, and batch actions.
  - Completed: Story 2.2 now has a dedicated `ProposalInbox.vue` with dense row metadata, filters, keyboard navigation, legal batch actions, local read/defer markers, visible loading/empty/error states, and focused GUI tests.
  - Related files/docs: `_bmad-output/implementation-artifacts/2-2-build-proposal-inbox-list-and-filters.md`, `agent-diva-gui/src/components/EvolutionView.vue`, `agent-diva-gui/src/components/evolution/ProposalInbox.vue`, `docs/logs/2026-06-epic-2-inbox/v0.0.1-proposal-inbox-list-and-filters/verification.md`.

- [x] Improve GUI image input experience for multimodal vision.
  - Context: The current image recognition path supports image file attachments, but direct clipboard image paste in the GUI composer is not implemented.
  - Expected boundary: This is a planned GUI/product optimization, not a backend vision-chain failure.
  - Target behavior: Pasted clipboard images should be captured, uploaded through the existing file attachment path, displayed as an image chip or preview, and handled consistently with model vision capability checks.
  - Related docs: `docs/logs/2026-06-multimodal-gui-boundary/v0.0.8-gui-paste-boundary/summary.md`
  - **Closed**: Implemented in `53bc086 feat(gui): add clipboard image paste support in composer`. All target behaviors covered: clipboard capture via `handlePaste`, upload through existing `uploadFile` API, image preview via attachments UI, multi-image support.

- [x] **2026-06-11 sandbox clippy cleanup** — 预存 lint 全清
  `cargo clippy -p agent-diva-sandbox --all-targets -- -D warnings` 12 个预存 error 全部修复：
  - `c790417 fix(sandbox): clean clippy issues in windows.rs` — `items_after_test_module`（helper 移到 mod tests 前）+ `field_reassign_with_default`（STARTUPINFOW 改 struct update syntax）
  - `bc1b7b5 fix(sandbox): clean clippy issues in orchestrator.rs` — `items_after_test_module` 同类问题
  - `ddaac54 docs(logs): record sandbox clippy cleanup iteration` — `docs/logs/2026-06-sandbox-clippy-cleanup/`
  - 验证: `cargo clippy -p agent-diva-sandbox --all-targets -- -D warnings` 退出码0；`cargo test -p agent-diva-sandbox` 99 passed

- [x] **2026-06-11 sandbox audit remediation batch** — 10 项未修项全部关闭
  原始 sandbox 审计 (2026-06-01) 2 CRITICAL + 4 HIGH + 3 MEDIUM + 3 P3，本批一次性收口：
  - P0-1 `90a1139 fix(sandbox): prevent shell metacharacter injection` — `format!`+`join(" ")` 改为 argv 数组传入
  - P1-4 `caad65a fix(sandbox): fail closed when macos sandbox is unavailable` — `is_available()==false` 改返 `Err(PlatformUnavailable)`
  - P1-5 `a473a57 fix(sandbox): tighten guardian default approvals` — `GuardianConfig::default()` 改 `auto_approve_known_safe=false`, `enable_auto_learning=false`
  - P1-6 `c3e1d54 fix(sandbox): ban interpreter path aliases` — `is_banned_prefix` 改 basename 匹配，覆盖 `/usr/bin/python3` 等
  - P2-7 `206f9fc fix(sandbox): expand protected path coverage` — `default_protected_paths()` 扩展 `.env.*`, `.npmrc`, `*.tfvars`, `id_rsa*`, `.aws/credentials` 等
  - P2-10 `aa6e2f4 fix(sandbox): surface non-zero exits as errors` — 非零退出码改返 `Err(ExecutionFailed { code, stdout, stderr })`
  - P2-9 `47f3195 refactor(sandbox): unify approval cache access` — manager 暴露高层方法，orchestrator 不再直 `approval_store().lock()`
  - P3-11 `b01430f refactor(sandbox): decouple orchestrator run steps` — `run()` 拆为 5 个 step 函数，外部 API 等价
  - P3-12 `c857889 refactor(sandbox): add crate feature gates` — `Cargo.toml` 加 `default/manager/orchestrator/platform-*` features，`lib.rs` 用 `#[cfg]` 控制
  - P3-13 `8c8cc7f refactor(sandbox): bridge sandbox and security policies` — `SandboxPolicy ↔ SecurityPolicy` 双向转换 + `#[deprecated]` 标注 + 映射表文档
  - 验证: `cargo check --workspace --all-targets` 通过；`cargo test -p agent-diva-sandbox` 99 passed (含新增 injection/exit-code/policy-mapping 用例)
  - 路线: 先在 `agent-diva-pro` 验证稳定；后续视情况抽取 backend/runtime 安全能力回流 `agent-diva` 主线
