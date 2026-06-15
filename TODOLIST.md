# TODOLIST

This file is the project-level backlog for bugs, gaps, and unfinished work found during development or review.

## Open

- [ ] Track and enforce isolated workspace handling when the project is in a parallel state.
  - Context: On 2026-06-15, project guidance was updated so that if a user says this project is currently in a "parallel" state, terminal work must move to an isolated branch workspace before development continues. Acceptable isolation includes a dedicated git worktree/branch or a copied sibling folder, as long as it does not affect other active partitions.
  - Current status: The 2026-06-15 TODOLIST closeout was performed from an isolated worktree and treated root dirty-work as input rather than editing it directly. The durable backlog item remains open until this isolation behavior is enforced mechanically or documented as an operational checklist.
  - Expected behavior: When "parallel" state is mentioned, create or switch to an isolated workspace first, develop on that branch/workspace, and keep the isolation status visible in this backlog until the process is fully operational.
  - Related files/docs: `AGENTS.md`, `TODOLIST.md`.

- [ ] Fix current Story 5.3 workspace validation blockers outside context compaction guardrails.
  - Context: During Story 5.3 validation on 2026-06-15, targeted 5.3 tests passed, but workspace validation was blocked by unrelated existing issues. Follow-up verification in isolated TODOLIST closeout shows the Laputa fmt/clippy blockers are resolved and `cargo test -p agent-diva-laputa` passes; current dirty-work also ignores `compaction_real_test` behind `TEAKACLOUD_API_KEY` and adds the GUI `agent-diva-agent` dependency. Remaining blocker is now limited to `cargo check -p agent-diva-gui`, which still fails on missing `AgentState::http_client()` and unavailable `WebviewWindowBuilder::transparent`.
  - Expected behavior: Workspace format, clippy, and test gates should pass without unrelated sandbox or GUI blockers.
  - Related files/docs: `agent-diva-sandbox/src/manager.rs`, `agent-diva-sandbox/src/platform/macos.rs`, `agent-diva-gui/src-tauri/src/commands.rs`, `agent-diva-agent/tests/compaction_real_test.rs`, `_bmad-output/implementation-artifacts/5-3-keep-context-compaction-session-local.md`.

- [ ] Fix current `agent-diva-gui` Rust compile blockers outside Story 6.2.
  - Context: During Story 6.2 validation on 2026-06-15, `just test` failed outside Laputa changes while compiling `agent-diva-gui`. Current dirty-work adds the missing `agent-diva-agent` dependency, so unresolved `agent_diva_agent` imports/usages are no longer the observed blocker. `cargo check -p agent-diva-gui` still fails in `src-tauri/src/commands.rs` because `AgentState::http_client()` is missing, the response type cannot be inferred from that call chain, and `WebviewWindowBuilder::transparent` is unavailable in the current Tauri API.
  - Expected behavior: Workspace `cargo test --all` should compile `agent-diva-gui` without unrelated Tauri command compile failures.
  - Related files/docs: `agent-diva-gui/src-tauri/src/commands.rs`, `agent-diva-gui/src-tauri/Cargo.toml`.

- [ ] Fix current Story 2.4 GUI validation blockers outside Evolution views.
  - Context: During Story 2.4 validation on 2026-06-15, `pnpm build` failed on unrelated unused symbols/import-path errors, and full `pnpm test` failed in unrelated suites: `SubAgentPanel.test.ts` needs an i18n install/mock and `DivaPetView.test.ts` needs `ChevronDown` in its lucide mock.
  - Expected behavior: Full `agent-diva-gui` build and vitest suite should pass so Evolution stories can be promoted without unrelated waivers.
  - Related files/docs: `agent-diva-gui/src/components/SubAgentPanel.test.ts`, `agent-diva-gui/src/features/diva-pet/components/DivaPetView.test.ts`, `agent-diva-gui/src/components/DecisionCard.vue`, `agent-diva-gui/src/components/NotebookView.vue`, `agent-diva-gui/src/components/TodoCard.vue`, `docs/logs/2026-06-evolution-runs-audit-policy/v0.0.1-runs-audit-policy-views/verification.md`.

- [ ] Fix `agent-diva-manager` skill service unit tests under the current built-in skill fixture set.
  - Context: During Epic 2/3 review remediation validation on 2026-06-15, `cargo test -p agent-diva-core -p agent-diva-laputa -p agent-diva-manager` passed core/Laputa tests but failed two unrelated manager tests: `skill_service::tests::delete_workspace_skill_and_restore_builtin_view` unwraps a missing `weather` built-in skill, and `skill_service::tests::delete_builtin_skill_is_rejected` no longer receives a "builtin" error message.
  - Expected behavior: `agent-diva-manager` unit tests should either create the built-in fixture they assert against or assert against a built-in skill that exists in the current test fixture set.
  - Related files/docs: `agent-diva-manager/src/skill_service.rs`, `docs/logs/2026-06-epic23-review-remediation/v0.0.1-evolution-governance-remediation/verification.md`.

