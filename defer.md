# Deferred backlog archive

Archived: 2026-10-10. This preserves the 47 non-current deferred entries and their historical context from the former `TODOLIST.md`. These entries are postponed, not completed or accepted. The current cross-project follow-ups are listed in [`TODOLIST.md`](TODOLIST.md).

## 最新版本盘点 — 2026-10-10

本节把旧归档与当前实现/决策对齐，不改变下面的历史记录，也不把局部修复误报为 Epic/Story 验收通过。对照基线：DIVA main `ceff6f2e`、VIVY main `003bc01e`、Laputa main `44f5a2c5`；另外检查了本地记忆闭环候选 VIVY `7eceb2b8`、Laputa `1c791ea`。Laputa 候选修复 `caeed1c` 尚未进入其 main。DIVA Wails host 分支 `96f5dd3f` 已包含在 DIVA main。

### 已交付或已有局部修复

| 旧条目 | 盘点结果 |
| --- | --- |
| `GO-HOST-AND-SEALED-PACK` | W1/W2 已交付；旧项可视为完成。后续 W5/W6/W7 不是本项的剩余工作。 |
| `WAILS-NATIVE-GATE` | W0 的 F1/F2/F3/F5/F6 已修复并有 Windows 实测；F4 音频设备仍缺，因此整个 native gate 没有关闭。 |
| `GO-DESKTOP-AND-SPEECH` | W3/W4 的 Linux host、前端 seam 与 Go speech 实现已交付；Windows speech 和真实 provider 证据仍缺，不能把整项标完成。 |
| `MEM-LOOP-USER-SOURCE`、`MEM-LOOP-REFLECTION-ACTMEM`、`MEM-LOOP-FAILED-WINDOW`、`MEM-LOOP-INFER-BOUNDS`、`MEM-LOOP-RECALL-INPUT`、`MEM-LOOP-RECOVERY-ISOLATION`、`MEM-LOOP-NATIVE-READ-LIFETIME` | 记忆闭环本地候选已修复/证明若干组件：可信来源与回执恢复、ACTMEM/反思/归档、未完成窗口保留 run 与水位、截断与大中文来源、重启召回/纠正/删除、墓碑重投、多个崩溃切点、读取生命周期与安全降级。各项完整矩阵、同一候选和前置 Story gate 仍未通过；状态见[记忆闭环 Story 表](docs/plans/diva-next/memory-loop/README.md)。这些是开发级局部证明，不是产品验收。 |
| `MEM-LOOP-FIXTURE-COMPLETE`、`MEM-LOOP-PINNED-REGRESSION` | 反思/recall/Restart 开发夹具和冻结源码 `just ci` 已补齐并通过；S03 schema、同一候选与正式验收仍未完成。 |
| `LAPUTA-ACTMEM-EMPTY-RENDER` | Laputa 候选 `caeed1c` 已修复空 entries 序列化并加验证；修复尚未进入 Laputa main `44f5a2c5`。当前清单已有 Laputa PR #5 / VIVY PR #46 的依赖顺序跟踪。 |
| `OLD-SECURITY-ISSUE-DISPOSITION` | 原 Issue #8 的旧依赖范围已由 DN-M-C 盘点替代；它不是“全仓库无安全问题”的结论。最新记录为 Rust audit 0 个漏洞、2 个 warning，`pnpm audit --prod` 记录 8 条 advisory，仍需按审计记录处理。 |

### 已被新计划或新决策吸收，不再单独排期

