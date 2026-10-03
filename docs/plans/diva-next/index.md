# DIVA Next — authoritative delivery index

Updated 2026-10-03 for the owner's closure directive. The current design is
[p0-design.md](p0-design.md), revision **DN-C2**. This index alone owns stage
status and dependencies; TODOLIST owns residual bugs/deferred work, and
[the ledger](backend-separation-contracts.md) owns verified wire mappings.
Issues #13/#15 and VIVY #18 are entry points, not competing design authorities.
The 2026-09 DN-P1 comment and P0-D1 are historical where superseded by DN-C2.

## Scope and acceptance

In scope: cognition, console observability, chat wiring, online SiliconFlow
STT and SiliconFlow/MiniMax TTS through DIVA/Tauri, native package preparation.
The owner performs final installed-product acceptance after engineering
prepares code/checks/artifacts and reproducible steps. A green build or merged
PR is not final acceptance. DN-C2 remains the detailed architecture and proposed-contract authority.
Its executable decomposition is now included below: **20 Story plans**, no
product implementation or new acceptance. Prior DN-C1 reconciled tracker
wording; this planning iteration makes no issue/PR/merge/release mutation.
Local ONNX/OLVRS, broad report/resource systems and pet expansion
are deferred. **Historical data import is cancelled; DN-7 is not a blocker.**

Source baseline: DIVA `f5866a0561e2676a9b8afc2405395dc72d087dfb`, VIVY
`1db8b55ce905ee4a212326e7801e0958e7f4376a`, Laputa
`dc6066e2bb983ebd6b31e53dec908a9a23366956`. Current bundled Generation remains
`1fd14fb2…`; newly merged backend code is not automatically in that artifact.
New source/recipe/library pins require fresh package acceptance evidence.

## Stage state

| Stage | Outcome | Immediate prerequisites | State / evidence |
| --- | --- | --- | --- |
| [DN-W](DN-W.md) | Old Rust business backend retired | — | Historical implementation delivered in #14; never restore as fallback |
| [DN-0](DN-0.md) | Inventory and contract freeze | — | Initial ledger delivered; DN-0C/S/P plans now supplied, capture/probes pending |
| [DN-L](DN-L.md) | Sealed shared library | DN-0 frozen contracts | Initial implementation merged in VIVY #28; new cognitive/OBS composition and repack pending; Windows evidence pending |
| [DN-5](DN-5.md) | Tauri/FFI lifetime | DN-L artifact/header | Initial implementation merged in DIVA #16; Linux evidence recorded; online native services are a narrow DN-6 extension |
| [DN-1](DN-1.md) | Typed client and state projection | DN-5 transport | Implemented on initial pin; extend only for newly frozen contracts |
| [DN-2](DN-2.md) | Chat/approval/cancel | DN-1 client | Initial real-model chain recorded; closure chat attachments/permission/rewind/goal UI and restart semantics pending |
| [DN-P](DN-P.md) | Core packaged chain | DN-2 core, DN-L artifact | Prior Linux chain 21/21 recorded; owner final acceptance pending; new artifact invalidates old coverage |
| [DN-3](DN-3.md) | Operational settings | DN-2 mutation/recovery | Initial slices delivered; scoped residuals in TODOLIST, not universal feature-parity acceptance |
| [DN-4](DN-4.md) / Laputa S08 | Bound cognition and companion UI | DN-1/2; Laputa S07 + VIVY #26 domain/runtime contracts | Masks delivered; cognition needs same-owner public facade extension, generated binding, per-run authority/recovery guards, embedded start/stop and model-input projection; bridge-only blocker is stale |
| OBS-06..09 | Console consumers and packaged OBS acceptance | VIVY #27 contracts; DN-1/2; DN-P for package gate | Backend producers merged; DIVA still token-only; exact contract freeze and consumer work pending; preserve the existing OBS track |
| [DN-6](DN-6.md) | DIVA-native online speech/media | DN-2 run state, DN-5 host; DN-C2 native speech contract/probes | DN-C2 + DN-6A/B/C plans supplied; DN-0S probes and native/provider implementation pending |
| [DN-M](DN-M.md) | Scoped semantic closure | DN-3 scoped dispositions, DN-4, DN-6, DN-2 additions, OBS consumers, DN-P | Static gate delivered; behavior and newly allowed native speech boundaries still need evidence |
| [DN-7](DN-7.md) | Historical import | — | **Cancelled by owner 2026-10-03**; no implementation and no release dependency |
| [DN-8](DN-8.md) | Clean package and final owner acceptance | DN-M; OBS package evidence; DN-P refreshed artifact | Pending; no automatic release; excludes DN-7 |

