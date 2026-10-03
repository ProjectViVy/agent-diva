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
PR is not final acceptance. DN-C2 is detailed architecture and proposed
contract design only, not product implementation or an executable plan package. The prior DN-C1 iteration also
reconciled tracker wording; this revision makes no external issue mutations.
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
| [DN-0](DN-0.md) | Inventory and contract freeze | — | Initial ledger delivered; closure-specific wire/action fixtures still need refresh |
| [DN-L](DN-L.md) | Sealed shared library | DN-0 frozen contracts | Initial implementation merged in VIVY #28; new cognitive/OBS composition and repack pending; Windows evidence pending |
| [DN-5](DN-5.md) | Tauri/FFI lifetime | DN-L artifact/header | Initial implementation merged in DIVA #16; Linux evidence recorded; online native services are a narrow DN-6 extension |
| [DN-1](DN-1.md) | Typed client and state projection | DN-5 transport | Implemented on initial pin; extend only for newly frozen contracts |
| [DN-2](DN-2.md) | Chat/approval/cancel | DN-1 client | Initial real-model chain recorded; closure chat attachments/permission/rewind/goal UI and restart semantics pending |
| [DN-P](DN-P.md) | Core packaged chain | DN-2 core, DN-L artifact | Prior Linux chain 21/21 recorded; owner final acceptance pending; new artifact invalidates old coverage |
| [DN-3](DN-3.md) | Operational settings | DN-2 mutation/recovery | Initial slices delivered; scoped residuals in TODOLIST, not universal feature-parity acceptance |
| [DN-4](DN-4.md) / Laputa S08 | Bound cognition and companion UI | DN-1/2; Laputa S07 + VIVY #26 domain/runtime contracts | Masks delivered; cognition needs same-owner public facade extension, generated binding, per-run authority/recovery guards, embedded start/stop and model-input projection; bridge-only blocker is stale |
| OBS-06..09 | Console consumers and packaged OBS acceptance | VIVY #27 contracts; DN-1/2; DN-P for package gate | Backend producers merged; DIVA still token-only; exact contract freeze and consumer work pending; preserve the existing OBS track |
| [DN-6](DN-6.md) | DIVA-native online speech/media | DN-2 run state, DN-5 host; DN-C2 native speech contract/probes | Architecture/command design supplied in DN-C2; Raw WAV/MP3, OS credential/provider/native probes and implementation pending |
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
architecture/contract design supersedes DN-C1. Existing stages
retain ownership; no second Epic/Story package or implementation is created.

Prior evidence remains under `docs/logs/2026-10-diva-next-dn2/`, `-dn3/`,
`-dn4/`, `-dnp/`, `-dnm/`. Current documentation-only evidence is under
`docs/logs/2026-10-diva-next-closure/v0.1.0-scope-and-architecture/` and
`v0.2.0-detailed-architecture/`.