| 旧条目 | 新归属 / 处理 |
| --- | --- |
| `WAILS-NATIVE-GATE`、`GO-HOST-AND-SEALED-PACK`、`GO-DESKTOP-AND-SPEECH`、`WAILS-PARITY-AND-RECOVERY`、`CABI-RUST-RETIREMENT`、`PAIRED-ARCHIVE-TAGS`、`WINDOWS-WAILS-ACCEPTANCE` | 旧描述由[当前 W0–W7 迁移索引](docs/plans/diva-next/index.md)统一维护。已完成、部分完成和仍阻塞的状态以上述索引为准；W5 promotion 未通过，W6 仍被 W5/配对 tag 阻塞，W7 尚待最终 owner acceptance。 |
| `MEM-LOOP-PIN-DRIFT`、`MEM-LOOP-REAL-BACKEND` | 候选来源锁定、真实后台与复验现在归 E01 的 S01/S02；S01/S02 仍 Blocked，不能从“已有局部测试”推导完成。 |
| `COGNITIVE-EMBEDDED-LIFECYCLE`、`VIVYSHUTDOWN-DEADLINE-UTIL` | 旧 Tauri 生命周期/单次 shutdown 记录归并到 Go host 的 W1/W5 teardown、recovery 验收；验收缺口仍在，不再作为独立功能项。 |
| `NATIVE-ACCEPTANCE-HANDOFF`、`DN-0S-PENDING-GATES`、`W3 Windows candidate acceptance`、`W4 real-provider evidence`、`W4 Windows speech leg` | 旧验收清单已拆到 W0/W4/W5/W7 的具体行；音频、凭据、完整矩阵与最终 Windows 包仍待完成。 |
| `DN-6B-SHELL-COMPILE`、`tauriVoiceFileReader`/`isTauri*` 命名、`W6 cleanup pending` | 旧 Rust/Tauri 编译和清理工作改由 Go speech 的 W4 与宿主退役 W6 处理。DIVA main 仍保留 `src-tauri`、Tauri 依赖及旧命名，因此不能标为代码清理已完成。 |
| `GARDEN-CONSOLE-DIST` | 旧 `go:embed` 失败记录被执行顺序吸收：先按[runbook](docs/plans/diva-next/memory-loop/runbook.md)构建 console UI，再编译/测试。它不是已落地的源码修复，也无需作为独立功能排期。 |
| `GENERAL-REPORTS-RESOURCES`、`GUI-PET-DORMANT`、`LOCAL-VOICE-OLVRS`、`desktop_pet_*` dispatch noop、`desktop_pet_start_drag` noop | 新范围决策明确：通用报表/资源、宠物窗口与拖拽、local ONNX/OLVRS 不纳入当前迁移/记忆验收；这些是有意搁置，不应当作待修 bug 或恢复旧功能的理由。 |

### 仍是实际未完成项，或只知道绕过方式

| 旧条目 | 当前状态 |
| --- | --- |
| `MEM-LOOP-NATIVE-LIVE` | Windows 密封产品和真实模型验收未执行。 |
| `WAILS-PARITY-AND-RECOVERY`、`CABI-RUST-RETIREMENT`、`PAIRED-ARCHIVE-TAGS`、`WINDOWS-WAILS-ACCEPTANCE` | 仍未完成：Linux W5-1 为 8/17；另一个 Windows fixture 为 15/17，两者是不同候选/验收层，均不能单独关闭 promotion gate。配对 archive tags 也仍未创建。 |
| `W5 promotion gate`、`W0-F4 mic/audio untestable` | Promotion 尚未通过；当前 Windows runner 没有可用音频端点。 |
| `Windows deps-link gotcha` | 已知 `mklink /D` 可绕过 junction 的 hash 问题；这是环境操作说明，不是代码修复。 |
| `Generated bindings drift` | 仍没有 CI drift check。 |
| `GOVERNANCE-DOCS-STALE` | 根目录架构/代理说明仍与 Go/Wails 现状不一致，尚无替代决策或修订证据。 |
| `SKILL-MANAGEMENT-RESIDUAL`、`COMMAND-RULES-AND-WIPE`、`SEARCH-MCP-TUNING-RESIDUAL`、`UI-COPY-CLEANUP`、`GUI-SESSION-PIN-PERSISTENCE` | 没找到最新实现已覆盖的证据；仍是明确的功能/契约缺口或低优先级清理。 |

核对来源：[记忆闭环状态](docs/plans/diva-next/memory-loop/README.md)、[Wails 迁移状态](docs/plans/diva-next/index.md)、[W0–5 证据](docs/logs/2026-10-wails-migration/)、[DN-M-C 安全盘点](docs/logs/2026-10-diva-next-closure/v0.4.13-dn-mc-audit/summary.md)。除上表标明“可视为完成”的项外，其余归档 checkbox 保持原样；`TODOLIST.md` 仍是当前唯一待办清单。

---

## Memory loop verification — execution started 2026-10-09