Stage links retain historical task/evidence records. DN-C2 and amended scope
sections take precedence over their old parity/import/host premises. No new
stage is Ready merely because a detailed architecture or source API exists.

## Requirements after explicit scope revision

- R-1: selected required behavior has real evidence or an explicit scope
  disposition. Deferred residuals are visible in TODOLIST, never called done.
- R-2: one VIVY Agent authority; no Rust Manager, executor or competing Agent
  database. Narrow native speech preferences/secrets/assets are permitted.
- R-3: real model/tools/approval/recovery and corrected chat controls.
- R-4: real durable scoped cognition, Persona/Mission projection and review.
- R-5: online speech/native avatar obey cancel, generation and replay semantics.
- R-6: native lifecycle/package preparation; owner final acceptance on Windows
  x64. Existing Linux evidence has only its recorded scope.
- R-7: cancelled historical import obligation; fresh Next home without old-home
  reads, copying or deletion. New media import remains a separate device action.
- R-8: reproducible independent package/CI, coherent scoped boundary gates.
- R-9: useful logs, trajectory and honest usage coverage in DIVA console.

## Next closure sequence

1. Freeze missing domain/public facade and native media contracts; refresh the
   existing ledger. Pin source APIs/fixtures; record exact engineering blockers.
2. Integrate cognition/embedded lifecycle, chat, console and online speech per
   accepted detailed plans. Shared-file edits are serialized, not delegated
   automatically. Console/chat can proceed on real existing contracts without
   waiting for unrelated report/resource backends.
3. Repack the selected Generation through the existing compiler; verify ABI,
   native command boundary, dependency graph, configuration and bundle hashes.
4. Reconcile DN-M from actual behavior and prepare DN-8 clean installation and
   acceptance steps. Deliver all available checks before asking for final
   owner acceptance; mark unavailable native checks pending.

Shared files: App.vue (chat then cognition then voice), typed contracts/client,
platform/desktop-host.ts, package/locks, shell lib/lifecycle, VIVY recipe and
app composition, CI/gates. Changes to compiled modules invalidate artifact
readiness. Do not widen dormant pet exemptions to admit new voice code.

## Decisions and evidence

2026-10-03 owner directive: cognition/console/chat/native closure approved;
online DIVA/Tauri speech added; historical import cancelled because there are
no current users to migrate; owner accepts the final product. Detailed DN-C2
architecture/contract design supersedes DN-C1. Existing stages retain ownership.
The 2026-10-03 execution decomposition adds child Story plans under those same
stages; it creates neither a competing index nor product implementation.

Prior evidence remains under `docs/logs/2026-10-diva-next-dn2/`, `-dn3/`,
`-dn4/`, `-dnp/`, `-dnm/`. Current documentation-only evidence is under
`docs/logs/2026-10-diva-next-closure/v0.1.0-scope-and-architecture/` and
`v0.2.0-detailed-architecture/`.


## Executable Story package

Planning revision **DN-C2-P1**, 2026-10-03. This section is the only live
Story status/dependency authority. Each linked file is an executable Plan,
including blocked downstream work; a backlog row alone is not a plan.
No Story is In progress/Done from document generation. **Ready** means a
concrete independent implementation/probe plan can be released; no code work
has started in this planning turn. **Blocked** identifies predecessor or
native evidence still needed, not a missing user product decision.

Epics reuse the existing stages: DN-0 (proof inputs), DN-4/DN-L (bound
cognition), DN-2 (real chat), OBS-C/D (console), DN-6 (online voice),
DN-P/DN-M/DN-8 (candidate/boundaries/owner handoff). DN-3 operational settings
are changed only alongside their owning cognition/chat/voice/console Story;
residual disposition is audited by DN-M-C. DN-7 remains cancelled.

