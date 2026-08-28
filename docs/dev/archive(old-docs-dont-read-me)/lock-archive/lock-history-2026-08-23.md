# LOCK.md History Archive (pre-2026-08-23)

This file archives historical lock entries and handoff notes from LOCK.md.
Archive date: 2026-08-23.

---

## Historical Active Locks (RELEASED)

### 2026-08-23 and earlier

- `hide installed skills from evolution view` — **RELEASED 2026-08-21T15:40:00+08:00** by `QoderCN`. Evolution view now lists only evolution-managed skills (`evolution_managed === true` filter in `EvolutionView.loadSkills`, selection cleanup on the filtered list, empty-state text updated); marketplace/manual installs and builtins stay managed in Settings -> Installed. Commit `9859d9c5` (GUI + tests + logs + TODOLIST). Gates: GUI vitest 480/480 (new filter case) + vue-tsc green; no Rust changes. Desktop smoke folded into `SKILL-MARKETPLACE-DESKTOP-SMOKE` (v0.2.2 acceptance). Not pushed.

- `installed skills tab: delete marketplace/manual skills, evolution-managed hint` — **RELEASED 2026-08-21T14:55:00+08:00** by `QoderCN`. Fixed the misread ":evolution 管理" label: removed the hardcoded placeholder badge from `InstalledSkillsTab.vue`; marketplace/manual skills now have a real delete action (confirm → `DELETE /api/skills/:slug` → toast → refresh), evolution-managed skills show a localized hint. Ownership derived from accepted proposals via new `SkillSummary.evolution_managed` + lock-free `accepted_slugs()` (no disk format change). Commit `17409ce7` (core + manager + Tauri/GUI + i18n + tests + logs + TODOLIST). Gates: core skill_home 7/7, GUI vitest 479/479 + vue-tsc, `just fmt-check`/`just check` green, workspace tests green excluding CLI/GUI binaries locked by the user's running processes. Desktop smoke folded into `SKILL-MARKETPLACE-DESKTOP-SMOKE` (v0.2.1 acceptance). Not pushed.

- `marketplace featured leaderboard via offline YAML snapshot (python script + gateway embed)` — **RELEASED 2026-08-21T13:35:00+08:00** by `QoderCN`. Offline featured leaderboard per user decision: Python fetch script (`agent-diva-manager/scripts/fetch_marketplace_featured.py`, skills.sh v1 API with Vercel OIDC token, unauthenticated homepage-leaderboard fallback) writes `agent-diva-manager/data/marketplace_featured.yaml` (first run: 100 real entries, web:leaderboard); gateway embeds the snapshot (`include_str!` + serde_yaml) and serves `GET /api/skills/marketplace/featured`; GUI empty-search state shows Top 100 with snapshot date. Commits `2a84eb18` (manager + script + snapshot + e2e), `b7ff9b4c` (Tauri command + MarketplaceTab + i18n + tests), `71b146ed` (iteration logs + TODOLIST). Gates: manager lib + e2e green, GUI vitest 474/474 + vue-tsc build, `just fmt-check`/`just check` green, workspace tests green excluding CLI/GUI binaries locked by the user's running processes. Desktop smoke pending (`SKILL-MARKETPLACE-DESKTOP-SMOKE`); v1 token path unvalidated (`SKILL-MARKETPLACE-V1-TOKEN-VERIFY`). Not pushed.

- `make Settings skill marketplace usable via skills.sh adapter` — **RELEASED 2026-08-20T00:05:00+08:00** by `QoderCN` (takeover from Grok). Real skills.sh directory wired end-to-end: manager `MarketplaceClient` + `/api/skills/marketplace/search|install` routes + snapshot install guards, Tauri `search_marketplace_skills`/`install_marketplace_skill`, GUI search-as-you-type `MarketplaceTab`. Commits `3f1b61ad` (manager) + `7a7d9b7b` (GUI) + `adac0936` (docs). Gates: manager 103 lib + e2e green, GUI vitest 472/472 + vue-tsc, just check (clippy -D) green. Desktop smoke pending (`SKILL-MARKETPLACE-DESKTOP-SMOKE`). Not pushed.

- `repeat Windows GUI release build + NSIS/MSI packaging (user-requested recompile)` — **RELEASED 2026-08-19T22:58:00+08:00** by `QoderCN`; full pipeline green on HEAD `79b30721` (includes QQ allowlist/group-reject and GUI channel editor fixes). Incremental rebuild of channels/gui/manager/cli + vite frontend, both installers regenerated 22:57: NSIS 139M, MSI 149M; `agent-diva-gui.exe` 166M, `agent-diva.exe` 41M. No source changes. Smoke test ready.

- `restore empty allow_from as unrestricted; QQ rejects group events` — **RELEASED 2026-08-19T21:35:00+08:00** by `Grok`. Commits `7ba9e048` (empty allow_from = allow all), `7611fbc7` (QQ reject group/guild events), `79b30721` (docs). Gates: channels lib clippy -D + 84/84. Restart gateway. Not pushed.

- `fix GUI channels settings card-edit blank / layout / YAML-only` — **RELEASED 2026-08-19T21:00:00+08:00** by `Grok`. Commits `c1aaaf5f` (shared ChannelEditorForm, schema-complete fields, hydrated wizard, card-edit stays on cards, list inline forms, layout) and `e4ab95d2` (iteration logs + TODOLIST). Gates: GUI vitest 468/468 + vue-tsc/vite build. Desktop smoke pending (`CHANNELS-EDITOR-DESKTOP-SMOKE`). Not pushed.

- `Windows GUI release build + NSIS/MSI packaging for smoke test` — **RELEASED 2026-08-19T12:55:00+08:00** by `QoderCN`; full `scripts/package-windows-gui.ps1` pipeline green (cargo release cli/service/gui in 10m 52s, bundle:prepare, NSIS + MSI). Artifacts: `target\release\bundle\nsis\Agent Diva_0.5.0_x64-setup.exe` (139M), `target\release\bundle\msi\Agent Diva_0.5.0_x64_en-US.msi` (149M), `target\release\agent-diva-gui.exe` (166M), `target\release\agent-diva.exe` (41M). No source changes. Smoke test pending with user.

- `show clean-mode process and tool labels as instant snapshots` — **RELEASED 2026-08-18T10:14:21+08:00** by `Codex`; clean-mode process snapshot rendering, tool label localization, focused GUI tests, and follow-up iteration log. Commit `3961b94`. Not pushed.

- `collapse clean-mode process messages into one thinking bubble` — **RELEASED 2026-08-18T09:52:15+08:00** by `Codex`; only clean-mode process aggregation, reasoning suppression, focused GUI tests, and follow-up iteration log. Commit `41142851`. Gates: GUI 64 files/457 tests + build, Rust fmt/check, and GUI HTTP smoke. Not pushed.

- `dismantle retired channel adapters behind opt-in Cargo features` — **RELEASED 2026-08-18T01:45:00+08:00** by `QoderCN`. slack/whatsapp/matrix/irc/mattermost/nextcloud_talk modules, re-exports, and all ChannelManager injection points are now behind opt-in `channel-*` features (default-off); slack-morphism optional; whatsapp integration test feature-gated; sources retained. Commits `e52cc361` (code + regression test) and the follow-up docs commit on `agent-diva-pro`. Gates: fmt-check, clippy -D both feature sets, channels 80/80 default & 133/133+integrations all-features, just check, workspace tests green. Environment note: C: was 100% full (target/debug 381 GB); user-approved PDB cleanup freed ~100 GB. Not pushed.

