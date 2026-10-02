# DIVA Next — authoritative planning index

The existing DN-P1 issue design remains historical context. The user's two-P0 directive selects a **Tauri thin shell + Go shared library**, superseding its non-Rust-host and desktop WebSocket assumptions. Current review draft: [P0-D1 design](p0-design.md). Only this index owns status and dependencies; no external issue has been modified.

Inspected baselines: DIVA `d96e396d1641a5e7636e30e1f6cdbd0e597b62c8`; VIVY `5347032d8f18a047b67e85761c0dcc48728bee7c`. Retired Rust behavior remains available in DIVA git history `0fd005a1`/origin/dev. Source pins do not prove a shipped Generation.

## Scope and state

P0-A delivers the shared library/bridge and packaged core lifecycle. P0-B delivers complete semantic migration of required old frontend functions. Full DIVA Next release still includes historical DN-7/DN-8 obligations. Planning is authorized; implementation is not started. Windows x64 is the target candidate until DN-0 freezes the acceptance platform. No dates, staffing or duration estimates are asserted.

| Story | Outcome / requirement | Immediate predecessors and required output | Plan | State | Evidence or blocker |
| --- | --- | --- | --- | --- | --- |
| DN-W | Retired Rust business backend deleted / R-2,R-8 | — | [DN-W](DN-W.md) | Done (historical) | Historical branch evidence: 485 GUI tests and Vue build; not rerun here |
| DN-0 | Full mapping, core/ABI/platform contract freeze / A1,B1 | — | [DN-0](DN-0.md) | Complete (proposed; owner review) | Ledger + frozen ABI + captured core-rpc transcript delivered; gatewayless Go tests pass on linux (Go 1.26.8 installed); `EMBEDDED-SWEEPER-OWNERSHIP` resolved in DN-L |
| DN-L | Sealed DIVA DLL + safe embedding / A1 | DN-0: ABI, recipe needs, target/toolchain | [DN-L](DN-L.md) | Complete (proposed; owner review) | `internal/embedded` host + ABI v1 c-shared exports + `--target shared` pack/inspect + `recipes/diva.vivy.yml` landed on `feat/diva-embedded`; C ABI smoke (init→turn→approval→cancel→poll→shutdown) green on linux/amd64; windows/amd64 c-shared + Rust FFI acceptance still pending |
| DN-5 | Tauri shell + vivy-bridge / A2 | DN-L: accepted DLL/header/manifest | [DN-5](DN-5.md) | Complete (proposed; owner review) | `vivy-bridge` (dlopen, owned-return guards, single event pump) + Tauri v2 shell (vivy_call/vivy:event, close-hide + tray Quit, single-instance) landed on DIVA-NEXT-P0; workspace tests 10/10, clippy clean, deb bundles the staged runtime; native transcript: real .so window open, close->hide, relaunch focus, SIGINT ordered exit on linux/amd64; live streaming/approval-reopen + Windows runner still pending |
| DN-1 | Typed client + state/event projection / A3,B2 | DN-5: verified call/event transport | [DN-1](DN-1.md) | Complete (proposed; owner review) | `api/vivy/{contracts,transport,client}` (envelope-preserving, never-retried, unknownOutcome on post-mutation timeout/transport loss) + `state/vivy-session` projection ((run_id,seq) dedup, gap-safe cursor, snapshot-authoritative resync, pending approval/question survival) + capability ledger resolved from negotiated initialize state; GUI tests 507/507, build clean, vivy-bridge 7/7 incl. real-artifact DN-1 smoke (initialize/session/subscribe/cancel -32004/approval+question lists/delete) |
| DN-2 | Real chat/session/approval/cancel / A3,B2 | DN-1: client and recovery projection | [DN-2](DN-2.md) | Complete (proposed; owner review) | `VivyChatController` orchestration authority + App.vue rewiring (2761->1059 lines, legacy listeners/mock/DTOs deleted); review/respond interactions bound to backend ids; plan via session/work + plan/decide; GUI tests 519/519, vue-tsc + build clean. Task 5 native acceptance green on linux/amd64: real sensenova model streamed, write_file gated by policy (deny=no effect, approve=effect in run sandbox), run/cancel->run.cancelled, Journal + review snapshot readback — 16/16 checks (`docs/logs/2026-10-diva-next-dn2/v0.2.0-live-model-acceptance/`) |
| DN-P | Packaged P0-A acceptance + runtime boundary gate / A1–A3 | DN-2: accepted native core chain | [DN-P](DN-P.md) | Planned | Does not wait for unrelated domain migration |
| DN-3 | Settings/operational migration / B2,R-1,R-3 | DN-2: verified mutation/recovery path | [DN-3](DN-3.md) | Complete (proposed; owner review) | Slices A–D landed (ee01962b/204a9c23/ad33f349/011e493c): providers+model+credentials, tools/MCP/skills/marketplace, channels/cron, token stats+sandbox+dead-surface purge; zero non-vivy_call invokes outside the dormant pet feature; vitest 434/434, vue-tsc+build clean; live persist/reopen via sealed .so 10/10 (settings survive VivyShutdown→VivyInit; invalid preset -32602 fail-closed); gaps on TODOLIST (COMMAND-RULES, BACKEND-WIPE, SKILL-MANAGEMENT…) |
| DN-4 | Companion migration / B2,R-4 | DN-2: verified mutation/recovery path | [DN-4](DN-4.md) | Blocked | Pinned persona/memory/evolution/report semantic mappings not yet verified; not a claim all APIs are absent |
| DN-6 | Speech/avatar/native domain migration / B2,R-5 | DN-2: accepted run/utterance/cancel projection | [DN-6](DN-6.md) | Blocked | Requires verified speech/resource contracts; DN-5 host is inherited through DN-2 |
| DN-M | Complete P0-B semantic closure / B1–B3 | DN-3,DN-4,DN-6: domain evidence; DN-P: runtime boundary gate | [DN-M](DN-M.md) | Planned | Also consumes DN-0 ledger as the explicit completeness contract |
| DN-7 | Historical offline data handoff / R-7 | DN-3,DN-4: accepted domain schemas | [DN-7](DN-7.md) | Blocked | Import contracts not frozen; outside these two P0s unless required by a mapped function |
| DN-8 | Full DIVA Next release / R-1,R-2,R-8 | DN-M: full semantic migration; DN-7: import acceptance | [DN-8](DN-8.md) | Planned | P0 closure alone does not close full release |