See the [Epic–Story package and live status](docs/plans/diva-next/memory-loop/README.md).
Formal Story acceptance and final same-candidate gates remain open even where developer-level repairs are recorded. Environment/full-CI evidence is in docs/logs/2026-10-memory-loop-repair/v0.2.0-source-and-environment/. New reflection/Restart developer evidence and continuation are in docs/logs/2026-10-memory-loop-chain/v0.1.0-reflection-and-restart/. Historical failures remain unchanged.


- [ ] **DEFERRED** **MEM-LOOP-REAL-BACKEND** P0 — Reproduce w0-5's selected-backend
  unavailable result and prove real Mentle write/search/expand/reopen through
  the sealed host. Audit local-model initialization and read-only modes (S02).
- [ ] **DEFERRED** **MEM-LOOP-USER-SOURCE** P0 acceptance — Admitted durable user-source/role repair and random-fact canonical/process regression are implemented. Trusted accepted receipt recovery is repaired without rewriting old effects. Finish S04 V05–V09 and separately verify any legacy source backfill; do not mark full Story Done from partial proofs. See docs/logs/2026-10-memory-loop-repair/v0.2.0-source-and-environment/.
- [ ] **DEFERRED** **MEM-LOOP-REFLECTION-ACTMEM** P1 — Prove actual Pulse/Recap/Work,
  automatic reflection, effect receipts and persona-review continuity;
  Actual Pulse/Recap/Restart/completed and live cancellation Session archive, preserved old Work and stale-patch rejection now have v1.1 developer proof (VIVY 1c0ef749 / Laputa 4da565e). Complete concurrent Work/trigger and all formal same-candidate gates; domain API availability alone is not product wiring evidence (S05–S07).
- [ ] **DEFERRED** **MEM-LOOP-FAILED-WINDOW** P0 — Budget and unresolved-window fences are repaired; VIVY `d72add09` preserves run/window/watermark and exposes blocked status, Laputa `256d1f1` stops after unresolved work. VIVY e62a929e / v1.3 repairs real-process recovery commit collision and exposes unknown_outcome; runtime race45 / actual App race2 pass. Continue original-operation receipt recovery and six-cut crash matrix before acceptance. Do not infer crash idempotence from the now-green single-source reflection case (S06/S10).
- [ ] **DEFERRED** **MEM-LOOP-INFER-BOUNDS** P0 acceptance — Truncation and duplicate post-reflection evidence packets repaired: VIVY `e1de33ae`, Laputa `9bc39af`. Actual over-4-KiB Unicode-source reflection, runtime 75, selected composition 11, Laputa 64 and Garden 480 pass with zero skip. Complete remaining input limits/policy matrix and final new-source CI/sealing; this remains local work (S06).
- [ ] **DEFERRED** **MEM-LOOP-RECALL-INPUT** P0 acceptance — Native manifested Context Source and actual three-profile Restart recall/six controls/later correction-delete input now have developer proof (VIVY 9cbed3ea; v1.0 checkpoint). Complete formal S08/S09, explicit Agent tool-only boundary and final same-candidate gates; do not substitute Frozen Core/BML or scripted replies for live acceptance.
- [ ] **DEFERRED** **MEM-LOOP-RECOVERY-ISOLATION** P0 — Six process-crash cuts, unknown
  effect handling, backend degradation, correction/deletion in later model input, foreign scope
  and untrusted memory content require integrated evidence (S09–S11). Public
  correction/tombstone/restart and accepted-source redelivery now have developer
  proof (VIVY 7a1086a5, v0.9 checkpoint); later model-input proof is in v1.0. Full S08 and crash cuts remain open.
- [ ] **DEFERRED** **MEM-LOOP-NATIVE-READ-LIFETIME** P0 acceptance — Cache admission, Observer recovery startup, canceled ONNX Close and canonical evidence/collection REDs repaired (VIVY b348803c; Laputa fa95945/19816ce/4faf95a). Preserve positive cold-recall race deadline failure, earlier Close timeout and correction RPC failure; complete S11 robustness/full-candidate evidence before acceptance (v1.0 checkpoint).
- [ ] **DEFERRED** **MEM-LOOP-NATIVE-LIVE** P1 — Verify the same sealed candidate on
  Windows and with a real model; historical 15/17 migration rows do not
  establish memory-loop correctness (S12/S13).