- `retire unverified channels from GUI (backend retained)` — **RELEASED 2026-08-18T10:05:00+08:00** by `QoderCN`. User decision: slack/whatsapp/nextcloud_talk/mattermost/matrix/irc removed from the channels settings page; backend adapters/schema kept as historical code. Commits `56802e12` (GUI: RETIRED_CHANNELS filter, wizard/platform/icon registry pruning, 4 icon components deleted, tests) and `ff72d1f0` (iteration logs + TODOLIST). Gates: GUI vitest 451/451 + vue-tsc build. Desktop smoke pending (TODOLIST CHANNELS-RETIRE-DESKTOP-SMOKE). Not pushed.

- `fix GUI channels settings page (contract/toggle/wizard/matrix + gateway channel updates)` — **RELEASED 2026-08-18T00:25:00+08:00** by `QoderCN`. Commits `0c244364` (GUI contract/toggle/wizard/matrix + 13 vitest), `cce52a4e` (manager channel routing + disk fallback + unit tests), `c2ecd263` (docs/TODOLIST). Gates: GUI vitest 448/448 + vue-tsc build, manager fmt/clippy/test, workspace fmt-check/check and tests (excluding CLI/GUI binaries locked by the user's running gateway/desktop processes). Restart gateway + GUI to pick up the fix; manual acceptance in docs/logs acceptance.md. Not pushed.

- `unify GUI design-token theme system (colors/typography/spacing)` — **RELEASED 2026-08-17T23:10:00+08:00** by `QoderCN`. Phase 1 complete: design-token base + Tailwind wiring (`edbf0134`), useTheme governance, `.theme-*` override retirement (`e8e4b337`), semantic-color tokenization in ChatView/ConversationSidebar/McpManagementCard (`785b954b`). vitest 435/435 and `npm run build` (vue-tsc) green. Phase 2 backlog in TODOLIST (GUI-STYLE-UNIFICATION-PHASE-2). Not pushed.

- `fix GUI chat stick-to-bottom behavior while reading history` — **RELEASED 2026-08-17T21:23:00+08:00** by `Codex`. ChatView now preserves upward reading position and resumes bottom following on send/session switch. Commit `a0255a44`; not pushed.

- `recover empty post-tool replies from upstream finish/usage signals` — **RELEASED 2026-08-17T12:55:00+08:00** by `Grok`. Empty post-tool completions classified from finish_reason/usage/window/assembly; one summary-only retry. Commit `1fff591f`. Restart gateway. Not pushed.

- `unblock gateway boot on illegal approval expire replay` — **RELEASED 2026-08-17T04:50:00+08:00** by `Grok`. Duplicate `expired` events replay as Expired; `expire()` is idempotent; `states_page` skips unreplayable aggregates. Commit `c0a16777`. Restart gateway. Not pushed.

- `remove runtime PII redaction` — **RELEASED 2026-08-17T04:15:00+08:00** by `Grok`. Default PII hiding is off; tool results, inbound messages, artifacts, and AutoDream summaries keep original text. Commit `5ba63e1b`. Restart gateway. Not pushed.

- `preserve memory record ids through PII redaction` — **RELEASED 2026-08-17T03:40:00+08:00** by `Grok`. Phone patterns now use word boundaries; BML / checkpoint ids are protected spans in `redact_pii`. Commit `6c752054`. Restart gateway to pick up the fix. Not pushed.

- `fix first-run Persona setup unknown Laputa API error` — **RELEASED 2026-08-17T02:35:00+08:00** by `Grok`. Manager Persona routes now wrap `{ "status": "ok", "persona": ... }`; Tauri decoder accepts that envelope and the older `{ "status": PersonaStatusView }` object. Focused Manager/Tauri tests, fmt-check, and clippy on the two crates passed. Not pushed.

- `Correct Laputa architecture gate wording` — **RELEASED 2026-08-15T06:25:00+08:00** by `Grok`; R1-hold wording corrected. Docs only. No S1.

- `Schedule GUI in TODOLIST` — **RELEASED 2026-08-15T06:05:00+08:00** by `Grok`; UI-S2/S3/S4 scheduled. Docs only.

- `Unify Recap + TODOLIST` — **RELEASED 2026-08-15T05:50:00+08:00** by `Grok`; Recap semantics unified; TODOLIST refreshed. Docs only.

- `Create protection branch` — **RELEASED 2026-08-15T05:20:00+08:00** by `Grok`; protect/cognitive-pre-clean-break-20260815 @ 2aab18cc. Not pushed.

- `Approve D4 and freeze recap` — **RELEASED 2026-08-15T05:00:00+08:00** by `Grok`; D4 approved; S8 per-turn recap. No production code. No backup branch.

- `Approve D3 and draft D4` — **RELEASED 2026-08-15T04:20:00+08:00** by `Grok`; D3 approved; D4 draft + review fixes. No production code. No backup branch.

- `Draft D3 Evolution/Skill` — **RELEASED 2026-08-15T03:00:00+08:00** by `Grok`; D3 design draft + review fixes. No production code.

- `Record D2 approval` — **RELEASED 2026-08-15T01:30:00+08:00** by `Grok`; D2 approved. No production code.

- `Approve D1 and draft D2` — **RELEASED 2026-08-15T01:15:00+08:00** by `Grok`; D1 approved; D2 draft + review fixes. No production code.

- `Draft D1 Persona architecture` — **RELEASED 2026-08-15T00:10:00+08:00** by `Grok`; D1 design draft + review fixes. No production code.

- `Freeze D0 A/B/C` — **RELEASED 2026-08-14T23:05:00+08:00** by `Grok`; P22 + S1 revision + D7. No production code.

- `Draft D0 domain-authority ADR` — **RELEASED 2026-08-14T22:10:00+08:00** by `Grok`; D0 design draft + architect review fixes. No production code.

- `Freeze S9/P21 MEMRULES` — **RELEASED 2026-08-14T20:25:00+08:00** by `Grok`; MEMRULES out of Laputa, GA-style write-time inject. No production code.

- `Clarify AutoDream Laputa allow/deny matrix` — **RELEASED 2026-08-14T10:35:00+08:00** by `Grok`; P19 allow/deny matrix. No production code.

- `Reject AutoDream persona writes; schedule diagnostic` — **RELEASED 2026-08-14T00:20:00+08:00** by `Grok`; P19 + Evolution D5 + TODOLIST diagnostic. No AutoDream production code.

- `P18 v1-closed roster, team-extensible kinds` — **RELEASED 2026-08-13T22:50:00+08:00** by `Grok`; P18 recorded. No production code.

- `Record IDENTITY body + DARK.MD booths` — **RELEASED 2026-08-13T22:25:00+08:00** by `Grok`; P17 DARK booths + IDENTITY body recorded. No production code.

- `Persona authority Markdown roster decision revision` — **RELEASED 2026-08-13T21:45:00+08:00` by `Grok`; decision record P1/P13–P16 + current-boundary sync. No production code.

- `COGNITIVE-R4-CLEAN-BREAK-SAFETY research package` — **RELEASED 2026-08-13T20:55:00+08:00** by `Grok`; documentation-only R4 research package delivered under `docs/research/cognitive-r4-clean-break-safety-2026-08/` plus logs/index. No production source/config/build changes.

- `COGNITIVE-R3-PERSONA-WORKSPACE research package` — **RELEASED 2026-08-13T20:15:00+08:00** by `Grok`; documentation-only R3 research package delivered under `docs/research/cognitive-r3-persona-workspace-2026-08/` plus logs/index. No production source/config/build changes.

- `COGNITIVE-R2-STM-CONTEXT research package` — **RELEASED 2026-08-13T19:05:00+08:00** by `Grok`; documentation-only R2 research package delivered under `docs/research/cognitive-r2-stm-context-2026-08/` plus logs/index. No production source/config/build changes.

- `COGNITIVE-R0-CURRENT-STATE research package` — **RELEASED 2026-08-13T17:20:00+08:00** by `Grok`; documentation-only R0 current-state inventory delivered under `docs/research/cognitive-r0-current-state-2026-08/` plus logs/index. No production source/config/build changes.

- `COGNITIVE-R1-GENERICAGENT-EVOLUTION research package` — **RELEASED 2026-08-13T13:30:00+08:00** by `Grok`; documentation-only R1 research package delivered under `docs/research/cognitive-r1-genericagent-evolution-2026-08/` plus logs/index. No production source/config/build changes.

- `Collect remaining archive-root legacy files and refresh archive package` — **RELEASED 2026-08-13T10:07:16+08:00** by `Codex`; documentation-only cleanup of seven legacy files left beside the archive index, plus ZIP/manifest refresh is complete. No source, config, or build files were changed; tests were not run per user instruction.

- `Consolidate pre-existing archive batches and compress legacy corpus` — **RELEASED 2026-08-13T10:05:19+08:00** by `Codex`; documentation-only consolidation of already archived historical batches under `docs/dev/archive(old-docs-dont-read-me)/`; no source, config, or build files were changed, and tests were not run per user instruction.

- `Reorganize complete docs corpus and pre-August logs` — **RELEASED 2026-08-13T09:53:49+08:00** by `Codex`; documentation-only cleanup of the complete `docs/` corpus, current architecture/research entrypoints, retained decision summaries, legacy archive folders, manifests, and ZIP packages is complete. Production source, config, and build files were out of scope; tests were not run per user instruction.

- `Cognitive workspace reset master EPIC` — **RELEASED 2026-08-13T02:47:47+08:00** by `Codex`; documentation-only orchestration committed in `0d2acb60`. Today's product decisions are consolidated into R0-R4 research, D0-D4 architecture, and implementation gates; no production code or target architecture design was created.

- `STM cross-session clean-break decision` — **RELEASED 2026-08-13T02:35:20+08:00** by `Codex`; documentation-only decision committed in `503c836d`. Freezes memory/persona/STM boundaries and GUI ownership while leaving storage, automation and context assembly to research.

- `Laputa first-run initialization decision` — **RELEASED 2026-08-13T02:26:10+08:00** by `Codex`; documentation-only decision committed in `544d1944`. Records five-authority first-run initialization, direct atomic write, absence-only trigger, and lifetime Persona/WORLD history.

- `Persona workspace interaction decision` — **RELEASED 2026-08-13T01:45:30+08:00** by `Codex`; documentation-only decision committed in `6ff3744b`. Records current-document, pending-change and history states, plus the boundary between content review and security approval.

- `Persona Markdown authority decision` — **RELEASED 2026-08-13T01:17:17+08:00** by `Codex`; Markdown authority, source/preview/diff workspace, direct-save boundary and zero-compatibility deletion direction recorded in `262a8297`.

- `TODOLIST stale-record archive` — **RELEASED 2026-08-13T01:01:09+08:00** by `Codex`; root backlog reduced to 30 active items, full pre-cleanup snapshot and archive index recorded in `77bac367`; no product code changes.

- `Governance / Persona / Evolution recovery` — **RELEASED 2026-08-12T23:24:30+08:00** by `Codex`; unified Memory approval authority, AutoDream proposal-boundary registration, Persona/Evolution resilient GUI state, full isolated `just ci`, GUI tests/build, and real-workspace read-only smoke complete in `78e2bcf5`, `d6f82ea3`, `ab4705e4`, and `df21bd17`. Gateway PID 27624 is running the current branch; visual/state-changing M3 acceptance remains with user.

- `CTX-C5b canonical checkpoint implementation` — **RELEASED 2026-08-11T18:30+08:00** by `Codex`; canonical checkpoint clean break, focused/full validation, TODOLIST and iteration logs complete in commits `19a5bfec`, `c6e3ace4`, and `12bb92ce`; not pushed.

- `CTX-C5a final wire cache prefix implementation` — **RELEASED 2026-08-11T18:05+08:00** by `Codex`; provider final-wire snapshot, agent observer clean break, focused/full gates, TODOLIST and iteration logs complete.

- `CTX-C5e automatic deferred tool activation planning` — **RELEASED 2026-08-11T17:10+08:00** by `Codex`; technical plan, TODOLIST and iteration logs complete; no production-code changes.

- `CTX-C5 plan formatting follow-up` — **RELEASED 2026-08-11T16:30+08:00** by `Codex`; trailing whitespace removed in commit `1f91c0c7`.

- `CTX-C5 lightweight context convergence plan and clean-break policy` — **RELEASED 2026-08-11T16:25+08:00** by `Codex`; technical plan, research index, TODOLIST and iteration logs complete; no production-code changes.

- `CTX-C4 deferred tool discovery, same-turn mount, and recall verification` — **RELEASED 2026-08-11T10:20+08:00** by `Codex`; implementation and final verification complete in commits `66fb1ed4`, `8b79fcac`, `2747462d`, `646b50c0`, and `07bc4ee0`; not pushed.

- None. `merge feat/gmh41-budget-closure + fix/small-fixes-batch` — owner `QoderCN` — **RELEASED 2026-08-11T09:39+08:00**; merges `8c1f5b39` (GMH batch) + `c05ce32a` (small-fixes) on `agent-diva-pro`, marker cleanup `a1a60355`, TODOLIST bookkeeping `5245e92f`; gates green except the 6 pre-existing CLI wiremock 502 cases; not pushed.

- None. `CTX-C3 tool-result artifact references and microcompact` released `2026-08-11T09:18+08:00`; see Handoff Notes.

- `small-fixes-batch (gateway port config + clippy lint batch)` — owner `QoderCN` — **RELEASED 2026-08-11T09:40+08:00; MERGED 2026-08-11 via `c05ce32a`**（3 commits：`83b87787` / `1aba27a3` / `2ccb05e6`）.

---

## Historical Handoff Notes (pre-2026-08-23)

- `2026-08-21T19:30:00+08:00`: Released branch rename + slim main rebuild (QoderCN). 1) Local `agent-diva-pro` renamed to `dev`; pushed `origin/dev` and deleted `origin/agent-diva-pro` (user-requested rename). 2) `main` rebuilt from dev content without force push: `-s ours` merge of local main history + origin/main README history (`af1fc999`, `e420b9a5`), then commit `0fd005a1` strips `docs/` (4007 files) keeping only `docs/resources/diva.png`; README doc/contributing links repointed to the dev branch on GitHub. Pushed `origin/main` (fast-forward) and synced `origin/dev` (`88607742..af1fc999`). Backup branch `oldmain` (pre-rebuild main @ `6ebe4b75`) pushed to `origin/oldmain` per user request. Working tree back on `dev` with full docs.

- `2026-08-21T18:45:00+08:00`: Released EN/ZH README refresh (QoderCN). Both root READMEs rewritten to the 2026-08 project state: 17-crate workspace layout, BML/Laputa/AutoDream memory & persona section, skills.sh marketplace + Evolution, sandbox/HITL approvals, verified CLI command list, corrected skills paths (`{config_dir}/skills`), MSRV 1.80, fixed all dead doc links. Fact-checked against Cargo.toml members, the built CLI binary, providers.yaml, skills.rs, and existing doc paths; iteration logs in `docs/logs/2026-08-readme-refresh/v0.9.9-readme-current-state/`. Docs-only commit on `agent-diva-pro`. Not pushed.

- `2026-08-21T18:15:00+08:00`: Pushed `agent-diva-pro` to origin per user request (QoderCN). `d05d22c3..1f86d8ad` (22 commits) pushed to `origin/agent-diva-pro`, including the marketplace adapter/featured snapshot, skills tab cleanup, autodream progress persistence, clean-mode GUI fixes, and the `chore: bump workspace global version to 0.9.9` commit (`1f86d8ad`). Working tree only carries the LOCK.md edit and the pre-existing `.vibeyardignore` deletion.

- `2026-08-21T13:35:00+08:00`: Released featured leaderboard snapshot (QoderCN). User decision: offline YAML snapshot via Python script instead of runtime scraping/token calls. Three commits on `agent-diva-pro`: `2a84eb18` manager (script + data YAML + `FeaturedSnapshot` embed + `/api/skills/marketplace/featured` route + e2e), `b7ff9b4c` GUI (Tauri command + empty-state Top 100 + i18n + vitest), `71b146ed` docs/TODOLIST. Gates green: manager marketplace lib tests + e2e 2/2, GUI 474/474 + vue-tsc build, `just fmt-check`/`just check`, workspace tests (CLI/GUI binaries excluded — locked by user's running processes). First snapshot = 100 real entries from the unauthenticated homepage leaderboard; the official v1 API path (`AGENT_DIVA_SKILLS_MARKETPLACE_TOKEN`) awaits a token per TODOLIST `SKILL-MARKETPLACE-V1-TOKEN-VERIFY`. Rebuild + restart gateway/GUI, then follow `docs/logs/2026-08-skill-marketplace/v0.2.0-featured-leaderboard-snapshot/acceptance.md`. Not pushed.

- `2026-08-20T00:05:00+08:00`: Released skills.sh marketplace adapter (QoderCN, takeover from Grok complete). Three commits on `agent-diva-pro`: `3f1b61ad` manager adapter + routes + snapshot install guards + e2e, `7a7d9b7b` Tauri commands + GUI MarketplaceTab rewrite + i18n + tests, `adac0936` iteration logs + TODOLIST. Gates green: manager lib 103 + `skill_marketplace_e2e` via wiremock, GUI vitest 472/472 + vue-tsc build, cargo fmt + `just check` (clippy -D). Rebuild + restart gateway and GUI to pick up the feature, then follow `docs/logs/2026-08-skill-marketplace/v0.1.0-skills-sh-marketplace-adapter/acceptance.md`. Not pushed.

- `2026-08-19T23:37:00+08:00`: Took over stale marketplace lock (QoderCN). Grok claimed the skills.sh marketplace scope at 23:20 but ran out of credits before landing any change (verified: working tree only carries the LOCK.md edit and a pre-existing `.vibeyardignore` deletion; no marketplace code, no `docs/logs/2026-08-skill-marketplace/`, no stash). Marked stale per rule 6; QoderCN re-claims the same scope and starts implementation (manager adapter + routes, Tauri commands, GUI wiring). Protocol confirmed live: `GET https://skills.sh/api/search?q=&limit=20` (unauthenticated JSON) and `GET https://skills.sh/api/download/{owner}/{repo}/{slug}` returning `{files:[{path,contents}],hash}` — mirrors `npx skills` CLI.

- `2026-08-19T22:58:00+08:00`: Released repeat packaging build (QoderCN, user-requested recompile). `scripts/package-windows-gui.ps1` full pipeline green on `agent-diva-pro` @ `79b30721`, now including QQ allowlist/group reject (`7ba9e048`/`7611fbc7`) and GUI channel editor restore (`c1aaaf5f`). Incremental rebuild (~1h incl. Tauri re-bundle); installers regenerated 22:57 — NSIS `target\release\bundle\nsis\Agent Diva_0.5.0_x64-setup.exe` (139M) and MSI `target\release\bundle\msi\Agent Diva_0.5.0_x64_en-US.msi` (149M); binaries `agent-diva.exe` (41M) / `agent-diva-gui.exe` (166M). No source changes; nothing to commit. Smoke test can start now.

- `2026-08-19T21:35:00+08:00`: Released QQ allowlist + group reject (Grok). Empty `allow_from` is unrestricted again; QQ group/guild events are logged and dropped. Commits `7ba9e048` + `7611fbc7` + `79b30721`. Restart gateway. Not pushed.

- `2026-08-19T21:00:00+08:00`: Released GUI channel editor restore (Grok). Card edit opens a prefilled wizard; list view uses the same schema form for all seven visible channels; YAML-only copy removed. Commits `c1aaaf5f` + `e4ab95d2`. Restart GUI for smoke. Not pushed.

- `2026-08-18T06:30:00+08:00`: Released homepage plan-approval orchestration fix (Codex). Commit `9fe502eb` routes unified plan allows through the canonical approve/continue execution chain, synchronizes the executing runtime, and exposes retry state when stream startup fails. GUI build and 452 tests, workspace clippy, and isolated-target workspace tests passed; real desktop smoke remains pending per the iteration verification log.

- `2026-08-18T10:05:00+08:00`: Released GUI retirement of unverified channels (QoderCN). Per user decision 2026-08-18, slack/whatsapp/nextcloud_talk/mattermost/matrix/irc are hidden from card view, list sidebar, and wizard; backend code untouched (historical retention). Commits `56802e12` + `ff72d1f0` on `agent-diva-pro`. GUI vitest 451/451 and vue-tsc production build green; no Rust changes. Restart the GUI and follow `docs/logs/2026-08-channels-settings-fix/v0.1.1-retire-unverified-channels-gui/acceptance.md` for the manual smoke. Not pushed.

- `2026-08-18T12:10:00+08:00`: Pushed `agent-diva-pro` to origin per user request (QoderCN). `ee863cc0..d05d22c3` (195 commits) pushed to `origin/agent-diva-pro`, including all previously "Not pushed" batches (channels settings fix, GUI design tokens, GUI/backend channel retirement, feature-gated retired channel adapters, iteration logs). Working tree only carries this LOCK.md change.

- `2026-08-18T01:45:00+08:00`: Released retired-channel backend dismantling (QoderCN). Per user decision 2026-08-18, the six unverified channels are compiled only under opt-in Cargo features `channel-slack` (pulls slack-morphism), `channel-whatsapp`, `channel-matrix`, `channel-irc`, `channel-mattermost`, `channel-nextcloud-talk`; default builds exclude their adapters and all ChannelManager injection points, so they drop out of the routable channel list automatically. Sources retained; re-enable via `cargo build -p agent-diva-channels --features channel-*`. Commits `e52cc361` (code) + docs commit on `agent-diva-pro`; iteration logs in `docs/logs/2026-08-channels-backend-retirement/v0.2.0-retired-channel-feature-gates/`. Environment: C: drive had been 100% full (target/debug 381 GB); with user approval, deleted all target/debug PDBs (~99 GB) to unblock builds — no correctness impact, incremental builds regenerate symbols. Restart gateway to pick up the new default build. Not pushed.

- `2026-08-18T00:25:00+08:00`: Released channels settings page repair (QoderCN). Three commits on `agent-diva-pro`: `0c244364` GUI (raw config map normalized into named/enabled cards, card-toggle persistence for the clicked channel, wizard edit prefill + merge-on-save + bool/channels coercion, matrix icon/name registration, wizard grid scoped to CHANNEL_PLATFORMS), `cce52a4e` manager (neuro-link/irc/mattermost/nextcloud_talk update routing, unknown-channel error, get_channels disk fallback), `c2ecd263` docs/TODOLIST. Gates: GUI vitest 448/448 + vue-tsc build; manager fmt/clippy -D/test; workspace fmt-check/check/test green except CLI/GUI crates excluded because the user's running agent-diva.exe / agent-diva-gui.exe lock the debug binaries (rerun after next restart; CLI has 6 pre-existing CLI-WIREMOCK-502-PREEXISTING cases). Restart the gateway and GUI for the fix to take effect; manual acceptance checklist in `docs/logs/2026-08-channels-settings-fix/v0.1.0-channels-settings-repair/acceptance.md`. Not pushed.

- `2026-08-17T23:10:00+08:00`: Released GUI design-token unification phase 1 (QoderCN). Four commits on `agent-diva-pro`: `edbf0134` token base + Tailwind wiring, useTheme centralization, `e8e4b337` `.theme-*` override retirement (+160/−446), `785b954b` semantic color tokenization. vitest 435/435 + vue-tsc/build green; four-theme manual smoke recorded in `docs/logs/2026-08-gui-style-unification/v0.1.0-design-tokens/acceptance.md`. Phase 2 (remaining ~31 files, pet overlays, WelcomeWizard palette, tk-* font-size migration) in TODOLIST as GUI-STYLE-UNIFICATION-PHASE-2. Not pushed.

- `2026-08-17T21:23:00+08:00`: Released after the GUI chat scroll fix. ChatView 11/11, GUI 430/430, GUI build/Tauri check, workspace fmt/check/test, and user manual long-message smoke passed. Commit `a0255a44`; not pushed.

- `2026-08-17T12:55:00+08:00`: Released after empty post-tool summary recovery. Successful empty completions after tools are classified (input pressure / output truncated / empty stop) before the Chinese fallback. Restart gateway/agent. Commit `1fff591f`; not pushed.

- `2026-08-17T04:50:00+08:00`: Released after gateway boot ledger fix. Field `governance.db` had `088420b7-…` as requested/allowed/expired/expired. Commit `c0a16777`. Restart `just diva-gate`. Not pushed.

- `2026-08-17T04:15:00+08:00`: Released after removing runtime PII hiding per user decision. Commit `5ba63e1b`. Restart gateway/agent before re-smoke. CLI config-show and GUI console secret key redaction left in place.

- `2026-08-17T03:40:00+08:00`: Released after memory-id PII false positive fix. `memory-{micros}-{digest}` no longer loses its timestamp to `[REDACTED:Phone]`. Restart gateway/agent before re-smoking CRUD. Commit `6c752054`; not pushed.

- `2026-08-17T02:35:00+08:00`: Released after first-run Persona setup envelope fix. Opening 「建立 Persona」 treated `{ status: PersonaStatusView }` as an error and showed `unknown Laputa API error`. New Manager envelope is `{ status: "ok", persona: view }`; Tauri also accepts the old object envelope. Restart GUI (and gateway if you want the new envelope). Not pushed.

- `2026-08-14T00:30:00+08:00`: Released I1-S1 after stopping Frozen Core `null` section seeds and `WORLD.MD` pre-seeding. Laputa/Manager tests, `just fmt-check`, `just check`, and full `just test` passed. Code commit `559faa8d`; S2/S3/S4 remain separately gated.

- `2026-08-15T06:25:00+08:00`: Corrected stale R1-hold gate in laputa architecture pack.

- `2026-08-15T06:05:00+08:00`: GUI scheduled in TODOLIST as UI-S2/S3/S4.

- `2026-08-15T05:50:00+08:00`: Recap wording unified; TODOLIST caught up.

- `2026-08-15T05:20:00+08:00`: Protection branch created locally at 2aab18cc. Not pushed. S1 not started.

- `2026-08-15T05:00:00+08:00`: Released after D4 approval and S8 Recap revision. Next: user says 切 then S1. Docs only.

- `2026-08-15T04:20:00+08:00`: Released after D3 approval and D4 delivery draft. Backup only when user says 切. Docs only.

- `2026-08-15T03:00:00+08:00`: Released after D3 Evolution/Skill design draft. Awaiting user review. Docs only.

- `2026-08-15T01:30:00+08:00`: Released after recording D2 approval. Docs only.

- `2026-08-15T01:15:00+08:00`: Released after D1 user approval and D2 design draft. D2 awaits review. Docs only.

- `2026-08-15T00:10:00+08:00`: Released after D1 Persona design draft. Path `{config_dir}/persona/`. Awaiting user review. Docs only.

- `2026-08-14T23:05:00+08:00`: Released after freezing D0 A/B/C as P22 / S1 revision / D7. One-machine companion; BML follows persona; distill always Evolution review. Docs only.

- `2026-08-14T22:10:00+08:00`: Released after D0 design draft. Open questions A/B/C in domain-authority.md §11. Docs only.

- `2026-08-14T20:25:00+08:00`: Released after freezing S9/P21 MEMRULES. Not a persona file; `{config_dir}/memory/MEMRULES.MD`; Memory settings editable; context like GA L0. Docs only. No production code.

- `2026-08-14T10:35:00+08:00`: Released after correcting P19: AutoDream is a Laputa proposal generator with an allow/deny matrix, not a total ban.

- `2026-08-14T00:20:00+08:00`: Released after recording AutoDream-must-not-write persona (P19/D5) and diagnostic backlog. Source inventory confirmed two propose-only paths (agent tool + AutoDream emit). Docs only.

- `2026-08-13T22:50:00+08:00`: Released after P18 (v1 closed set, team-extensible kind registry, users cannot add authority kinds). Docs only.

- `2026-08-13T22:25:00+08:00`: Released after recording IDENTITY-includes-body and DARK.MD FEAR/SHADOW booths. Docs only.

- `2026-08-13T21:45:00+08:00`: Released after writing authority roster into the Persona decision record and syncing current product-facing entries. No production source, config, or build changes.

- `2026-08-13T21:20:00+08:00`: Claimed Persona authority Markdown roster revision (REDLINE/USER/DREAM, uppercase names, Frozen Core 10-char DREAM).

- `2026-08-13T20:55:00+08:00`: Released after COGNITIVE-R4-CLEAN-BREAK-SAFETY documentation-only research package. Deliverables under `docs/research/cognitive-r4-clean-break-safety-2026-08/` plus EPIC/index/TODOLIST/logs. No production source, config, or build changes; `just ci` not run (docs-only). No protect/* branch created. Architecture design remains blocked pending user Research Gate on R0–R4.

- `2026-08-13T20:25:00+08:00`: Claimed COGNITIVE-R4-CLEAN-BREAK-SAFETY documentation-only research package. Scope is research docs, EPIC/index, R0/R3 open-gap lines, TODOLIST R4/EPIC status, and iteration logs. No production source, config, or build changes.

- `2026-08-13T20:15:00+08:00`: Released after COGNITIVE-R3-PERSONA-WORKSPACE documentation-only research package. Deliverables under `docs/research/cognitive-r3-persona-workspace-2026-08/` plus EPIC/index/TODOLIST/logs. No production source, config, or build changes; `just ci` not run (docs-only, same as R0/R1/R2). Architecture design remains blocked.

- `2026-08-13T19:20:00+08:00`: Claimed COGNITIVE-R3-PERSONA-WORKSPACE documentation-only research package. Scope is research docs, EPIC/index, R0 open-gap line, TODOLIST R3/EPIC status, and iteration logs. No production source, config, or build changes.

- `2026-08-13T19:05:00+08:00`: Released after COGNITIVE-R2-STM-CONTEXT documentation-only research package. Deliverables under `docs/research/cognitive-r2-stm-context-2026-08/` plus EPIC/index/TODOLIST/logs. No production source, config, or build changes; `just ci` not run (docs-only, same as R0/R1). Architecture design remains blocked.

- `2026-08-12T12:55:00+08:00`: Released code-review residual scope. Runtime commits `b4c10047`, `bb92204d`, `bdef0f87`; approval `e3f6660d`; context/GUI `d3dc56ed`; docs/TODOLIST `93e07f75`. `just fmt-check`, `just check`, focused Rust libs, GUI `npm test` (455/455) and `npm run build` passed. Final `just ci` also passed, including workspace tests, feature gates and BML clean-break gate. M3 real desktop smoke remains open in TODOLIST.

- `2026-08-12T11:20:00+08:00`: Merged `feat/m3-hitl-closure` into `agent-diva-pro` as `131d2dc5` (clean ort; S1–S5 three-mode HITL). TODOLIST backlog from review committed `586147cc`; post-merge TODOLIST bookkeeping follows. Sandbox lib tests 127/127. Not pushed. Residual: dual-channel stream, human M3 smoke, Track A doc/UX gaps still open in TODOLIST.

- `2026-08-12T10:45:00+08:00`: Released line-review cache+HITL. Review log at `docs/logs/2026-08-code-review-cache-hitl/v0.0.1-line-review/`. Critical: M3 HITL S1–S5 lives only on `feat/m3-hitl-closure` (not merged into `agent-diva-pro`); HEAD Guardian still merges OnRequest|UnlessTrusted and ShellTool has no mode-driven Guardian. Track A C1–C5 is on HEAD.

- `2026-08-12T01:10:30+08:00`: Released G2D automated E2E coverage. Added `agent-diva-manager/tests/autodream_laputa_e2e.rs` with 6 Manager HTTP vertical scenarios and the four iteration log documents; TODOLIST records the automated gate and a pre-existing Windows stale-lock timing flake. Commit `b4d2a84b`; `just ci` passed; no push. G2D+ real desktop acceptance remains pending.

- `2026-08-12T01:00:00+08:00`: Claimed independent Manager HTTP automated E2E coverage for the G2D+ preparation batch. Scope is limited to the new integration suite, TODOLIST bookkeeping, and iteration logs; no production code or existing test files are to be changed unless a test exposes a focused defect.

- `2026-08-12T01:00:00+08:00`: Released C1c Skills Reload wiring. Added workspace-scoped Runtime Control, lazy all-Session skills invalidation, Manager upload/delete notifications, Applied-only `memory_distill` reload, upload no-op detection, focused tests, `just ci`, and CLI help smoke. Commits `cdfb8e23` and `9639df5c`; no push. LOCK remains intentionally uncommitted.

- `2026-08-11T20:45:00+08:00`: Claimed C1c Skills Reload wiring; superseded by the release note above after implementation, validation, and focused commits.

- `2026-08-11T20:34:00+08:00`: Released after C1c Workspace Memory Epoch implementation. Typed Provider authority/projection revisions, workspace-scoped Runtime Control refresh, apply/recovery/replay/rollback notification wiring, focused tests, `just ci`, and CLI help smoke passed. Commits `bc989338` and `824ff890`; no push. Skills reload remains a separate TODO.

- `2026-08-11T19:03:17+08:00`: Released after CTX-C5c/C5e/C5d tools lifecycle convergence. Canonical tool results, automatic deferred activation, three-region bounded context recovery, full gates, CLI help smoke, and deletion-proof passed. Commits: `6f1331f0`, `3ce9eba2`, `e25a97fd`, `0ffa01fb`, `54715677`, `ce1dc08b`; no push.

- `2026-08-11T18:30:00+08:00`: Released after CTX-C5b implementation. The workspace now has one bounded `canonical_checkpoint_v1`, one unified checkpoint compactor entry, tool-group-aware boundaries, and reactive turn-local pending updates. `just ci`, CLI help smoke, and the production deletion proof passed. C5c-C5e remain separate follow-up scope; no push.

- `2026-08-11T18:05:00+08:00`: Released after C5a closure. Final provider-wire cache snapshots and CORE-only tool prefix hashes are live; heuristic hit/miss state is deleted. Commits: `73dff1bc`, `a8747e86`, `1673b2d8`. Full `just fmt-check`, `just check`, `just test`, `just ci`, affected strict Clippy, and CLI help smoke passed. C5b is next; no push performed.

- `2026-08-11T17:10:00+08:00`: Released after adding C5e automatic deferred tool activation. The scheduled clean break retains `tool_search`, removes model-visible `mount_tool` and persistent discovered state, bounds the task-local active set, and preserves all authorization/approval gates.

- `2026-08-11T16:25:00+08:00`: Released after revising C5 into lightweight context convergence with a strict clean-break/no-compatibility policy and C5a-C5d slices. Documentation diff check passed; implementation remains pending and must use separate focused locks/commits.

- `2026-08-11T10:20:05+08:00`: Released after CTX-C4 implementation, focused and full workspace tests, `just ci`, strict affected Clippy, and CLI `--help` smoke all passed. No push performed. C4 iteration logs are in `docs/logs/2026-08-context-management-enhancement/v0.0.9-c4-deferred-tool-discovery-recall/`.

- `2026-08-11T09:39:00+08:00`: Released after merging both parked batches into `agent-diva-pro` (QoderCN, user-approved). `feat/gmh41-budget-closure` merged `8c1f5b39` (only LOCK.md conflicted: kept mainline notes + branch's 12:00 GMH closure note superseding the 10:15 interim; stray marker cleaned in `a1a60355`). `fix/small-fixes-batch` merged `c05ce32a` (clean). TODOLIST bookkeeping `5245e92f`: GATEWAY-PORT-CONFIG-IGNORED and PROVIDERS-EXAMPLE-1.94-CLIPPY checked, laputa `int_plus_one` closed, LAPUTA-TESTS-1.94-ALL-TARGETS-CLIPPY added. Gates on merged tree: `just fmt-check` / `just check` green; `just test` fails only the 6 pre-existing `CLI-WIREMOCK-502-PREEXISTING` cases; CLI bin 16/16 including the two new gateway-port tests. Not pushed. Follow-up: remove merged worktrees `agent-diva-gmh41` / `agent-diva-small-fixes` (pending user OK); gateway 端口人工验收仍挂人工测试汇总区。

- `2026-08-11T09:18:00+08:00`: Released after CTX-C3 closure. Secure session-persistent tool artifacts, structured full-output execution, versioned references, bound `read_tool_result`, main/supervised wiring, session deletion/startup GC, and C2-driven oldest-first microcompact are complete. Commits: `b6959dc2`, `02fe040c`, `ffff0fe6`, `cdc88fe7`. Full `just fmt-check`, `just check`, `just test`, `just ci`, affected strict all-target Clippy, and CLI `--help` smoke passed. C4 remains next.

- `2026-08-11T10:25:00+08:00`: QoderCN edited `TODOLIST.md` **with explicit user authorization** while the CTX-C3 lock covers it: inserted a new aggregation section 「人工测试验收汇总（Human / Real-Device Smoke）」 between Operational Rules and Open Backlog. Follow-up (same user request, commit after this note): StepFun moved out of the manual section into a new 「E2E 自动化验收汇总」 section, which also aggregates the Wave 3 residual production-path E2E items. No existing items were moved, reworded, or checked. Mainline session: if in-flight TODOLIST edits touch those insertion points, keep both changes. Committed as standalone docs changes; `LOCK.md` itself left uncommitted (shared live mutex file).

- `2026-08-11T12:00:00+08:00`: Released GMH closure lock (QoderCN, `feat/gmh41-budget-closure` worktree `agent-diva-gmh41`). GMH-41/50/52 代码可完成项全部落地并逐片单 concern 提交（未 push）：S2a 删迁移死代码 `93e9f5b6`；S1b 拒绝熔断 `321f4405`；S1c offline 拒绝 `9e772267`；S2b migration feature flags `325c834d`；S3 CI+deletion-proof `77380b5f`；fmt `f388dda6`。`just ci` 仅剩既有 `CLI-WIREMOCK-502-PREEXISTING` 6 例失败（未触碰 agent-diva-cli）。day/hour 限额已按用户决策推迟为待决策独立功能提案（见 TODOLIST.md）。
  （supersedes the 10:15 interim note: S1b/S1c/S2b/S3 then pending.）

- `2026-08-11T08:15:00+08:00`: Released after CTX-C2 layered context budget and AssemblyReport closure. Stable rules, CORE/DEFERRED schemas, L1, WM, Recall, compaction, history, inline tool results and current turn are measured in typed layers; Recall drops explicitly under pressure; macro compaction and legacy count fallback report typed reasons. Agent 413 tests, strict Clippy, full `just ci`, clean-break and CLI help smoke passed. C3 is next.

- `2026-08-11T09:40:00+08:00`: Released `small-fixes-batch` (QoderCN, isolated worktree, no overlap with CTX-C2): 3 commits on `fix/small-fixes-batch` — `83b87787` gateway port config fix (cli), `1aba27a3` providers Rust 1.94 clippy lint batch, `2ccb05e6` iteration logs. Gates: just fmt-check / just check / cli bin 16/16 / providers --all-targets clippy + retry 11/11 green. Deferred until CTX-C2 releases TODOLIST.md: mark GATEWAY-PORT-CONFIG-IGNORED and PROVIDERS-EXAMPLE-1.94-CLIPPY done, close `Laputa service 预存 clippy int_plus_one` (verified already resolved), and add new entry LAPUTA-TESTS-1.94-ALL-TARGETS-CLIPPY (authority_boundary_guard / direct_write_guard / governance_proof_loop dead_code ×5, context_plane_invariants cmp_owned, authority_boundaries ×1).

- `2026-08-11T02:14:55+08:00`: Released after C1d prompt-cache observability closure. Provider cache profiles, explicit stable-system/core-tool anchors, classified prefix hashes, warmup-aware two-sample miss detection, and cache-token ledger persistence are implemented. Affected tests, strict Clippy, full workspace fmt/check/test, and CLI help smoke passed. `just ci` reaches only the pre-existing clean-break violation at `agent-diva-laputa/src/bml/mod.rs:3`; C2 is next.

- `2026-08-10T23:44:12+08:00`: Released after C1c/P0-2 SessionStable section cache. ContextBuilder now caches four stable sections and rendered prefix per session with typed break reasons and prefix versions; mask/L1 refresh is selective, reset/delete/shutdown are wired, and compact does not clear the cache. T5–T6, Agent 403 tests, full workspace fmt/check/test, and CLI help smoke passed. `just ci` reaches the final pre-existing clean-break violation at `agent-diva-laputa/src/bml/mod.rs:3`, recorded in TODOLIST. C1d is next.

- `2026-08-10T22:27:59+08:00`: Released after C1b/P0-3 tool schema stability. `ToolRegistry` now emits sorted CORE then DEFERRED definitions with recursive JSON object canonicalization; MCP/custom registration uses the deferred suffix. T4 covers repeat calls, reverse registration, and independent ToolAssembly rebuilds. Tooling 31, tools 115, agent 397, affected clippy, CLI help smoke, and full `just fmt-check` / `just check` / `just test` gates passed. Provider `apply_cache_control` was not changed. C1c/P0-2 is next.

- `2026-08-10T18:44:07+08:00`: Released after C1a/P0-1 typed stable-prefix production migration. Stable prompt now consumes C1-0 sections; time/session, WM, Recall, Plan/Ask/Scheduled use provider-aware post-prefix envelopes; reactive compaction reuses the same turn snapshot. Provider 122, agent 396, compaction integration 11, compaction E2E 15, and full `just fmt-check` / `just check` / `just test` gates passed. C1b tool schema stability is next.

- `2026-08-10T01:15:00+08:00`: Released after freezing context-management construction decisions DEC-CTX-A..G across README/C0/C1: provider-aware volatile serialization, typed C1 migration, explicit snapshot invalidation, atomic tool/cache changes, artifact safety, same-turn mount, and classified cache observability. Documentation-only revision; diff check clean.

- `2026-08-10T00:45:00+08:00`: Released after C1-0 context contract and characterization closure: provider-neutral section/stability/order skeleton, six focused contract/characterization tests, no production wire-shape change, agent lib 390 tests and full `just fmt-check` / `just check` / `just test` gates green. C1 stable-prefix migration remains next.

- `2026-08-10T00:05:00+08:00`: Released after recording the user-approved SEV-P1 disposition: OpenHarness aggregate closed except deferred EventBus Trait Hooks, Harness Gap prioritized next, and F3/GMH-52/Windows release/CLARIFY-HITL deferred until later.

- `2026-08-09T17:56:00+08:00`: Released `LAPUTA-PERSONA-WORKSPACE` after implementing and validating the singleton persona lifecycle workspace, session Frozen Core effectiveness projection, inline governance actions, and strict no-legacy-compatibility boundary.

- `2026-08-07T23:55:00+08:00`: Released after LAPUTA-COGNITIVE-SYNC (S0–S7) closure: 15 commits on `feat/laputa-cognitive-sync`（未 push）— S0 基线修复 `88195ffa`；S1 cognitive/MEMRULES `41e60a7e`；S2 WORLD.MD claim 存储+治理 upsert `5788eddf`/`b22b50e8`；S3 Frozen Core 会话冻结 `1c97d7be`；S4 人格文件层退役 `49e778e1`/`57044ba8`/`989a18a2`；S5 注册表 14→8 硬删 `e61630b8`；S6 报告边界重构+D2 产物迁移 `23ea4b1e`/`856335b2`/`6def944e`；S7 Context Plane 负向不变量矩阵 `2457239b`；另 3 笔 TODOLIST docs 提交。全量 `just ci` 仅余 6 个基线预存在 CLI wiremock 502 失败（CLI-WIREMOCK-502-PREEXISTING）。S6-4 节律以进程级集成测试验证，完整 daemon-cron 真机挂 G2D+。四件套：docs/logs/2026-08-laputa-cognitive-sync/(plan/summary/verification/acceptance)。待用户评审合并分支。

- `2026-08-07T14:00:00+08:00`: Took over stale Wave 5 S2 lock (expired 2026-08-07T13:30+08:00, no heartbeat after 11:30; 核实该锁对应工作已全部提交——`16aa46ed` Wave 5 S2 superseded gate、`166e499a` Wave 5 收口、Wave 6 亦已 close，仅锁文件未释放，无丢失工作)。新任务 LAPUTA-COGNITIVE-SYNC 开工清理：3 commits 落袋（提案+TODOLIST 冻结 `2c3fbe5e`、根文档归位 docs/ + crate AGENTS `fca1505e`、legacy archive 清理 `23bf2de9`）；工作树清零后从 agent-diva-pro 切 `feat/laputa-cognitive-sync` 开始 S1。

- `2026-08-06T22:30:00+08:00`: Released after GA-MEM-PARITY Wave 4 (AutoDream G4 dedup) closure: 3 commits — `f042bba4` laputa `LaputaService::applied_authority_digests` + new `LaputaError::InvalidState(String)` variant + service wave4_tests × 3; `a39638bb` autodream worker dual-path digest merge (legacy section + typed authority) + tracing::warn graceful degradation + worker wave4_tests × 3; `76dc772c` docs close (TODOLIST WAVE4 checked + "Wave 4 延期项" G1/G2/G3/G5/G6/G7/G10/G11/G12 条目化 + memory-write-paths-contract.md "Realized in Wave 4" 追溯注脚 + v0.0.8 iteration logs). All gates green per slice (fmt/clippy -D warnings; laputa 38+9, autodream 14+6 suites); full workspace test only fails the 6 pre-existing CLI wiremock 502 cases (CLI-WIREMOCK-502-PREEXISTING). Wave 5 (consolidation 条目化 + B7 GC + F3/F4/F6/F7 延期项收口) or G2D+ 真机桌面验收 pending. Deferred to G2D+ / 后续独立 Wave: G1 手动触发端到端、G2 自动阈值触发联通、G3 多源输入闭环、G5 候选→proposal 端到端、G6 审查 UI、G7 节律报告可见、G10 与 agent 即时记忆分工真机验证、G11 L4/salient 等价、G12 Action-Verified 公理对齐.

- `2026-08-06T19:45:00+08:00`: Released after GA-MEM-PARITY Wave 3 (read-side closure) closure: 3 commits — laputa wave3_tests (8 tests + supersedes-target production bug fix via new TypedMemoryStore::superseded_target_ids), agent wave3_tests (3 tests covering D2 prefetch degradation + D4 typed injection order), docs close (TODOLIST WAVE3 checked + F3/F4/F6/F7 延期条目化 + v0.0.7 iteration logs). All gates green per slice (fmt/clippy -D warnings; laputa lib 35 + 集成 9 / agent lib 386 + 集成 15); full workspace test (excluding CLI/GUI) green; CLI 6 pre-existing wiremock 502 cases remain (CLI-WIREMOCK-502-PREEXISTING). Wave 4 (AutoDream dedup, inventory §10.3 G4) pending. Deferred to Wave 5 / GMH-52: F3 GUI/CLI approval memory 端到端, F4 同会话热注入, F6 Rollback 端到端, F7 tombstone U3 完整路径.

- `2026-08-06T17:25:00+08:00`: Released after GA-MEM-PARITY Wave 2 (layers + working memory) closure: 7 commits — working memory trait surface + L1 budget config, typed L1 index rendering + session checkpoint (incl. deps lockfile), L0 policy + working memory turn injection + session end enumeration, update_working_checkpoint tool + distill evidence, docs close (TODOLIST WAVE2 checked + v0.0.6 iteration logs), CLI builtin gates wiring fix. All gates green per slice (fmt/clippy -D warnings; core 692 / laputa 27+ / agent 383 / tools 109 / manager); full workspace test only fails the 6 pre-existing CLI wiremock 502 cases (CLI-WIREMOCK-502-PREEXISTING). Wave 3 (read-side closure) pending; see TODOLIST WAVE3-MEMORY-READ-CLOSURE. Deferred to Wave 5: session-abort checkpoint residue GC, B9 enforced tool-result evidence binding.

- `2026-08-06T15:35:00+08:00`: Released after GA-MEM-PARITY Wave 1 (memory tool CRUD) closure: 6 commits — core CRUD trait surface, typed memory CRUD provider methods, legacy proposal-first CRUD + coordinator wiring, memory add/list/search/update/remove/distill tools, docs close (TODOLIST WAVE1 checked + v0.0.5 iteration logs). All gates green per slice (fmt/clippy -D warnings; core 682+/laputa 20+/agent 378+/tools 12+); full workspace test only fails the 6 pre-existing CLI wiremock 502 cases (CLI-WIREMOCK-502-PREEXISTING). Wave 2 (working memory/layers) and Wave 3 (read-side closure) pending; see TODOLIST WAVE3-MEMORY-READ-CLOSURE.

- `2026-08-06T03:15:00+08:00`: Released after GUI provider error/retry visibility fix (ERROR-SILENT + RETRY-VISIBILITY): 10 commits — core AgentEvent variants, providers retry listener channel (Arc callback + trait hook + ProviderTap forward), agent per-call listener injection, manager dual-stage idle timeout + SSE mapping, Tauri bridge provider events + broken-stream fallback, GUI retry/stall badges, e2e collector match, fmt, docs+TODOLIST. All gates green (fmt/check; core 678 / providers 127 / agent 372 / manager 109 / e2e 61 / GUI vitest 454 + vue-tsc). CLI smoke vs local 500-mock confirmed 1+3 retries + error propagation. HTTP SSE end-to-end + GUI desktop smoke pending (blocked by hardcoded gateway port 3000 while user gateway PID 8856 holds it; recorded GATEWAY-PORT-CONFIG-IGNORED). New TODOs: GUI-TAURI-PLAN-STREAM-DISCONNECT, PROVIDERS-EXAMPLE-1.94-CLIPPY, GATEWAY-PORT-CONFIG-IGNORED. User gateway/GUI processes untouched.

- `2026-08-06T01:15:00+08:00`: Released after recording GUI-PROVIDER-RETRY-VISIBILITY backlog entry (TODOLIST.md) per user instruction - record only, no fix.

- `2026-08-06T01:05:00+08:00`: Released after recording GUI-PROVIDER-ERROR-SILENT backlog entry (TODOLIST.md) per user instruction - record only, no fix.

- `2026-08-05T22:35:00+08:00`: Released after CLARIFY-HITL Phase 2 surface closed loop (ask_user): manager HTTP API + injection, CLI interactive (chat/tui), Tauri bridge, GUI QuestionCard with 2s polling; 4 feature commits + style cleanup; manager 106 / core 676 / tools 94 / agent 371 tests pass, GUI vitest 451 + vue-tsc clean, clippy -D warnings clean. Remaining: manual smoke with a real LLM (acceptance.md).

- `2026-08-05T21:40:00+08:00`: Released after CLARIFY-HITL Phase 1 运行时 MVP（ask_user 工具）：core `AskUserCoordinator`、tools `AskUserTool`、装配+配置+prompt、13 个新测试 + mock 集成闭环测试全绿；fmt/clippy 通过；受影响 crate 全量测试通过。全量 `just test` 仅有 6 个既有 CLI wiremock 502 失败（预存在，stash 验证，TODOLIST `CLI-WIREMOCK-502-PREEXISTING`）。Phase 2（GUI QuestionCard / CLI interactive / manager 注入）待独立迭代。

- `2026-08-05T20:33:28+08:00`: Released after archiving ask-user / conversational clarify HITL research (`docs/research/ask-user-clarify-hitl-proposal.md`, README index, TODOLIST CLARIFY-HITL, `docs/logs/2026-08-ask-user-hitl-research/v0.0.1-research-archive/`). Implementation pending.

- `2026-08-05T19:05:00+08:00`: Released after archiving sandbox+HITL approval proposal to `docs/research/sandbox-hitl-approval-policy-proposal.md` (+ README index, cross-link, docs/logs). Implementation still pending; see TODOLIST `审批三模式完善`.