| Story / Plan | Epic / requirements | Outcome | Immediate predecessor and exact output | State | Evidence / blocker |
| --- | --- | --- | --- | --- | --- |
| [DN-0C](closure/DN-0C.md) | DN-0 / R-2, R-3, R-9 | Capture current chat and console contracts | — | Ready | Concrete independent plan; execution not started |
| [DN-0S](closure/DN-0S.md) | DN-0 / R-5, R-6, R-8 | Probe native speech seams and provider mappings | — | Ready | Concrete independent plan; execution not started |
| [DN-0P](closure/DN-0P.md) | DN-0 / R-2, R-6, R-8 | Inventory pinned build and native dependency closure | — | Ready | Concrete independent plan; execution not started |
| [DN-4A](closure/DN-4A.md) | DN-4 / R-2, R-4 | Expose one owned Garden domain and selected backend | — | Ready | Concrete independent plan; execution not started |
| [DN-LC](closure/DN-LC.md) | DN-L / R-2, R-4, R-8 | Generate and compose the selected cognitive factory | DN-4A: same-owner public facade/conformance | Blocked | Predecessor output/evidence pending |
| [DN-4B](closure/DN-4B.md) | DN-4 / R-2, R-3, R-4, R-6 | Bind primary authority and safe cognitive runtime lifecycle | DN-LC: sealed factory/bundle/provider inventory | Blocked | Predecessor output/evidence pending |
| [DN-4C](closure/DN-4C.md) | DN-4 / R-2, R-4 | Implement governed human cognitive actions | DN-4B: actual primary context, runtime guards/lifecycle | Blocked | Predecessor output/evidence pending |
| [DN-4D](closure/DN-4D.md) | DN-4 / R-3, R-4 | Connect cognitive setup and scoped companion views | DN-4C: captured authenticated human action contracts | Blocked | Predecessor output/evidence pending |
| [DN-2A](closure/DN-2A.md) | DN-2 / R-2, R-3 | Wire image send and admitted permission controls | DN-0C: captured chat/OBS source fixtures | Blocked | Predecessor output/evidence pending |
| [DN-2B](closure/DN-2B.md) | DN-2 / R-3, R-4, R-5 | Fix selected-turn regeneration and recovery controls | DN-2A: image/policy payload and mutation lane | Blocked | Predecessor output/evidence pending |
| [OBS-06](observability/OBS-06.md) | OBS-C / R-2, R-9 | Truthful token dashboard and host connection | DN-0C: captured chat/OBS source fixtures | Blocked | Predecessor output/evidence pending |
| [OBS-07](observability/OBS-07.md) | OBS-C / R-2, R-3, R-9 | Single-owner trajectory and child references | DN-0C: captured chat/OBS source fixtures | Blocked | Predecessor output/evidence pending |
| [OBS-08](observability/OBS-08.md) | OBS-C / R-2, R-9 | Bounded diagnostics and honest GUI persistence | DN-0C: captured chat/OBS source fixtures | Blocked | Predecessor output/evidence pending |
| [DN-6A](closure/DN-6A.md) | DN-6 / R-2, R-5, R-8 | Native speech preferences, credentials and reference assets | DN-0S: accepted IPC/keyring/provider compatibility | Blocked | Predecessor output/evidence pending |
| [DN-6B](closure/DN-6B.md) | DN-6 / R-2, R-5, R-6, R-9 | Bounded cloud providers and native request cancellation | DN-6A: native config/key/asset contracts | Blocked | Predecessor output/evidence pending |
| [DN-6C](closure/DN-6C.md) | DN-6 / R-3, R-5, R-9 | Connect main-chat voice and generation-safe playback | DN-6B: provider/cancellation/IPC contracts; DN-2B: selected-turn/recovery/Goal/invalidation; OBS-08: diagnostic/recorder API and evidence | Blocked | Predecessor output/evidence pending |
| [DN-P-C](closure/DN-P-C.md) | DN-P / R-2, R-6, R-8 | Repack and inspect the integrated closure Generation | DN-4D: accepted scoped cognitive UI code/evidence; OBS-06: token/connection consumer evidence; OBS-07: single-owner trajectory/child consumer evidence; DN-6C: generation-safe main-chat voice code/evidence; DN-0P: pinned clean staging/native inventory | Blocked | Predecessor output/evidence pending |
| [OBS-09](observability/OBS-09.md) | OBS-D / R-6, R-8, R-9 | Verify observability in the new native candidate | DN-P-C: immutable candidate/hash/build matrix; OBS-06: token/connection consumer evidence; OBS-07: single-owner trajectory/child consumer evidence; OBS-08: diagnostic/recorder API and evidence | Evidence landed | [closure-packaged-obs.json](fixtures/closure-packaged-obs.json): R-9 matrix 12 passed / 1 failed / 3 pending on candidate `d0155e26`; findings OBS09-F1..F5 (runtime log sink absent, FrozenCore capture seam unwired, per-process action grants, shutdown cancels suspended runs); owner acceptance pending |
| [DN-M-C](closure/DN-M-C.md) | DN-M / R-1, R-2, R-3, R-4, R-5, R-7, R-8, R-9 | Audit scoped behavior and boundary closure | OBS-09: packaged OBS evidence on that candidate | Evidence landed | C2-8 matrix: R-1/2/7/8 pass; R-3/4/5 pass with residuals/findings; R-6 pending; R-9 pass with 1 failure + 3 pending; cargo/pnpm audits recorded; 10 rows passed to DN-8C |
| [DN-8C](closure/DN-8C.md) | DN-8 / R-1, R-3, R-4, R-5, R-6, R-7, R-8, R-9 | Prepare clean native installation and owner acceptance | DN-M-C: scoped requirement/boundary evidence | Handoff ready | [owner-acceptance.md](closure/owner-acceptance.md) written with candidate pins, 26-row checklist, gap list F1–F5, diagnostic export procedure; `windows-shell-check` CI job added for the real Windows compile; candidate version `0.4.13`; owner acceptance pending |