- [ ] **DEFERRED** **MEM-LOOP-PIN-DRIFT** P1 — DIVA's source lock, historical Windows
  artifact and current sibling repositories differ. Preserve exact baseline
  evidence and use supported candidate repinning before acceptance (S01).

## Approved migration work — planning delivered, execution not started

- [ ] **DEFERRED** **WAILS-NATIVE-GATE** P0 — W0 must prove pinned Windows native caller,
  media, lifecycle, keyring and WebView2 behavior before adoption.
- [ ] **DEFERRED** **GO-HOST-AND-SEALED-PACK** P0 — W1/W2 public Go lifetime owner and
  generated external host packaging; retain single Runtime/Journal/grants.
- [ ] **DEFERRED** **GO-DESKTOP-AND-SPEECH** P1 — W3/W4 frontend seam and DIVA Go speech;
  preserve raw audio, secret boundary, cancel and generation semantics.
- [ ] **DEFERRED** **WAILS-PARITY-AND-RECOVERY** P0 — W5 fresh FrozenCore, persisted-session
  admission, runtime logs, required chat/cognition/console/voice and truthful
  hide/quit/crash/restart evidence. No language-change shortcut closes these.
- [ ] **DEFERRED** **CABI-RUST-RETIREMENT** P1 — W6 remove retired host/ABI and replace
  CI/instructions after W5. Blocked until paired archive tags exist.
- [ ] **DEFERRED** **PAIRED-ARCHIVE-TAGS** P1 — archive/tauri-cabi branches created at
  DIVA 5444795a / VIVY fc559e6b. Annotated archive/tauri-cabi-20261004 tags
  remain pending tag-capable authenticated access; exact instructions in
  [archive record](docs/plans/diva-next/wails/archive.md). Old ignored binary
  is not preserved or reverified by the source freeze.
- [ ] **DEFERRED** **WINDOWS-WAILS-ACCEPTANCE** P1 — W7 clean installed final Go product;
  engineering preparation first, owner acceptance last.

The index contains eight executable Story plans with immediate dependencies,
six waves and verification gates. This publication authorizes no product
implementation and makes no Ready/Done implementation claim.

## Prior closure source deliveries and retained residuals

- [x] **COGNITIVE-COMPOSITION** P1 — Landed via DN-4A/DN-LC/DN-4C + sealed in
  DN-P-C (`vivy/diva-cognitive`, 20 control actions, one Garden owner per
  profile). Candidate-level residuals moved to OBS09 findings: F2 (Bootstrap
  capture seam unwired — first turn on a fresh session fails until a Frozen
  Core is captured), F3 (per-process action grants). See OBS-09 fixture;
  owner acceptance via NATIVE-ACCEPTANCE-HANDOFF.
- [ ] **DEFERRED** **COGNITIVE-EMBEDDED-LIFECYCLE** P1 — Current merged VIVY source
  already starts the cognitive loop through StartEmbeddedServices. W1/W5
  must prove single startup, policy-disabled behavior, stop/drain ordering,
  recovery and bounded close on the new Go host; do not add a duplicate loop.
- [x] **COGNITIVE-AUTHORITY-AND-RECOVERY** P0 — DN-4B landed per-run
  persisted authority/guard (ActiveRunID/PendingThrough instead of
  Attempt++), durable `unknown_outcome` block (`recovery_required`), and
  serialized admission; capture keeps `run_id:journal_seq` identity so no
  new-attempt replay after unknown outcome. Candidate-level OBS check did
  not re-drive a recovery scenario; exercised gate remains owner scope.
- [x] **COGNITIVE-SUPERVISOR-CAPTURE** P1 — DN-4B: supervisor session
  (`sess_cognitive_supervisor`) primaries excluded from capture with ack
  receipt; capture key stays `run_id:journal_seq` so restarts cannot feed
  strategy output back as fresh evidence.
