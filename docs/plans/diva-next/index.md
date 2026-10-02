# DIVA Next — authoritative planning index

The existing DN-P1 issue design remains historical context. The user's two-P0 directive selects a **Tauri thin shell + Go shared library**, superseding its non-Rust-host and desktop WebSocket assumptions. Current review draft: [P0-D1 design](p0-design.md). Only this index owns status and dependencies; no external issue has been modified.

Inspected baselines: DIVA `d96e396d1641a5e7636e30e1f6cdbd0e597b62c8`; VIVY `5347032d8f18a047b67e85761c0dcc48728bee7c`. Retired Rust behavior remains available in DIVA git history `0fd005a1`/origin/dev. Source pins do not prove a shipped Generation.

## Scope and state

P0-A delivers the shared library/bridge and packaged core lifecycle. P0-B delivers complete semantic migration of required old frontend functions. Full DIVA Next release still includes historical DN-7/DN-8 obligations. Planning is authorized; implementation is not started. Windows x64 is the target candidate until DN-0 freezes the acceptance platform. No dates, staffing or duration estimates are asserted.

| Story | Outcome / requirement | Immediate predecessors and required output | Plan | State | Evidence or blocker |
| --- | --- | --- | --- | --- | --- |
| DN-W | Retired Rust business backend deleted / R-2,R-8 | — | [DN-W](DN-W.md) | Done (historical) | Historical branch evidence: 485 GUI tests and Vue build; not rerun here |
| DN-0 | Full mapping, core/ABI/platform contract freeze / A1,B1 | — | [DN-0](DN-0.md) | Ready (investigation only) | Code baselines inspected; complete ledger/fixtures and target verification remain |
| DN-L | Sealed DIVA DLL + safe embedding / A1 | DN-0: ABI, recipe needs, target/toolchain | [DN-L](DN-L.md) | Blocked | Core contract/platform freeze; native target runner/shared artifact acceptance still required. Go 1.26.4 is now installed for research; not native acceptance |
| DN-5 | Tauri shell + vivy-bridge / A2 | DN-L: accepted DLL/header/manifest | [DN-5](DN-5.md) | Planned | Native target runner and accepted ABI producer required |
| DN-1 | Typed client + state/event projection / A3,B2 | DN-5: verified call/event transport | [DN-1](DN-1.md) | Planned | Core schemas from DN-0 remain binding |
| DN-2 | Real chat/session/approval/cancel / A3,B2 | DN-1: client and recovery projection | [DN-2](DN-2.md) | Planned | Live model and safe gated tool required for acceptance |
| DN-P | Packaged P0-A acceptance + runtime boundary gate / A1–A3 | DN-2: accepted native core chain | [DN-P](DN-P.md) | Planned | Does not wait for unrelated domain migration |
| DN-3 | Settings/operational migration / B2,R-1,R-3 | DN-2: verified mutation/recovery path | [DN-3](DN-3.md) | Planned | Per-domain DN-0 schema/fixture gates |
| DN-4 | Companion migration / B2,R-4 | DN-2: verified mutation/recovery path | [DN-4](DN-4.md) | Blocked | Pinned persona/memory/evolution/report semantic mappings not yet verified; not a claim all APIs are absent |
| DN-6 | Speech/avatar/native domain migration / B2,R-5 | DN-2: accepted run/utterance/cancel projection | [DN-6](DN-6.md) | Blocked | Requires verified speech/resource contracts; DN-5 host is inherited through DN-2 |
| DN-M | Complete P0-B semantic closure / B1–B3 | DN-3,DN-4,DN-6: domain evidence; DN-P: runtime boundary gate | [DN-M](DN-M.md) | Planned | Also consumes DN-0 ledger as the explicit completeness contract |
| DN-7 | Historical offline data handoff / R-7 | DN-3,DN-4: accepted domain schemas | [DN-7](DN-7.md) | Blocked | Import contracts not frozen; outside these two P0s unless required by a mapped function |
| DN-8 | Full DIVA Next release / R-1,R-2,R-8 | DN-M: full semantic migration; DN-7: import acceptance | [DN-8](DN-8.md) | Planned | P0 closure alone does not close full release |

## Observability domain extension (OBS-D1)

Reviewable specification: [observability/architecture.md](observability/architecture.md). [Epic/Story map](observability/index.md) owns traceability/order; this parent index alone owns delivery state. This is the observability slice of DN-3. The remaining DN-3 domains and full P0-B obligations are unchanged. Planning is authorized; implementation approval/spec freeze is pending.

| Story | Result / requirements | Immediate predecessors and required output | Plan | State | Evidence or blocker |
| --- | --- | --- | --- | --- | --- |
| OBS-01 | Terminal/file logging + prettylog / O1 | — | [OBS-01](observability/OBS-01.md) | Planned | OBS-D1 review pending; prettylog merge 8b8e037 verified; no VIVY integration yet |
| OBS-02 | Actual model-call Journal lifecycle / O2 | — | [OBS-02](observability/OBS-02.md) | Planned | OBS-D1 schema review pending; pinned Eino/provider seam investigated; no code changed |
| OBS-03 | Usage projection + VIVY token UI / O3,O8 | OBS-02: schemas and real event fixtures | [OBS-03](observability/OBS-03.md) | Blocked | Producer identity/settlement not implemented |
| OBS-04 | Trajectory projection + VIVY live UI / O4,O8 | OBS-02: stable call/event fixtures | [OBS-04](observability/OBS-04.md) | Blocked | Producer identity/settlement not implemented |
| OBS-05 | Diagnostic RPC + GUI writer / O5 | OBS-01: owned log families/redactor | [OBS-05](observability/OBS-05.md) | Blocked | Producer service/capabilities not implemented |
| OBS-06 | DIVA token/connection console / O3,O6,O7 | OBS-03: stats fixtures; DN-2: accepted client/session | [OBS-06](observability/OBS-06.md) | Blocked | Real shared transport and producer acceptance missing |
| OBS-07 | DIVA trajectory/child view / O4 | OBS-04: trajectory fixtures; DN-2: accepted discovery/events | [OBS-07](observability/OBS-07.md) | Blocked | Real shared transport and producer acceptance missing |
| OBS-08 | DIVA diagnostics/GUI logging / O5,O7 | OBS-05: diagnostic fixtures; DN-2: accepted client/lifetime | [OBS-08](observability/OBS-08.md) | Blocked | Proposed diagnostic producer and real transport missing |
| OBS-09 | Packaged observability acceptance / O7,O8 | OBS-06,OBS-07,OBS-08: accepted consumers; DN-P: native core gate | [OBS-09](observability/OBS-09.md) | Blocked | No packaged cross-repository acceptance yet |

DN-3's observability domain is accepted only with OBS-09 evidence. Other DN-3 slices are independently gated. This domain completion gate is not an execution edge from DN-3 to its child Stories. Root Stories may start after specification review/implementation authorization; no mock predecessor releases a blocked consumer.

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

- 2026-10-02 OBS-D1: user requested the follow-up observability plan after merging prettylog. Added nine executable Stories, domain disposition and explicit native dependencies. Installed official Go 1.26.4 and inspected Eino v0.9.13; this supersedes the environment-only “Go unavailable” observation, without releasing DN-L. No issue/comment or product code modified.

## Next action

Review OBS-D1 and feed its concrete observability schema/ledger requirements into DN-0. Execute DN-0's bounded inventory/contract freeze. Do not release DN-L until ABI/Generation/native-host prerequisites have evidence. Current artifacts are concrete reviewable plans, not a claim that the two P0s are implemented.
