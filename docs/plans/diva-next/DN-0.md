# DN-0 — Inventory and contract freeze

- **Epic:** A (migration foundation) · **Requirements:** R-1 · **Outcome:** every runtime-facing call and persisted domain has a recorded disposition; the VIVY integration revision is pinned.
- **Authoritative design:** issue #13 DN-P1 §1–§6 · **Baseline:** `0fd005a105d8987df02ae7b796a3c591e04b9ca3` · **Status:** Ready · **Index:** [index.md](index.md)
- **In scope:** read-only inventory of the old chain; classification; contract record. **Out of scope:** any code change, deletion, or implementation. **Escalate:** a required behavior with no identifiable VIVY contract and no #63 track.

## Contracts

Produces `docs/plans/diva-next/backend-separation-contracts.md` (proposed path from issue §6): the inventory table + disposition per call/domain + pinned VIVY artifact/revision + named dependency for each unresolved contract. DN-1..DN-8 consume this file.

## Tasks

- [ ] Enumerate registered Tauri commands in `agent-diva-gui/src-tauri/src/{commands,lib,app_state,gateway_status,shutdown_manager,notebook,tray,process_utils,embedded_server}.rs` **at baseline `0fd005a1` (git history — the tree is deleted by DN-W)**; map each to frontend `invoke` consumers in `agent-diva-gui/src/` (scan baseline: ~158 invoke sites across 18 files, 26 listen sites across 3 files — verify actual list, do not trust the counts).
- [ ] Enumerate Manager HTTP/SSE routes the frontend reaches, direct provider calls, native-only calls, stored data, and configuration secrets.
- [ ] Classify each item: reuse presentation / replace by existing VIVY contract / blocked on #63 / native host / offline import / explicitly retired. Record consumer, target method/action, capability, input/output/error shape, backend revision, acceptance fixture.
- [ ] Include all currently required channels/providers/native interactions; omissions get an explicit product disposition, not silent deletion.
- [ ] Select one VIVY integration artifact/revision; reconcile memory, continuity, Plan/Goal, channel and orchestration branches against #63. Record the chosen revision in the contract doc. Do not make unrelated future backend features a global blocker.
- [ ] Freeze initial desktop acceptance platform + DN-5 host-probe checklist (Windows is the planning candidate).

## Verification

- Contract doc exists; every runtime-facing call and persisted domain has a disposition; unresolved contracts carry a named VIVY dependency.
- Reviewer can trace each classification back to a source anchor.

## Evidence to supervisor

Contract doc path, pinned VIVY revision, list of unresolved contracts with their #63 links.

## Notes

Inventory alone claims no build or implementation completion.