- [x] **PINNED-NATIVE-BUILD-CLOSURE** P1 — DN-0P staged tree + CGO inventory
  (libc only, no ONNX/OpenSSL/libsqlite3) and DN-P-C repack/inspect
  (generation `331bb89d`, 21 modules, sha256 `d0155e26`); source pins in
  `closure-packaged-obs.json`. Windows x64 link remains under
  NATIVE-ACCEPTANCE-HANDOFF.
- [x] **CONSOLE-OBS-CONSUMERS** P1 — OBS-06/07/08 landed; OBS-09 verified on
  the packaged candidate: trajectory v2 + token totals equal journal, one
  event owner, bounded queries, gap flag, restart-stable replay. Residuals:
  runtime log sink absent (OBS09-F1), window hide/reopen + rotation pending.
- [x] **CHAT-ATTACHMENTS** P1 — DN-2A: image bytes/MIME wired through
  `vivy_call` with size guards and explicit unsupported-file UI.
- [x] **CHAT-CONTROLS** P2 — DN-2B: permission mode, regenerate-in-place,
  start_goal entry and authoritative cancel wired to VIVY contracts.
- [x] **ONLINE-SPEECH-NATIVE** P1 — DN-6A/B/C landed: exact native commands,
  keyring credentials with explicit unavailable state, SiliconFlow STT +
  SiliconFlow/MiniMax TTS request/cancel paths, generation-scoped playback in
  main chat. Live-provider/Windows gates remain under DN-0S-PENDING-GATES /
  NATIVE-ACCEPTANCE-HANDOFF.
- [x] **VOICE-ASSET-BOUNDARY** P2 — DN-6A: bounded `voice_asset_*` import/
  list/read/delete with leases; general VRM management stays deferred.
- [x] **RUNTIME-REPIN-AND-GATES** P1 — Repacked/inspected/staged under DN-P-C
  (v0.4.11): Generation `331bb89d…` with `vivy/diva-cognitive` + 20 actions;
  `vivy.default.yaml` gained the nine `diva.cognitive.*` write-action allow
  rules; inventory `dn_pc_refresh` recorded. GUI-level/Windows candidate
  exercise remains under NATIVE-ACCEPTANCE-HANDOFF.
- [ ] **DEFERRED** **NATIVE-ACCEPTANCE-HANDOFF** P1 — Superseded host procedure: W7
  prepares the sealed Go/Wails Windows x64 package, real-model/mic/STT/TTS,
  hide/reopen/Quit, cognition/restart and console checks. DLL/header/FFI are
  archived requirements, not tasks for the new mainline product.
- [ ] **DEFERRED** **GOVERNANCE-DOCS-STALE** P1 — AGENTS.md / AGENTS-ARCH.MD / CLAUDE.md /
  LAPUTA.md describe retired Rust crates, old memory rules and absent root
  Cargo recipes. DN-C2 records the conflict; reconcile stale descriptions
  against the authorized Next scope. Obsolete crate rules do not create a
  new permission gate or justify resurrecting retired code.
- [ ] **DEFERRED** **OLD-SECURITY-ISSUE-DISPOSITION** P2 — Issue #8 targets retired
  aws-lc-sys/rsa dependency chains. Neither package exists in the two current
  shell/bridge lockfiles. Fresh `cargo audit` + `pnpm audit` evidence lands
  with DN-M-C (see ledger); no vulnerability-free claim beyond the audited
  advisory sets.

## Deferred residuals (not completed)

- [ ] **DEFERRED** **GENERAL-REPORTS-RESOURCES** P2 — Broader notebook/report scheduling
  and generic resource management remain outside this closure. Cognitive
  reflection notes and required new voice assets are in scope; keep truthful
  unavailable states for other surfaces.
- [ ] **DEFERRED** **SKILL-MANAGEMENT-RESIDUAL** P2 — Upload/delete/content edit/request
  review lack verified DIVA contracts. Latest VIVY has revisions/list when
  its backing dependency is bound, so history is not universally absent.
- [ ] **DEFERRED** **COMMAND-RULES-AND-WIPE** P2 — No verified equivalent for the removed
  command-rule UI or backend wipe. LocalStorage reset does not wipe VIVY data.
- [ ] **DEFERRED** **SEARCH-MCP-TUNING-RESIDUAL** P3 — Legacy search roster/key/max-results,
  MCP timeout/form knobs, context ratio and runtime status columns lack full
  scoped parity. Keep as residuals; do not resurrect old commands.