## Preserved full-product requirements

R-1: every required old behavior has accepted replacement/disposition. R-2: no old Rust business runtime or competing persistence. R-3: real model/chat/tool/approval/recovery against pinned VIVY. R-4: real durable companion state. R-5: optional voice/avatar obey run/cancel/replay semantics. R-6: native lifetime on declared platforms. R-7: repeatable source-preserving historical import. R-8: independent clean product package/CI. P0-D1 narrows the first closing gate without deleting these existing obligations.

## Execution waves

1. DN-0: inventory and freeze; complete read-only work even where native verification is unavailable.
2. DN-L: library, generated DIVA recipe, pack/inspect and C ABI smoke.
3. DN-5: Tauri/vivy-bridge and native lifetime.
4. DN-1: client/projection and replay through the real bridge.
5. DN-2: core user flow and real approval/cancel.
6. DN-P + DN-3 + DN-4 + DN-6: P0-A acceptance and domain migration are logically independent after their contracts are accepted; blocked domains remain blocked.
7. DN-M + DN-7: P0-B closure and historical import.
8. DN-8: whole-product release.

These are topological batches, not promises of parallel staffing or permission to delegate. Contract-based scaffolding may be prepared earlier, but no Story is accepted from a mock predecessor. DN-P is the first P0 closing point; DN-M is the second. A missing DN-4/DN-6 contract need not delay DN-P.

## Shared-file serialization

- App.vue: DN-2 first, then DN-3/DN-4/DN-6 one at a time.
- desktop.ts/capabilities.ts: domain owners make scoped edits, DN-M performs final reconciliation.
- package/lock: DN-5 owns initial shell dependencies; later edits serialized.
- VIVY recipe/pack: DN-L owns; domain-driven capability additions review against DN-0, then rebuild/reinspect and invalidate old artifact acceptance.
- justfile/CI/boundary scripts: DN-5 scaffolds, DN-P supplies runtime gate, DN-M extends consumer gate, DN-8 completes release.
- A wire-schema change updates shared contract fixtures and all consuming plans before implementation resumes. No concurrent writers have been assigned.

## Decision log

- Historical 2026-09-27 DN-P1: inventory, migration, host probe and final release plan established.
- Historical 2026-09-27 DN-W directive: delete old backend before parity; deletion is complete.
- Current P0-D1 directive: use Tauri only as native shell and Go shared library as backend; keep the old business runtime retired. DN-5 moves before DN-1/DN-2 because desktop transport now depends on the DLL, not WebSocket.
- P0-D1 adds DN-L, DN-P and DN-M; updates existing Stories instead of starting a competing planning package. Requires plan review before implementation.

## Next action

DN-0, DN-L, DN-5, DN-1, DN-2 and DN-3 are delivered and awaiting owner review. Outstanding native gates: GUI-level window reopen against a live model (RPC-level snapshot readback already proven in DN-2 task 5), and the Windows/amd64 c-shared + FFI acceptance runner. Next wave: DN-P (packaged P0-A acceptance + runtime boundary gate) or DN-4 (companion migration — still Blocked on persona/memory/evolution/report semantic mappings).