- [ ] Fix pre-existing GUI build blockers outside Evolution workspace work.
  - Context: During Story 2.1 validation on 2026-06-14, `pnpm build` in `agent-diva-gui` failed after touched-file type issues were fixed. Remaining failures are unrelated existing issues: unused imports/variables in `DecisionCard.vue`, `NotebookView.vue`, several settings components, and an invalid `../../api/desktop` import in `TodoCard.vue`.
  - Expected behavior: `agent-diva-gui` should pass `vue-tsc --noEmit && vite build` without unrelated no-unused and path-resolution failures.
  - Related files/docs: `agent-diva-gui/src/components/DecisionCard.vue`, `agent-diva-gui/src/components/NotebookView.vue`, `agent-diva-gui/src/components/settings/ChannelsSettings.vue`, `agent-diva-gui/src/components/settings/ChannelWizardModal.vue`, `agent-diva-gui/src/components/settings/MarketplaceTab.vue`, `agent-diva-gui/src/components/settings/ProvidersCardView.vue`, `agent-diva-gui/src/components/settings/ProvidersSettings.vue`, `agent-diva-gui/src/components/settings/SandboxSettingsSection.vue`, `agent-diva-gui/src/components/TodoCard.vue`.

- [ ] Fix pre-existing full GUI vitest environment failures.
  - Context: During Story 2.1 validation on 2026-06-14, targeted Evolution/NormalMode tests passed, but full `pnpm test` failed in unrelated suites. `SubAgentPanel.test.ts` lacks a vue-i18n install/mock for `useI18n`, and `DivaPetView.test.ts` has a `lucide-vue-next` mock missing `ChevronDown`.
  - Expected behavior: Full `agent-diva-gui` vitest suite should pass after shared test setup/mocks are aligned with current components.
  - Related files/docs: `agent-diva-gui/src/components/SubAgentPanel.test.ts`, `agent-diva-gui/src/features/diva-pet/components/DivaPetView.test.ts`.

- [ ] Fix pre-existing `agent-diva-agent` clippy failures blocking `just check`.
  - Context: During Story 1.3 validation on 2026-06-14, `just check` failed outside the Laputa proposal changes. Reported issues include unused `ToolPolicy` import and unread `parent_tool_limits` in `agent-diva-agent/src/subagent.rs`, needless borrows in `agent-diva-agent/src/agent_loop/loop_turn.rs` and `agent-diva-agent/src/context.rs`, and manual char comparison in `agent-diva-agent/src/mask/mask_file.rs`.
  - Expected behavior: Workspace `cargo clippy --all -- -D warnings` should pass after unrelated pre-existing lint cleanup.
  - Related files/docs: `agent-diva-agent/src/subagent.rs`, `agent-diva-agent/src/agent_loop/loop_turn.rs`, `agent-diva-agent/src/context.rs`, `agent-diva-agent/src/mask/mask_file.rs`; `docs/logs/2026-06-laputa-proposals/v0.0.1-proposal-crud-state-transitions/verification.md`.

- [ ] Fix pre-existing mask/runtime warnings in `agent-diva-agent` blocking warning-free validation.
  - Context: During Story 2.2 validation on 2026-06-14, targeted `cargo test -p agent-diva-agent mask::mask_registry` and `mask::tool_policy` passed, but both surfaced unrelated existing warnings: an unused `ToolPolicy` import and unread `parent_tool_limits` field in `agent-diva-agent/src/subagent.rs`.
  - Expected behavior: Focused agent crate validation for mask stories should run without unrelated warnings so stricter `-D warnings` checks can be trusted.
  - Related files/docs: `agent-diva-agent/src/subagent.rs`; `docs/logs/2026-06-reviewer-assist-readonly/v0.0.1-reviewer-assist-readonly/verification.md`.

- [ ] Clean pre-existing workspace rustfmt drift.
  - Context: During Story 1.1 validation on 2026-06-14, `cargo fmt --all -- --check` failed on unrelated pre-existing formatting diffs outside the governance domain type changes, including `agent-diva-agent`, `agent-diva-core/src/planning`, `agent-diva-manager`, and `agent-diva-sandbox` files.
  - Expected behavior: Workspace-wide format check should pass without requiring unrelated formatting churn during focused story work.
  - Related files/docs: validation output for Story 1.1; `docs/logs/2026-06-governance-domain-types/v0.0.1-governance-domain-types/verification.md`.

## Done

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