- [ ] **DEFERRED** **GUI-PET-DORMANT** P2 — Pet windows/neuro-link remain dormant by ruling.
  Online voice must work from main chat/settings without restoring this host.
- [ ] **DEFERRED** **LOCAL-VOICE-OLVRS** P3 — Local ONNX/sherpa and OLVRS follow a separate
  design; no implementation or dependency added in online voice closure.
- [ ] **DEFERRED** **VIVYSHUTDOWN-DEADLINE-UTIL** P3 — Prior auto-title in-flight shutdown
  deadline occurred once. Track bounded teardown on the new artifact; do not
  present driver retries as a product fix.
- [ ] **DEFERRED** **UI-COPY-CLEANUP** P3 — Sweep unused DN-3 locale keys and misleading
  controls only alongside touched flows; no unrelated broad frontend rewrite.
- [ ] **DEFERRED** **LAPUTA-ACTMEM-EMPTY-RENDER** P3 — Upstream defect found by DN-4A:
  laputa `evolution.ActmemDocument.Render()` emits `entries:` null for an
  empty entry map, producing an ACTMEM head that its own reader rejects.
  Task-local native repair 5a5b436 now emits the valid empty map and has real fold/reopen proof; ce919da also repairs readable bounded provenance capsules. Upstream integration and new DN-4A artifact verification remain pending (v1.1 checkpoint).
- [ ] **DEFERRED** **GARDEN-CONSOLE-DIST** P3 — laputa `garden/console` fails
  `go build ./...` until the console UI build produces `dist/`; recorded in
  `closure-build-inputs.json` captured_failures, not patched.
- [ ] **DEFERRED** **DN-0S-PENDING-GATES** P2 — Windows WebView2 IPC/audio roundtrip,
  Credential Manager store path, MSVC+cmake aws-lc-sys link, and
  authenticated SiliconFlow/MiniMax calls remain pending; DN-6/DN-8C own.
- [ ] **DEFERRED** **DN-6B-SHELL-COMPILE** P2 — `src/speech/{mod,commands}.rs` +
  `src/lib.rs` wiring (4 new commands, diagnostic sink, Hide/Quit hooks)
  is rustfmt-clean but compile-unverified: tauri cannot link
  GTK/WebKitGTK on this VM. `just shell-test` / `just shell-clippy`
  whole-workspace runs remain a DN-8C obligation.

## Recorded delivery and cancellation

- [x] **DN-C2-P1-PLANS** — 20 executable Story plans supplied under existing
  DN/OBS stages with requirement coverage, immediate dependencies, waves,
  file ownership, checks and handoff. This closes planning only, not product work.

- [x] Initial shared host/bridge/core/settings/masks delivery: DIVA #16 and
  VIVY #28 merged; prior Linux evidence retained at its original pins.
- [x] **HISTORICAL-IMPORT-CANCELLED** — Owner decision 2026-10-03: no old data
  import, because there are no current DIVA users. DN-7 cancelled and removed
  from DN-8 prerequisites; do not build an importer or delete old data.
- [x] Previous mixed backlog preserved as an explicit historical snapshot:
  [2026-10-03-before-next-closure.md](docs/archive/todolist/2026-10-03-before-next-closure.md).
  Old Rust-specific tests/tasks are not active Next work by default; product
  residuals above remain visible. Historical boxes are not new acceptance.


## DN-W3 migration residuals (2026-10-04)