OBS-06/07/08 retain their original **token/connection**, **trajectory** and
**diagnostics** meanings. Their source producer prerequisites OBS-01…05 are
already merged at VIVY `1db8b55`; DN-0C captures the current actual structs and
replaces old pending-producer assumptions. The old DIVA plan branch
[`364163e`](https://github.com/ProjectViVy/agent-diva/tree/364163e97c972cec9aef007f1be72005af60f25a/docs/plans/diva-next/observability)
is provenance, not a competing execution map.

OBS-09 deliberately retains direct edges to OBS-06/07/08 although DN-P-C
also consumes their code (OBS-08 transitively via DN-6C): consumer evidence
matrices are a separate acceptance input from the rebuilt candidate's identity.
Other duplicated transitive edges are omitted. Logical dependency and
shared-file scheduling are separate. Later-wave plans become Ready only
after accepted exact predecessor outputs; a source merge or this wave table
does not meet that gate.

### Topological execution waves

1. DN-0C, DN-0S, DN-0P, DN-4A.
2. DN-LC, DN-2A, OBS-06, OBS-07, OBS-08, DN-6A.
3. DN-4B, DN-2B, DN-6B.
4. DN-4C, DN-6C.
5. DN-4D.
6. DN-P-C.
7. OBS-09.
8. DN-M-C.
9. DN-8C.

These are dependency waves, not duration/staffing/calendar promises. Root
DN-4A can implement library conformance while DN-0C/S/P probe independent
boundaries. Native access failures block the affected output; unaffected
plans continue. Provider keys/owner audio quality are not needed to write
offline contracts or source tests.

### Requirement coverage

| Requirement | Implementing / evidence Stories |
| --- | --- |
| R-1 | DN-M-C, DN-8C |
| R-2 | DN-0C, DN-0P, DN-4A, DN-LC, DN-4B, DN-4C, DN-2A, OBS-06, OBS-07, OBS-08, DN-6A, DN-6B, DN-P-C, DN-M-C |
| R-3 | DN-0C, DN-4B, DN-4D, DN-2A, DN-2B, OBS-07, DN-6C, DN-M-C, DN-8C |
| R-4 | DN-4A, DN-LC, DN-4B, DN-4C, DN-4D, DN-2B, DN-M-C, DN-8C |
| R-5 | DN-0S, DN-2B, DN-6A, DN-6B, DN-6C, DN-M-C, DN-8C |
| R-6 | DN-0S, DN-0P, DN-4B, DN-6B, DN-P-C, OBS-09, DN-8C |
| R-7 | DN-M-C, DN-8C |
| R-8 | DN-0S, DN-0P, DN-LC, DN-6A, DN-P-C, OBS-09, DN-M-C, DN-8C |
| R-9 | DN-0C, OBS-06, OBS-07, OBS-08, DN-6B, DN-6C, OBS-09, DN-M-C, DN-8C |

R-1 applies additionally to every Story's evidence/disposition. R-2's authority
and exact boundary constraints apply to all code even where a local table row
lists only its direct user requirement. Final owner acceptance is distinct
from every engineering row.

## Execution contract

Read root/local repository instructions, DN-C2, C2-1…6 ledger, this index and
the selected Plan. Baseline code pins remain DIVA `f5866a0`, VIVY `1db8b55`,
Laputa `dc6066e`; published architecture parent is `0e738dd`. A newer source
pin requires readback and invalidates affected fixtures/package acceptance.
New paths in plans are proposed, never evidence of existing code.

**Plan/source corrections:** only PersonaMarkdownEditor currently exists;
PersonaSetupGate/PersonaMemoryView/MemoryView/EvolutionView are new scoped
views in DN-4D. Old AuditPage/guiLogger/SubAgentPanel assumptions are not
used as existing paths. Existing voice recorder/player algorithms may be
moved into active features/voice; their old provider fetch/pet_* wrappers
are not an allowed product transport.

| Shared surface | Integration order / ownership |
| --- | --- |
| Chat controller, contracts/client, App.vue | DN-2A → DN-2B; merge OBS read-only projection changes serially; then DN-4D, then DN-6C main composition |
| ConsoleView, observability facade/DTOs | OBS-06 → OBS-07 → OBS-08 shared edits; domain code/tests may be prepared separately |
| VIVY app/recipe/generated inventory | DN-LC → DN-4B → DN-4C; DN-P-C alone freezes/repackages final pins |
| Source/context/capture/runtime state | DN-4B owns runtime changes; DN-4C consumes controls without a second scheduler |
| Native lib/lifecycle, desktop-host, Cargo locks/gates | DN-6A → DN-6B → DN-6C; native/AST command gates accompany each activation |
| Ledger/index/TODO and evidence | One supervisor integrates accepted outputs; preserve historical evidence and scope |

No delegation is requested by this plan package. Use native task-by-task
execution by default; if delegation is later authorized, resolve shared
contracts/file ownership first and keep integration/review capacity bounded.

Verification uses actual present commands: DIVA `just gui-test`, `gui-build`,
`shell-bridge-test`, `shell-test`, `shell-clippy`, `tauri-build`, `ci` and the
two Node/Python boundary gates. Root retired Rust `cargo test --all`/fmt/check
recipes are absent; this planning turn changes Markdown only, so it runs
document/DAG/path checks and no product tests. Implementation uses the owning
current shell/Go/GUI gates. Root instruction files are not rewritten here;
their stale descriptions remain a separately recorded governance follow-up.

Each implementation task starts with meaningful failing assertions where
behavior changes, then minimal code, focused proof, the necessary owning
integration checks and one scoped English Conventional Commit. Probe/build/
handoff-only tasks collect actual evidence rather than manufacture red tests.
Do not turn mock/source/unit evidence into native or owner acceptance.

Handoff for every Story includes Plan + DN-C2/ledger revision, predecessor
commit/fixture/artifact hashes, permitted files, observed checks/failures,
remaining environment limits and a clean scoped commit. Record results in
existing versioned logs; update this index/TODO. Unknown mutation effects,
authority changes, missing bound capabilities and artifact/hash mismatch
stop affected execution. Reconcile upstream changes and all downstream plans
before resuming; a local technical correction within scope needs no new
product permission round.

Scope/interface/acceptance/major-cost changes require a concrete user decision.
No automatic issue messages, PR, merge or release. The existing explicit
upload authorization covers publishing this documentation update to the same
design branch; it is not product release authorization.
