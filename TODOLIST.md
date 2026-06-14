# TODOLIST

This file is the project-level backlog for bugs, gaps, and unfinished work found during development or review.

## Open

- [ ] Fix pre-existing `agent-diva-laputa` clippy failures blocking `just check`.
  - Context: During Story 3.3 validation on 2026-06-14, `just check` failed outside the AutoDream worker changes. Reported issues include `too_many_arguments` in proposal rollback handling and `manual_inspect` in rollback cleanup error handling.
  - Expected behavior: Workspace `cargo clippy --all -- -D warnings` should pass after unrelated Laputa lint cleanup.
  - Related files/docs: `agent-diva-laputa/src/proposals.rs`, `agent-diva-laputa/src/service.rs`; `docs/logs/2026-06-autodream-restricted-worker/v0.0.1-restricted-reflection-worker/verification.md`.

- [ ] Resolve pre-existing Story 3.1 validation blockers outside AutoDream.
  - Context: During Story 3.1 validation on 2026-06-14, `cargo fmt --check` failed on unrelated existing rustfmt drift in `agent-diva-agent`, and `cargo check -p agent-diva-gui` failed on unrelated existing `agent-diva-sandbox` compile errors.
  - Expected behavior: workspace formatting and GUI Tauri compile validation should pass after unrelated rustfmt and sandbox compile blockers are fixed.
  - Related files/docs: `agent-diva-agent/src/agent_loop/loop_turn.rs`, `agent-diva-agent/src/context.rs`, `agent-diva-agent/src/mask/*`, `agent-diva-agent/src/planning/*`, `agent-diva-agent/src/subagent.rs`, `agent-diva-agent/src/tool_assembly.rs`, `agent-diva-sandbox/src/exec_policy.rs`, `agent-diva-sandbox/src/platform/macos.rs`; `docs/logs/2026-06-autodream-manual-run-lifecycle/v0.0.1-manual-run-lifecycle/verification.md`.

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

- [ ] Fix pre-existing `agent-diva-sandbox` compile failures blocking `just test`.
  - Context: During Story 1.3 validation on 2026-06-14, `just test` failed outside the Laputa proposal changes. Errors include missing `lock_exclusive` method for `std::fs::File` in `agent-diva-sandbox/src/exec_policy.rs` and `bool`/`&bool` match arm mismatch in `agent-diva-sandbox/src/platform/macos.rs`.
  - Expected behavior: Workspace `cargo test --all` should compile all crates and run tests without unrelated sandbox failures.
  - Related files/docs: `agent-diva-sandbox/src/exec_policy.rs`, `agent-diva-sandbox/src/platform/macos.rs`; `docs/logs/2026-06-laputa-proposals/v0.0.1-proposal-crud-state-transitions/verification.md`.

- [ ] Fix pre-existing mask/runtime warnings in `agent-diva-agent` blocking warning-free validation.
  - Context: During Story 2.2 validation on 2026-06-14, targeted `cargo test -p agent-diva-agent mask::mask_registry` and `mask::tool_policy` passed, but both surfaced unrelated existing warnings: an unused `ToolPolicy` import and unread `parent_tool_limits` field in `agent-diva-agent/src/subagent.rs`.
  - Expected behavior: Focused agent crate validation for mask stories should run without unrelated warnings so stricter `-D warnings` checks can be trusted.
  - Related files/docs: `agent-diva-agent/src/subagent.rs`; `docs/logs/2026-06-reviewer-assist-readonly/v0.0.1-reviewer-assist-readonly/verification.md`.

- [ ] Clean pre-existing workspace rustfmt drift.
  - Context: During Story 1.1 validation on 2026-06-14, `cargo fmt --all -- --check` failed on unrelated pre-existing formatting diffs outside the governance domain type changes, including `agent-diva-agent`, `agent-diva-core/src/planning`, `agent-diva-manager`, and `agent-diva-sandbox` files.
  - Expected behavior: Workspace-wide format check should pass without requiring unrelated formatting churn during focused story work.
  - Related files/docs: validation output for Story 1.1; `docs/logs/2026-06-governance-domain-types/v0.0.1-governance-domain-types/verification.md`.

- [ ] Complete Story 2.2 inbox behavior that Story 2.3 still depends on.
  - Context: During Story 2.3 implementation on 2026-06-14, the proposal detail pane and governance actions were added on top of the existing Evolution inbox shell, but Story 2.2 requirements remain incomplete: row metadata density, filters, keyboard navigation, and batch actions are not fully implemented in the current tree.
  - Expected behavior: Evolution inbox should satisfy Story 2.2 before Epic 2 GUI acceptance relies on Story 2.3 detail flow.
  - Related files/docs: `_bmad-output/implementation-artifacts/2-2-build-proposal-inbox-list-and-filters.md`, `agent-diva-gui/src/components/EvolutionView.vue`.

## Done

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