- [ ] **DEFERRED** **W5 promotion gate** — after w0-5: 15 passed / 2 pending / 0 failed in `fixtures/wails-candidate-acceptance.json` (VOICE-REAL [no audio endpoints on VM] + Windows rollup pending — both environmental); `scripts/ci/check_wails_candidate.py --require-all-passed` is the gate.
- [x] **W0-F5 sealed turn/start frozen-core gate** — FIXED upstream (vivy `e3b60280`: lazy capture on first Prepare on agentapi not_found); verified w0-4 — fresh-session GUI turn wrote `frozen_core_sessions` row at first Prepare and reached the model call.
- [x] **W0-F6 GUI never binds/renders session** — FIXED in diva `dda9d7da`. Root cause (CDP capture, w0-4): wails3 injects `window._wails.environment` via execJS on NavigationCompleted, after mount; `isTauri()` sampled at mount was false → browser-mode return → `vivyChat.connect()` never ran. Fix waits for `wails:runtime-config-ready` (3s bound); verified: online on boot + history renders + reload/restart resync clean.
- [x] **W0-F1 sealed frontend not embedded** — FIXED upstream (vivy `42c263f2`: pack overlays built dist into staged tree); verified live w0-3 (real Vue UI renders).
- [x] **W0-F2 no reopen affordance** — FIXED upstream (diva `8db11f51`: tray AttachWindow + Show/Quit menu); verified live w0-3 (click toggles, Quit releases lease).
- [x] **W0-F3 no explicit-Quit affordance** — FIXED upstream with F2 (tray Quit → graceful teardown + lease release); verified w0-3.
- [ ] **DEFERRED** **W0-F4 mic/audio untestable on build VM** — Audiosrv disabled + zero PNP audio devices on Windows Server 2022; needs an audio-capable machine (plus operator credentials for real-provider rows).
- [ ] **DEFERRED** **Windows deps-link gotcha** — `mklink /J` junctions break `hashSourceTree` (`Incorrect function`); real directory symlinks (`mklink /D`, elevated) work. Worth a setup note or scripted step for Windows builds.
- [ ] **DEFERRED** **W3 Windows candidate acceptance** — lifetime + UI/chat rows green on Windows (w0-2…w0-4); remaining pending rows are grants-catalog/cognitive drives + VOICE-REAL (no audio endpoints).
- [ ] **DEFERRED** **W4 real-provider evidence** — SiliconFlow STT + SiliconFlow/MiniMax TTS need operator-held credentials; scripted fixtures landed (w4-1).
- [ ] **DEFERRED** **W4 Windows speech leg** — Credential Manager proven (w0-2 Task 5); speech lifecycle on Windows blocked by W0-F4.
- [ ] **DEFERRED** **`desktop_pet_*` dispatch noop** — pet commands still `not_ready`; pet lands with drag/expansion (deferred).
- [ ] **DEFERRED** **`desktop_pet_start_drag` noop** — DesktopPetOverlay drag calls a stub; Wails native drag (`--wails-draggable` regions) lands with pet expansion (deferred).
- [ ] **DEFERRED** **`tauriVoiceFileReader`/`isTauri*` naming** — functional seam is Wails; leftover cosmetic names + locale strings ("Tauri 运行时") should be renamed in W4/W5 cleanup.
- [ ] **DEFERRED** **W6 cleanup pending** — `src-tauri/`, `@tauri-apps/*` deps, Rust/tango recipes in justfile, `scripts/tauri` helpers all still present until W6 removal.
- [ ] **DEFERRED** **Generated bindings drift** — `src/generated/wails/` must be regenerated (`just desktop-bindings`) after any bound-method change; no drift check in CI yet.

## GUI follow-ups

- [ ] **DEFERRED** **GUI-SESSION-PIN-PERSISTENCE** P3 — no verified session pin write contract exists; the no-op pin action and stale grouping were removed until persistence is available.


## Memory execution environment and fixture blockers

- [x] **MEM-LOOP-NATIVE-BUILD-DEPS** — Resolved task-locally without system/HOME changes: extracted native headers/libraries, supported sealed Linux host test/race and development build pass. Original product lock is unchanged. Windows/native UI/live-model acceptance remains separate (S01/S12/S13).
- [ ] **DEFERRED** **MEM-LOOP-FIXTURE-COMPLETE** P1 — ack/full App/real storage evidence works; reflection/recall modes, reflected observation and real process Restart are explicitly pending. Complete S03 contract before downstream gate acceptance.
- [ ] **DEFERRED** **MEM-LOOP-PINNED-REGRESSION** acceptance — Underlying blockers repaired: Laputa 63 pass; Mentle 536 pass / 0 skip; Garden 480 pass / 0 skip and process e2e 2 pass. Exact dependency closure/model/config bindings preserved. Managed ancestor instruction/VCS and Codeface home-write fixtures repaired; complete frozen-source just ci now exits 0. Formal same-candidate Story evidence remains open.
